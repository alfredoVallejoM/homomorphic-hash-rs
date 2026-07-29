use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, BatchSize, Throughput};
use std::time::{Instant, Duration};
use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::Path;

// IMPORTACIONES DE LA LIBRERÍA
use homomorphic_hash_rs::algebra::traits::FiniteField;
use homomorphic_hash_rs::algebra::galois_256::GaloisSignature256;
use homomorphic_hash_rs::topology::traits::HomomorphicAggregator;
use homomorphic_hash_rs::topology::sequence::SequenceAggregator;
use homomorphic_hash_rs::engine::proofs::ProofGenerator;

// =============================================================================
// MOTOR DE TELEMETRÍA RAW CON RIGOR ESTADÍSTICO (MEDIANA + P99)
// =============================================================================
pub struct CsvTelemetry {
    file: std::fs::File,
}

impl CsvTelemetry {
    pub fn new(experiment_name: &str, headers: &str) -> Self {
        let dir = Path::new("metrology_data");
        if !dir.exists() {
            create_dir_all(dir).unwrap();
        }
        let filepath = dir.join(format!("{}.csv", experiment_name));

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(filepath)
            .unwrap();

        writeln!(file, "{}", headers).unwrap();
        Self { file }
    }

    /// Registra la Mediana y el Percentil 99 para evaluar el determinismo termodinámico.
    pub fn record_distribution(&mut self, depth: usize, structure: &str, phase: &str, mut times_ns: Vec<u128>) {
        if times_ns.is_empty() { return; }

        times_ns.sort_unstable();
        let median = times_ns[times_ns.len() / 2];

        // Percentil 99 (Tail Latency)
        let p99_idx = (times_ns.len() as f64 * 0.99).ceil() as usize;
        let p99 = times_ns[p99_idx.min(times_ns.len() - 1)];

        writeln!(self.file, "{},{},{},{},{}", depth, structure, phase, median, p99).unwrap();
    }
}

// =============================================================================
// GENERADOR DE MASA FÍSICA CAUSAL (ENTROPÍA)
// =============================================================================
fn generate_entropy(size: usize) -> Vec<[u8; 32]> {
    (0..size).map(|i| {
        let mut buf = [0u8; 32];
        let bytes = (i as u64).to_be_bytes();
        buf[24..32].copy_from_slice(&bytes);
        buf
    }).collect()
}

