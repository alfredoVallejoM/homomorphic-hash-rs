use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput, BatchSize};
use rs_merkle::{MerkleTree, algorithms::Sha256}; // Librería de Grado de Producción
use std::time::{Instant, Duration};
use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::Path;

// IMPORTACIONES DE LA LIBRERÍA
use homomorphic_hash_rs::algebra::traits::FiniteField;
use homomorphic_hash_rs::algebra::galois_256::GaloisSignature256;
use homomorphic_hash_rs::topology::traits::HomomorphicAggregator;
use homomorphic_hash_rs::topology::multiset::MultisetAggregator;
use homomorphic_hash_rs::engine::proofs::{ProofGenerator, ProofVerifier};

// =============================================================================
// MOTOR DE TELEMETRÍA RAW CON RIGOR ESTADÍSTICO (VERSIÓN Q1: MEDIANA + P99)
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

    /// Registra la Mediana y el Percentil 99 de un conjunto de mediciones para
    /// demostrar determinismo absoluto y aislar el ruido térmico del SO.
    pub fn record_distribution(&mut self, n: usize, structure: &str, phase: &str, mut times_ns: Vec<u128>) {
        if times_ns.is_empty() { return; }

        // Ordenación in-place para estadística no paramétrica
        times_ns.sort_unstable();

        // Mediana (Rendimiento Típico)
        let median = times_ns[times_ns.len() / 2];

        // Percentil 99 (Límite de Latencia Superior / Tail Latency)
        let p99_idx = (times_ns.len() as f64 * 0.99).ceil() as usize;
        let p99 = times_ns[p99_idx.min(times_ns.len() - 1)];

        writeln!(self.file, "{},{},{},{},{}", n, structure, phase, median, p99).unwrap();
    }
}

// =============================================================================
// GENERADOR DE MASA FÍSICA (ENTROPÍA)
// =============================================================================
fn generate_entropy(size: usize) -> Vec<[u8; 32]> {
    (0..size).map(|i| {
        let mut buf = [0u8; 32];
        let bytes = (i as u64).to_le_bytes();
        buf[0..8].copy_from_slice(&bytes);
        buf
    }).collect()
}

