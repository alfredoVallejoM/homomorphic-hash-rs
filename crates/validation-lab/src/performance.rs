use std::{
    hint::black_box,
    process::Command,
    time::{Duration, Instant},
};

use homomorphic_hash_rs::{
    CanonicalElementEncoder, FastGraphLabeler, GraphExecution, GraphWorkspace,
    IncidenceGraphBuilder, PrimeIntegerEncoder, RefinementProfile, SequenceSignature,
};
use microfield::Fp251V1;
use sha2::{Digest, Sha256};

use crate::model::{EnvironmentReport, PerformanceReport, PerformanceSample, ValidationManifest};

const DOMAIN: u64 = 0x4636_5045_5246_0001;

pub fn run_campaign(manifest: &ValidationManifest) -> Result<PerformanceReport, String> {
    let mut samples = Vec::new();
    for &size in &[1_024_usize, 65_536] {
        let values: Vec<u8> = (0..size).map(|index| (index % 251) as u8).collect();
        samples.push(measure("sequence-f251-recompute", size, manifest, || {
            let mut signature = SequenceSignature::<Fp251V1, _>::new(
                CanonicalElementEncoder,
                Fp251V1::from_u64_mod(11),
            )
            .expect("valid base");
            for value in &values {
                signature.push(&[*value]).expect("canonical byte");
            }
            signature.state().to_canonical_byte()
        }));
        let midpoint = values.len() / 2;
        let left = build_sequence(&values[..midpoint]);
        let right = build_sequence(&values[midpoint..]);
        samples.push(measure("sequence-f251-merge", size, manifest, || {
            left.concatenate(&right)
                .expect("same context")
                .state()
                .to_canonical_byte()
        }));
        samples.push(measure("sha256-bytes", size, manifest, || {
            Sha256::digest(black_box(&values))[0]
        }));
    }

    for &vertices in &manifest.performance.sparse_graph_vertices {
        let graph = sparse_cycle(vertices)?;
        let labeler = FastGraphLabeler::<Fp251V1, _, 3>::new(
            PrimeIntegerEncoder::new(DOMAIN),
            RefinementProfile::Fast {
                rounds: manifest.graph.rounds,
            },
        )
        .map_err(debug_error)?;
        let prepared = labeler.prepare(&graph).map_err(debug_error)?;
        let mut workspace = GraphWorkspace::new();
        workspace.reserve_for(vertices, manifest.graph.rounds);
        samples.push(measure_mut(
            "graph-f251-prepared-sequential",
            vertices,
            manifest,
            || {
                labeler
                    .analyze_prepared_with_workspace(
                        black_box(&prepared),
                        &mut workspace,
                        GraphExecution::Sequential,
                    )
                    .expect("prepared graph")
                    .signature()
                    .lanes()[0]
                    .to_canonical_byte()
            },
        ));
    }

    Ok(PerformanceReport {
        schema_version: 1,
        campaign_id: manifest.campaign_id.clone(),
        environment: environment(),
        samples,
    })
}

fn build_sequence(values: &[u8]) -> SequenceSignature<Fp251V1, CanonicalElementEncoder> {
    let mut signature = SequenceSignature::new(CanonicalElementEncoder, Fp251V1::from_u64_mod(11))
        .expect("valid base");
    for value in values {
        signature.push(&[*value]).expect("canonical byte");
    }
    signature
}

fn sparse_cycle(vertices: usize) -> Result<homomorphic_hash_rs::IncidenceGraph, String> {
    let mut builder = IncidenceGraphBuilder::new();
    let ids: Vec<_> = (0..vertices)
        .map(|_| builder.add_vertex(Vec::new()))
        .collect();
    if vertices > 1 {
        for index in 0..vertices {
            builder
                .add_undirected_relation(
                    ids[index],
                    ids[(index + 1) % vertices],
                    b"edge".to_vec(),
                    Vec::new(),
                    1,
                )
                .map_err(debug_error)?;
        }
    }
    builder.build().map_err(debug_error)
}

fn measure<T: Into<u64>>(
    operation: &str,
    input_size: usize,
    manifest: &ValidationManifest,
    action: impl Fn() -> T,
) -> PerformanceSample {
    for _ in 0..manifest.performance.warmup_iterations {
        black_box(action());
    }
    let mut checksum = 0_u64;
    let mut durations = Vec::with_capacity(manifest.performance.measured_iterations);
    for _ in 0..manifest.performance.measured_iterations {
        let started = Instant::now();
        checksum ^= black_box(action()).into();
        durations.push(started.elapsed());
    }
    sample(operation, input_size, durations, checksum)
}

fn measure_mut<T: Into<u64>>(
    operation: &str,
    input_size: usize,
    manifest: &ValidationManifest,
    mut action: impl FnMut() -> T,
) -> PerformanceSample {
    for _ in 0..manifest.performance.warmup_iterations {
        black_box(action());
    }
    let mut checksum = 0_u64;
    let mut durations = Vec::with_capacity(manifest.performance.measured_iterations);
    for _ in 0..manifest.performance.measured_iterations {
        let started = Instant::now();
        checksum ^= black_box(action()).into();
        durations.push(started.elapsed());
    }
    sample(operation, input_size, durations, checksum)
}

fn sample(
    operation: &str,
    input_size: usize,
    mut durations: Vec<Duration>,
    checksum: u64,
) -> PerformanceSample {
    durations.sort_unstable();
    let median = durations[durations.len() / 2].as_nanos();
    let p95_index = ((durations.len() - 1) * 95).div_ceil(100);
    PerformanceSample {
        operation: operation.into(),
        input_size,
        iterations: durations.len(),
        median_ns: median,
        p95_ns: durations[p95_index].as_nanos(),
        checksum: format!("{checksum:016x}"),
    }
}

fn environment() -> EnvironmentReport {
    let rustc = Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .unwrap_or_else(|| "unavailable".into());
    let mut detected_features = Vec::new();
    #[cfg(target_arch = "x86_64")]
    for feature in ["avx2", "bmi2", "pclmulqdq"] {
        let detected = match feature {
            "avx2" => std::is_x86_feature_detected!("avx2"),
            "bmi2" => std::is_x86_feature_detected!("bmi2"),
            "pclmulqdq" => std::is_x86_feature_detected!("pclmulqdq"),
            _ => false,
        };
        if detected {
            detected_features.push(feature.into());
        }
    }
    #[cfg(target_arch = "aarch64")]
    for feature in ["neon", "aes"] {
        let detected = match feature {
            "neon" => std::arch::is_aarch64_feature_detected!("neon"),
            "aes" => std::arch::is_aarch64_feature_detected!("aes"),
            _ => false,
        };
        if detected {
            detected_features.push(feature.into());
        }
    }
    EnvironmentReport {
        architecture: std::env::consts::ARCH.into(),
        operating_system: std::env::consts::OS.into(),
        rustc,
        logical_threads: std::thread::available_parallelism()
            .map_or(1, std::num::NonZeroUsize::get),
        detected_features,
    }
}

trait CanonicalByte {
    fn to_canonical_byte(self) -> u8;
}

impl CanonicalByte for Fp251V1 {
    fn to_canonical_byte(self) -> u8 {
        use microfield::CanonicalEncoding;
        self.to_canonical()[0]
    }
}

fn debug_error(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}