// =============================================================================
// METROLOGÍA: EXPERIMENTO B (LÍMITES CAUSALES Y ROLLBACK O(1) vs O(N))
// =============================================================================
pub fn bench_causal_rollback(c: &mut Criterion) {
    let mut group = c.benchmark_group("Exp_B_Causal_Rollback");

    // CONFIGURACIÓN TÉRMICA DE GRADO Q1
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(5));
    // Reducimos sample_size en Criterion a 50 porque las operaciones O(N)
    // masivas dominarán el tiempo de reloj, la estadística real está en nuestra pasada sombra.
    group.sample_size(50);

    let mut csv = CsvTelemetry::new("exp_b_raw_data", "Depth,Structure,Phase,Time_ns_Median,Time_ns_P99");

    // BARRIDO LOGARÍTMICO DENSO: Desde 100 hasta 1,000,000 de profundidad causal
    let mut depths = Vec::new();
    let decades = 4; // De 10^2 a 10^6
    let points_per_decade = 3;
    for i in 0..=(decades * points_per_decade) {
        let exponent = 2.0 + (i as f64) / (points_per_decade as f64);
        depths.push((10.0_f64.powf(exponent)) as usize);
    }
    depths.dedup();

    for &depth in &depths {
        group.throughput(Throughput::Elements(depth as u64));

        let dataset = generate_entropy(depth);
        let embedded_dataset: Vec<GaloisSignature256> = dataset.iter()
            .map(|d| SequenceAggregator::embed_to_field(d))
            .collect();

        // ---------------------------------------------------------------------
        // PASADA DE TELEMETRÍA SOMBRA (MUESTREO ESTOCÁSTICO DINÁMICO A CSV)
        // ---------------------------------------------------------------------
        // Dynamic Sampling: Protegemos la RAM y la CPU escalando inversamente las muestras
        let samples_on = if depth > 500_000 { 10 } else if depth > 50_000 { 50 } else { 200 };
        let samples_o1 = 10_000; // Fuerza bruta inamovible para aislar el O(1)

        let mut t_build_forward = Vec::with_capacity(samples_on);
        let mut t_single_rollback = Vec::with_capacity(samples_o1);
        let mut t_full_rollback = Vec::with_capacity(samples_on);

        // Pre-cálculo del estado terminal para las pruebas de LIFO
        let mut terminal_state = SequenceAggregator::<GaloisSignature256>::empty_state();
        for (i, element) in embedded_dataset.iter().enumerate() {
            terminal_state = SequenceAggregator::aggregate(&terminal_state, element, i);
        }
        let last_element = dataset.last().unwrap();

        // 1. Build Forward (O(N) - Construir la línea temporal)
        for _ in 0..samples_on {
            let start = Instant::now();
            let mut s = SequenceAggregator::<GaloisSignature256>::empty_state();
            for (i, element) in embedded_dataset.iter().enumerate() {
                s = SequenceAggregator::aggregate(&s, element, i);
            }
            t_build_forward.push(start.elapsed().as_nanos());
            black_box(s);
        }

        // 2. Single Rollback (O(1) - El límite de Fermat)
        for _ in 0..samples_o1 {
            let start = Instant::now();
            let w = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SequenceAggregator>(
                &terminal_state,
                last_element
            ).unwrap();
            t_single_rollback.push(start.elapsed().as_nanos());
            black_box(w);
        }

        // 3. Full Reverse Rollback (O(N) - Desenrollar todo el universo)
        for _ in 0..samples_on {
            let start = Instant::now();
            let mut s = terminal_state;
            for raw_element in dataset.iter().rev() {
                let w = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SequenceAggregator>(
                    &s,
                    raw_element
                ).unwrap();
                s = w.state_remainder;
            }
            t_full_rollback.push(start.elapsed().as_nanos());
            black_box(s);
        }

        // Volcado al CSV
        csv.record_distribution(depth, "Galois_Sequence", "1_Build_Forward", t_build_forward);
        csv.record_distribution(depth, "Galois_Sequence", "2_Single_O1_Rollback", t_single_rollback);
        csv.record_distribution(depth, "Galois_Sequence", "3_Full_Reverse_Rollback", t_full_rollback);

        // ---------------------------------------------------------------------
        // PASADA DE CRITERION (Reporte HTML/JSON oficial)
        // ---------------------------------------------------------------------

        group.bench_with_input(BenchmarkId::new("1_Build_Forward", depth), &depth, |b, _| {
            b.iter(|| {
                let mut s = SequenceAggregator::<GaloisSignature256>::empty_state();
                for (i, element) in embedded_dataset.iter().enumerate() {
                    s = SequenceAggregator::aggregate(&s, element, i);
                }
                black_box(s)
            })
        });

        group.bench_with_input(BenchmarkId::new("2_Single_O1_Rollback", depth), &depth, |b, _| {
            b.iter(|| {
                black_box(ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SequenceAggregator>(
                    &terminal_state,
                    last_element
                ).unwrap())
            })
        });

        group.bench_with_input(BenchmarkId::new("3_Full_Reverse_Rollback", depth), &depth, |b, _| {
            b.iter_batched(
                || terminal_state,
                |mut state| {
                    for raw_element in dataset.iter().rev() {
                        let w = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, SequenceAggregator>(
                            &state,
                            raw_element
                        ).unwrap();
                        state = w.state_remainder;
                    }
                    state
                },
                BatchSize::LargeInput,
            )
        });
    }
    group.finish();
}

criterion_group!(benches, bench_causal_rollback);
criterion_main!(benches);