// =============================================================================
// METROLOGÍA: EXPERIMENTO A (O(1) vs O(log N))
// =============================================================================
pub fn bench_asymptotics(c: &mut Criterion) {
    let mut group = c.benchmark_group("Exp_A_Merkle_Asymptotics");

    // CONFIGURACIÓN TÉRMICA Y ESTADÍSTICA (NIVEL Q1)
    group.warm_up_time(Duration::from_secs(3)); // 3s para calentar cachés y estabilizar reloj de CPU
    group.measurement_time(Duration::from_secs(5));
    group.sample_size(100);

    // Cabecera actualizada para soportar el cálculo de distribución
    let mut csv = CsvTelemetry::new("exp_a_raw_data", "N,Structure,Phase,Time_ns_Median,Time_ns_P99");

    // BARRIDO LOGARÍTMICO HASTA SATURAR LA CACHÉ L3 (10^7 = ~320 MB en RAM)
    let mut sizes = Vec::new();
    let decades = 5; // De 10^2 a 10^7
    let points_per_decade = 3;
    for i in 0..=(decades * points_per_decade) {
        let exponent = 2.0 + (i as f64) / (points_per_decade as f64);
        sizes.push((10.0_f64.powf(exponent)) as usize);
    }
    sizes.dedup();

    for &size in &sizes {
        group.throughput(Throughput::Elements(size as u64));
        let dataset = generate_entropy(size);

        let target_index = size / 2;
        let target_element = dataset[target_index];

        // ---------------------------------------------------------------------
        // PASADA DE TELEMETRÍA SOMBRA (Muestreo Estocástico Masivo para CSV)
        // ---------------------------------------------------------------------
        // Ajustamos dinámicamente las muestras: mucha precisión para N pequeños,
        // límite térmico controlado para N colosales (evitar horas de ejecución).
        let shadow_samples = if size > 1_000_000 { 10 } else if size > 100_000 { 50 } else { 200 };

        let mut t_build_merkle = Vec::with_capacity(shadow_samples);
        let mut t_build_galois = Vec::with_capacity(shadow_samples);
        let mut t_prove_merkle = Vec::with_capacity(shadow_samples);
        let mut t_prove_galois = Vec::with_capacity(shadow_samples);
        let mut t_verify_merkle = Vec::with_capacity(shadow_samples);
        let mut t_verify_galois = Vec::with_capacity(shadow_samples);

        // Variables de anclaje de estado
        let mut final_merkle_root = [0u8; 32];
        let mut final_galois_state = MultisetAggregator::<GaloisSignature256>::empty_state();

        for _ in 0..shadow_samples {
            // 1. Build Merkle (rs_merkle)
            let start = Instant::now();
            let merkle_tree: MerkleTree<Sha256> = MerkleTree::from_leaves(&dataset);
            t_build_merkle.push(start.elapsed().as_nanos());
            final_merkle_root = merkle_tree.root().unwrap();

            // 1. Build Galois (O(N) Homomorphic Aggregation)
            let start = Instant::now();
            let mut galois_state = MultisetAggregator::<GaloisSignature256>::empty_state();
            for data in &dataset {
                galois_state = MultisetAggregator::aggregate(&galois_state, &MultisetAggregator::embed_to_field(data), 0);
            }
            t_build_galois.push(start.elapsed().as_nanos());
            final_galois_state = galois_state;

            // 2. Prove Merkle (O(log N) Path Generation)
            let start = Instant::now();
            let merkle_proof = merkle_tree.proof(&[target_index]);
            t_prove_merkle.push(start.elapsed().as_nanos());

            // 2. Prove Galois (O(1) Fermat Inversion)
            let start = Instant::now();
            let galois_proof = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&galois_state, &target_element).unwrap();
            t_prove_galois.push(start.elapsed().as_nanos());

            // 3. Verify Merkle (O(log N) Path Hashing)
            let start = Instant::now();
            black_box(merkle_proof.verify(final_merkle_root, &[target_index], &[target_element], size));
            t_verify_merkle.push(start.elapsed().as_nanos());

            // 3. Verify Galois (O(1) Isomorphic Re-evaluation)
            let start = Instant::now();
            black_box(ProofVerifier::verify_inclusion::<GaloisSignature256, MultisetAggregator>(&galois_state, &target_element, &galois_proof, 0));
            t_verify_galois.push(start.elapsed().as_nanos());
        }

        // Vaciado de distribuciones al CSV
        csv.record_distribution(size, "Merkle", "Build", t_build_merkle);
        csv.record_distribution(size, "Galois", "Build", t_build_galois);
        csv.record_distribution(size, "Merkle", "Prove", t_prove_merkle);
        csv.record_distribution(size, "Galois", "Prove", t_prove_galois);
        csv.record_distribution(size, "Merkle", "Verify", t_verify_merkle);
        csv.record_distribution(size, "Galois", "Verify", t_verify_galois);

        // ---------------------------------------------------------------------
        // PASADA DE CRITERION (Reporte HTML/JSON Oficial)
        // ---------------------------------------------------------------------
        let static_merkle_tree = MerkleTree::<Sha256>::from_leaves(&dataset);
        let static_merkle_proof = static_merkle_tree.proof(&[target_index]);

        group.bench_with_input(BenchmarkId::new("1_Build_Merkle", size), &size, |b, _| {
            b.iter_batched(
                || dataset.clone(),
                |data| MerkleTree::<Sha256>::from_leaves(&data),
                BatchSize::LargeInput,
            )
        });

        group.bench_with_input(BenchmarkId::new("1_Build_Galois", size), &size, |b, _| {
            b.iter_batched(
                || dataset.clone(),
                |data| {
                    let mut state = MultisetAggregator::<GaloisSignature256>::empty_state();
                    for d in &data {
                        state = MultisetAggregator::aggregate(&state, &MultisetAggregator::embed_to_field(d), 0);
                    }
                    state
                },
                BatchSize::LargeInput,
            )
        });

        group.bench_with_input(BenchmarkId::new("2_Prove_Merkle", size), &size, |b, _| {
            b.iter(|| black_box(static_merkle_tree.proof(&[target_index])))
        });

        group.bench_with_input(BenchmarkId::new("2_Prove_Galois", size), &size, |b, _| {
            b.iter(|| {
                black_box(ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(
                    &final_galois_state,
                    &target_element
                ).unwrap())
            })
        });

        group.bench_with_input(BenchmarkId::new("3_Verify_Merkle", size), &size, |b, _| {
            b.iter(|| {
                black_box(static_merkle_proof.verify(
                    final_merkle_root,
                    &[target_index],
                    &[target_element],
                    size
                ))
            })
        });

        let static_galois_proof = ProofGenerator::generate_inclusion_proof::<GaloisSignature256, MultisetAggregator>(&final_galois_state, &target_element).unwrap();

        group.bench_with_input(BenchmarkId::new("3_Verify_Galois", size), &size, |b, _| {
            b.iter(|| {
                black_box(ProofVerifier::verify_inclusion::<GaloisSignature256, MultisetAggregator>(
                    &final_galois_state,
                    &target_element,
                    &static_galois_proof,
                    0
                ))
            })
        });
    }
    group.finish();
}

criterion_group!(benches, bench_asymptotics);
criterion_main!(benches);
