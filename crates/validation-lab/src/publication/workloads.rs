use std::fs;

use homomorphic_hash_rs::{
    AdditiveDelta, AdditiveSignature, ApplicationNamespace, BidirectionalSequenceSignature,
    BinaryPolynomialEncoder, BoundedSetReconciler, CanonicalGraphDag, CanonicalSearchBudget,
    DatabaseColumn, DatabaseColumnType, DatabaseRow, DatabaseSchema, DatabaseValue,
    FastGraphLabeler, FileChunkProfile, GraphExecution, GraphSchemaId, GraphWorkspace,
    HomomorphicSummaryTree, IncidenceGraph, IncidenceGraphBuilder, Microcanon, MicrocanonOutcome,
    MultiEvaluationMultisetSignature, MultiEvaluationSequenceSignature, MultisetSignature,
    PartitionedDatabase, PrimeIntegerEncoder, ReconciliationLimits, RevisionedSignature,
    SequenceSignature,
};
use microfield::{
    generator::BinaryFieldFactory, BinaryPolynomialField, CanonicalEncoding, Engine, Field,
    Fp251V1, FpGoldilocks64V1, Gf2_128V1, Gf2_256AltV1, Gf2_256HhV1,
};

use super::model::BenchmarkCell;

type BinaryEncoder = BinaryPolynomialEncoder;
type Database = PartitionedDatabase<Gf2_128V1, BinaryEncoder>;
type SummaryTree = HomomorphicSummaryTree<Gf2_128V1, BinaryEncoder>;

pub const SUPPORTED_OPERATIONS: &[&str] = &[
    "field.gf2-128.mul",
    "field.gf2-256-hh.mul",
    "field.gf2-256-alt.mul",
    "field.fp251.mul",
    "field.goldilocks.mul",
    "field.gf2-128.batch-detected",
    "field.gf2-128.scalar-total",
    "field.gf2-128.batch-detected-total",
    "signature.additive.build",
    "signature.sequence.build",
    "signature.bidirectional.build",
    "signature.multiset.build",
    "signature.multi-multiset-k2.build",
    "signature.multi-sequence-k2.build",
    "signature.additive.build-total",
    "signature.additive.merge-total",
    "delta.additive.end-to-end",
    "summary-tree.rebuild",
    "summary-tree.local-edit-total",
    "summary-tree.rebuild-edit-total",
    "database.rebuild",
    "database.transaction-end-to-end",
    "database.table-rebuild-total",
    "reconciliation.decode",
    "graph.fast-prepared",
    "graph.exact",
    "graph.dag-reuse",
    "tool.binary-manifest.parse",
    "tool.binary-manifest.generate",
];

pub struct PreparedOperation {
    pub logical_units_per_action: u64,
    pub maximum_batch_iterations: Option<u64>,
    pub run: Box<dyn FnMut() -> u64>,
}

pub fn prepare(cell: &BenchmarkCell, seed: u64) -> Result<PreparedOperation, String> {
    match cell.operation.as_str() {
        "field.gf2-128.mul" => field_gf2_128(seed),
        "field.gf2-256-hh.mul" => field_gf2_256_hh(seed),
        "field.gf2-256-alt.mul" => field_gf2_256_alt(seed),
        "field.fp251.mul" => field_fp251(seed),
        "field.goldilocks.mul" => field_goldilocks(seed),
        "field.gf2-128.batch-detected" => field_batch(cell.scale, seed),
        "field.gf2-128.scalar-total" => field_scalar_total(cell.scale, seed),
        "field.gf2-128.batch-detected-total" => field_batch_total(cell.scale, seed),
        "signature.additive.build" => signature_additive(cell, seed),
        "signature.sequence.build" => signature_sequence(cell, seed),
        "signature.bidirectional.build" => signature_bidirectional(cell, seed),
        "signature.multiset.build" => signature_multiset(cell, seed),
        "signature.multi-multiset-k2.build" => signature_multi_multiset(cell, seed),
        "signature.multi-sequence-k2.build" => signature_multi_sequence(cell, seed),
        "signature.additive.build-total" => signature_additive_total(cell, seed),
        "signature.additive.merge-total" => signature_additive_merge_total(cell, seed),
        "delta.additive.end-to-end" => delta_additive(cell, seed),
        "summary-tree.rebuild" => summary_tree_rebuild(cell, seed),
        "summary-tree.local-edit-total" => summary_tree_local_edit_total(cell, seed),
        "summary-tree.rebuild-edit-total" => summary_tree_rebuild_edit_total(cell, seed),
        "database.rebuild" => database_rebuild(cell, seed),
        "database.transaction-end-to-end" => database_transaction_end_to_end(cell, seed),
        "database.table-rebuild-total" => database_table_rebuild_total(cell, seed),
        "reconciliation.decode" => reconciliation_decode(cell),
        "graph.fast-prepared" => graph_fast(cell),
        "graph.exact" => graph_exact(cell),
        "graph.dag-reuse" => graph_dag_reuse(cell),
        "tool.binary-manifest.parse" => tool_binary_manifest(false),
        "tool.binary-manifest.generate" => tool_binary_manifest(true),
        other => Err(format!(
            "unsupported publication benchmark operation {other:?}"
        )),
    }
}

fn field_gf2_128(seed: u64) -> Result<PreparedOperation, String> {
    let mut state = Gf2_128V1::from_polynomial_bytes_mod(&seed_bytes::<16>(seed));
    let factor = Gf2_128V1::from_polynomial_bytes_mod(&seed_bytes::<16>(seed.rotate_left(17)));
    Ok(single_unit(move || {
        state *= factor;
        checksum_field(state)
    }))
}

fn field_gf2_256_hh(seed: u64) -> Result<PreparedOperation, String> {
    let mut state = Gf2_256HhV1::from_polynomial_bytes_mod(&seed_bytes::<32>(seed));
    let factor = Gf2_256HhV1::from_polynomial_bytes_mod(&seed_bytes::<32>(seed.rotate_left(17)));
    Ok(single_unit(move || {
        state *= factor;
        checksum_field(state)
    }))
}

fn field_gf2_256_alt(seed: u64) -> Result<PreparedOperation, String> {
    let mut state = Gf2_256AltV1::from_polynomial_bytes_mod(&seed_bytes::<32>(seed));
    let factor = Gf2_256AltV1::from_polynomial_bytes_mod(&seed_bytes::<32>(seed.rotate_left(17)));
    Ok(single_unit(move || {
        state *= factor;
        checksum_field(state)
    }))
}

fn field_fp251(seed: u64) -> Result<PreparedOperation, String> {
    let mut state = Fp251V1::from_u64_mod(seed % 250 + 1);
    let factor = Fp251V1::from_u64_mod(seed.rotate_left(17) % 249 + 2);
    Ok(single_unit(move || {
        state *= factor;
        checksum_field(state)
    }))
}

fn field_goldilocks(seed: u64) -> Result<PreparedOperation, String> {
    let mut state = FpGoldilocks64V1::from_u64_mod(seed);
    let factor = FpGoldilocks64V1::from_u64_mod(seed.rotate_left(17) | 1);
    Ok(single_unit(move || {
        state *= factor;
        checksum_field(state)
    }))
}

fn field_batch(scale: usize, seed: u64) -> Result<PreparedOperation, String> {
    if scale == 0 {
        return Err("batch scale must be positive".into());
    }
    let left = Gf2_128V1::from_polynomial_bytes_mod(&seed_bytes::<16>(seed));
    let right = Gf2_128V1::from_polynomial_bytes_mod(&seed_bytes::<16>(seed.rotate_left(31)));
    let lhs = vec![left; scale];
    let rhs = vec![right; scale];
    let mut output = vec![Gf2_128V1::ZERO; scale];
    let engine = Engine::<Gf2_128V1>::builder()
        .expected_batch(scale)
        .detect()
        .map_err(debug_error)?;
    Ok(PreparedOperation {
        logical_units_per_action: scale as u64,
        maximum_batch_iterations: None,
        run: Box::new(move || {
            engine.mul_into(&mut output, &lhs, &rhs).unwrap();
            checksum_field(output[scale - 1])
        }),
    })
}

fn field_scalar_total(scale: usize, seed: u64) -> Result<PreparedOperation, String> {
    if scale == 0 {
        return Err("scalar field scale must be positive".into());
    }
    let left = Gf2_128V1::from_polynomial_bytes_mod(&seed_bytes::<16>(seed));
    let right = Gf2_128V1::from_polynomial_bytes_mod(&seed_bytes::<16>(seed.rotate_left(31)));
    let lhs = vec![left; scale];
    let rhs = vec![right; scale];
    let mut output = vec![Gf2_128V1::ZERO; scale];
    Ok(single_unit(move || {
        for index in 0..scale {
            output[index] = lhs[index] * rhs[index];
        }
        checksum_field(output[scale - 1])
    }))
}

fn field_batch_total(scale: usize, seed: u64) -> Result<PreparedOperation, String> {
    let mut operation = field_batch(scale, seed)?;
    operation.logical_units_per_action = 1;
    Ok(operation)
}

fn signature_additive(cell: &BenchmarkCell, seed: u64) -> Result<PreparedOperation, String> {
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    Ok(scaled(cell.scale, move || {
        let mut signature = AdditiveSignature::<Fp251V1, _>::new(prime_encoder());
        signature
            .absorb_many(items.iter().map(Vec::as_slice))
            .unwrap();
        checksum_field(signature.state())
    }))
}

fn signature_sequence(cell: &BenchmarkCell, seed: u64) -> Result<PreparedOperation, String> {
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    Ok(scaled(cell.scale, move || {
        let mut signature =
            SequenceSignature::<Fp251V1, _>::new(prime_encoder(), Fp251V1::from_u64_mod(7))
                .unwrap();
        signature
            .push_many(items.iter().map(Vec::as_slice))
            .unwrap();
        checksum_field(signature.state())
    }))
}

fn signature_bidirectional(cell: &BenchmarkCell, seed: u64) -> Result<PreparedOperation, String> {
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    Ok(scaled(cell.scale, move || {
        let mut signature = BidirectionalSequenceSignature::<Fp251V1, _>::new(
            prime_encoder(),
            Fp251V1::from_u64_mod(7),
        )
        .unwrap();
        signature
            .push_many(items.iter().map(Vec::as_slice))
            .unwrap();
        checksum_field(signature.forward_state()) ^ checksum_field(signature.reverse_state())
    }))
}

fn signature_multiset(cell: &BenchmarkCell, seed: u64) -> Result<PreparedOperation, String> {
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    Ok(scaled(cell.scale, move || {
        let mut signature = MultisetSignature::<Fp251V1, _>::new(prime_encoder(), Fp251V1::ONE);
        signature
            .insert_many(items.iter().map(Vec::as_slice))
            .unwrap();
        checksum_field(signature.evaluated_product())
    }))
}

fn signature_multi_multiset(cell: &BenchmarkCell, seed: u64) -> Result<PreparedOperation, String> {
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    Ok(scaled(cell.scale, move || {
        let mut signature = MultiEvaluationMultisetSignature::<Fp251V1, _, 2>::new(
            prime_encoder(),
            [Fp251V1::ONE, Fp251V1::from_u64_mod(2)],
        )
        .unwrap();
        signature
            .insert_many(items.iter().map(Vec::as_slice))
            .unwrap();
        signature
            .evaluated_products()
            .into_iter()
            .fold(0, |sum, value| sum ^ checksum_field(value))
    }))
}

fn signature_multi_sequence(cell: &BenchmarkCell, seed: u64) -> Result<PreparedOperation, String> {
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    Ok(scaled(cell.scale, move || {
        let mut signature = MultiEvaluationSequenceSignature::<Fp251V1, _, 2>::new(
            prime_encoder(),
            [Fp251V1::from_u64_mod(7), Fp251V1::from_u64_mod(11)],
        )
        .unwrap();
        signature
            .push_many(items.iter().map(Vec::as_slice))
            .unwrap();
        signature
            .states()
            .iter()
            .fold(0, |sum, value| sum ^ checksum_field(*value))
    }))
}

fn signature_additive_total(cell: &BenchmarkCell, seed: u64) -> Result<PreparedOperation, String> {
    let mut operation = signature_additive(cell, seed)?;
    operation.logical_units_per_action = 1;
    Ok(operation)
}

fn signature_additive_merge_total(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    let midpoint = items.len() / 2;
    let mut left = AdditiveSignature::<Fp251V1, _>::new(prime_encoder());
    left.absorb_many(items[..midpoint].iter().map(Vec::as_slice))
        .map_err(debug_error)?;
    let mut right = AdditiveSignature::<Fp251V1, _>::new(prime_encoder());
    right
        .absorb_many(items[midpoint..].iter().map(Vec::as_slice))
        .map_err(debug_error)?;
    Ok(single_unit(move || {
        checksum_field(left.combine(&right).unwrap().state())
    }))
}

fn delta_additive(cell: &BenchmarkCell, seed: u64) -> Result<PreparedOperation, String> {
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    let empty = AdditiveSignature::<Fp251V1, _>::new(prime_encoder());
    let mut added = AdditiveSignature::<Fp251V1, _>::new(prime_encoder());
    added
        .absorb_many(items.iter().map(Vec::as_slice))
        .map_err(debug_error)?;
    let namespace = ApplicationNamespace::derive(b"publication-delta-v1");
    let delta = AdditiveDelta::new(namespace, 0, empty.clone(), added).map_err(debug_error)?;
    Ok(scaled(cell.scale, move || {
        let mut revisioned = RevisionedSignature::new(namespace, empty.clone());
        revisioned.apply(&delta).unwrap();
        checksum_field(revisioned.state().state())
    }))
}

fn summary_tree_rebuild(cell: &BenchmarkCell, seed: u64) -> Result<PreparedOperation, String> {
    let mut bytes = deterministic_bytes(cell.scale, seed);
    let profile = FileChunkProfile::fixed(4_096).map_err(debug_error)?;
    Ok(scaled(cell.scale, move || {
        if !bytes.is_empty() {
            bytes[0] ^= 1;
        }
        let tree = build_tree(profile, &bytes).unwrap();
        checksum_field(tree.root().evaluation())
    }))
}

fn summary_tree_local_edit_total(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    let total = cell.dataset_size.unwrap_or(cell.scale);
    if cell.scale > total || total < 4_096 {
        return Err("invalid summary-tree edit/dataset scale".into());
    }
    let original = deterministic_bytes(total, seed);
    let profile = FileChunkProfile::fixed(4_096).map_err(debug_error)?;
    let mut tree = build_tree(profile, &original)?;
    let start = (total - cell.scale) / 2;
    let end = start + cell.scale;
    let mut replacement = vec![0xa5; cell.scale];
    let mut byte = 0xa5_u8;
    Ok(single_unit(move || {
        byte ^= 0xff;
        replacement.fill(byte);
        tree.replace_range(start..end, &replacement).unwrap();
        checksum_field(tree.root().evaluation())
    }))
}

fn summary_tree_rebuild_edit_total(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    let total = cell.dataset_size.unwrap_or(cell.scale);
    if cell.scale > total || total < 4_096 {
        return Err("invalid summary-tree rebuild/dataset scale".into());
    }
    let mut bytes = deterministic_bytes(total, seed);
    let profile = FileChunkProfile::fixed(4_096).map_err(debug_error)?;
    let start = (total - cell.scale) / 2;
    let end = start + cell.scale;
    let mut byte = 0xa5_u8;
    Ok(single_unit(move || {
        byte ^= 0xff;
        bytes[start..end].fill(byte);
        checksum_field(build_tree(profile, &bytes).unwrap().root().evaluation())
    }))
}

fn database_rebuild(cell: &BenchmarkCell, seed: u64) -> Result<PreparedOperation, String> {
    let schema = database_schema()?;
    let namespace = database_namespace();
    let mut version = seed;
    let rows = (0..cell.scale)
        .map(|id| database_row(id as u64, 1))
        .collect::<Vec<_>>();
    Ok(scaled(cell.scale, move || {
        version = version.wrapping_add(1);
        let mut candidate = rows.clone();
        if let Some(row) = candidate.first_mut() {
            *row = database_row(0, version);
        }
        let database = Database::from_rows(
            namespace,
            schema.clone(),
            16,
            binary_encoder(),
            Gf2_128V1::ONE,
            candidate,
        )
        .unwrap();
        checksum_field(database.summary().unwrap().evaluation())
    }))
}

fn database_transaction_end_to_end(
    cell: &BenchmarkCell,
    _seed: u64,
) -> Result<PreparedOperation, String> {
    let row_count = cell.dataset_size.unwrap_or(cell.scale);
    if cell.scale > row_count || row_count == 0 {
        return Err("invalid database mutation/dataset scale".into());
    }
    let schema = database_schema()?;
    let namespace = database_namespace();
    let rows = (0..row_count)
        .map(|id| database_row(id as u64, 1))
        .collect::<Vec<_>>();
    let mut database = Database::from_rows(
        namespace,
        schema.clone(),
        16,
        binary_encoder(),
        Gf2_128V1::ONE,
        rows,
    )
    .map_err(debug_error)?;
    let mut revision = 0_u64;
    let mutation_count = cell.scale;
    Ok(PreparedOperation {
        logical_units_per_action: 1,
        maximum_batch_iterations: Some(1),
        run: Box::new(move || {
            let mutations = (0..mutation_count)
                .map(|id| homomorphic_hash_rs::RowMutation::Update {
                    before: database_row(id as u64, revision + 1),
                    after: database_row(id as u64, revision + 2),
                })
                .collect();
            let transaction =
                homomorphic_hash_rs::TransactionDelta::new(namespace, &schema, revision, mutations)
                    .unwrap();
            database
                .apply_transaction(
                    &transaction,
                    homomorphic_hash_rs::DatabaseTransactionLimits::default(),
                )
                .unwrap();
            revision += 1;
            checksum_field(database.summary().unwrap().evaluation())
        }),
    })
}

fn database_table_rebuild_total(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    let row_count = cell.dataset_size.unwrap_or(cell.scale);
    if cell.scale > row_count || row_count == 0 {
        return Err("invalid database rebuild/dataset scale".into());
    }
    let schema = database_schema()?;
    let namespace = database_namespace();
    let rows = (0..row_count)
        .map(|id| database_row(id as u64, 1))
        .collect::<Vec<_>>();
    let mutation_count = cell.scale;
    let mut version = seed;
    Ok(single_unit(move || {
        version = version.wrapping_add(1);
        let mut candidate = rows.clone();
        for (id, row) in candidate.iter_mut().enumerate().take(mutation_count) {
            *row = database_row(id as u64, version);
        }
        let database = Database::from_rows(
            namespace,
            schema.clone(),
            16,
            binary_encoder(),
            Gf2_128V1::ONE,
            candidate,
        )
        .unwrap();
        checksum_field(database.summary().unwrap().evaluation())
    }))
}

fn reconciliation_decode(cell: &BenchmarkCell) -> Result<PreparedOperation, String> {
    let difference = cell.scale.clamp(2, 32);
    let reconciler = BoundedSetReconciler::new(ReconciliationLimits::new(160, 64, 64, 65_536))
        .map_err(debug_error)?;
    let left = (0..128_u16).collect::<Vec<_>>();
    let mut right = left.clone();
    for (index, value) in right.iter_mut().take(difference).enumerate() {
        *value = 128 + index as u16;
    }
    right.sort_unstable();
    let left_sketch = reconciler.sketch(&left).map_err(debug_error)?;
    let right_sketch = reconciler.sketch(&right).map_err(debug_error)?;
    Ok(scaled(difference, move || {
        let recovered = reconciler
            .reconcile(&left_sketch, &right_sketch, &right)
            .unwrap();
        (recovered.only_left().len() ^ recovered.only_right().len()) as u64
    }))
}

fn graph_fast(cell: &BenchmarkCell) -> Result<PreparedOperation, String> {
    // `PreparedGraph` borrows the immutable graph. The worker is a short-lived
    // process, so retaining this input until process exit keeps setup outside
    // the measured operation without a self-referential owner.
    let graph: &'static IncidenceGraph = Box::leak(Box::new(sparse_cycle(cell.scale.max(4))?));
    let labeler = FastGraphLabeler::<Fp251V1, _, 2>::new(
        prime_encoder(),
        homomorphic_hash_rs::RefinementProfile::fast(),
    )
    .map_err(debug_error)?;
    let prepared = labeler.prepare(graph).map_err(debug_error)?;
    let mut workspace = GraphWorkspace::new();
    workspace.reserve_for(graph.vertex_count(), 4);
    Ok(scaled(graph.vertex_count(), move || {
        let analysis = labeler
            .analyze_prepared_with_workspace(&prepared, &mut workspace, GraphExecution::Sequential)
            .unwrap();
        checksum_field(analysis.signature().lanes()[0])
    }))
}

fn graph_exact(cell: &BenchmarkCell) -> Result<PreparedOperation, String> {
    let graph = distinct_path(cell.scale.clamp(2, 14))?;
    let schema = GraphSchemaId::derive(b"publication-exact-v1");
    let canonizer = Microcanon::new(schema);
    let budget = CanonicalSearchBudget::new(1_000_000);
    Ok(scaled(graph.vertex_count(), move || {
        match canonizer.canonicalize(&graph, budget).unwrap() {
            MicrocanonOutcome::Exact { form, .. } => checksum_bytes(form.bytes()),
            MicrocanonOutcome::Inconclusive { report } => panic!("inconclusive: {report:?}"),
        }
    }))
}

fn graph_dag_reuse(cell: &BenchmarkCell) -> Result<PreparedOperation, String> {
    let graph = distinct_path(cell.scale.clamp(2, 14))?;
    let schema = GraphSchemaId::derive(b"publication-dag-v1");
    let canonizer = Microcanon::new(schema);
    let budget = CanonicalSearchBudget::new(1_000_000);
    let mut dag = CanonicalGraphDag::new(schema);
    dag.resolve(&graph, &canonizer, budget, &[], None)
        .map_err(debug_error)?;
    Ok(scaled(graph.vertex_count(), move || {
        let outcome = dag
            .resolve(&graph, &canonizer, budget, &[], Some(dag.revision()))
            .unwrap();
        match outcome {
            homomorphic_hash_rs::GraphDagResolveOutcome::Reused { node, .. } => node.as_u64(),
            other => panic!("expected DAG reuse, got {other:?}"),
        }
    }))
}

fn tool_binary_manifest(generate: bool) -> Result<PreparedOperation, String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../microfield/fields/gf2_128_v1.toml");
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("read binary field manifest {}: {error}", path.display()))?;
    Ok(single_unit(move || {
        let factory = BinaryFieldFactory::from_manifest_toml(&source).unwrap();
        if generate {
            let package = factory.generate().unwrap();
            checksum_bytes(package.rust_source())
        } else {
            source.len() as u64
        }
    }))
}

fn single_unit(action: impl FnMut() -> u64 + 'static) -> PreparedOperation {
    PreparedOperation {
        logical_units_per_action: 1,
        maximum_batch_iterations: None,
        run: Box::new(action),
    }
}

fn scaled(scale: usize, action: impl FnMut() -> u64 + 'static) -> PreparedOperation {
    PreparedOperation {
        logical_units_per_action: scale.max(1) as u64,
        maximum_batch_iterations: None,
        run: Box::new(action),
    }
}

fn seed_bytes<const N: usize>(seed: u64) -> [u8; N] {
    let mut bytes = [0_u8; N];
    let mut state = seed;
    for chunk in bytes.chunks_mut(8) {
        state = state
            .wrapping_add(0x9e37_79b9_7f4a_7c15)
            .rotate_left(17)
            .wrapping_mul(0xbf58_476d_1ce4_e5b9);
        chunk.copy_from_slice(&state.to_le_bytes()[..chunk.len()]);
    }
    bytes
}

fn payloads(count: usize, payload_bytes: usize, seed: u64) -> Result<Vec<Vec<u8>>, String> {
    if count == 0 || payload_bytes == 0 || payload_bytes > 1_048_576 {
        return Err("invalid publication payload dimensions".into());
    }
    Ok((0..count)
        .map(|index| deterministic_bytes(payload_bytes, seed ^ index as u64))
        .collect())
}

fn deterministic_bytes(count: usize, seed: u64) -> Vec<u8> {
    let mut state = seed;
    (0..count)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect()
}

fn prime_encoder() -> PrimeIntegerEncoder {
    PrimeIntegerEncoder::new(0x4250_5542_0001)
}

fn binary_encoder() -> BinaryEncoder {
    BinaryEncoder::new(0x4250_5542_0002)
}

fn build_tree(profile: FileChunkProfile, bytes: &[u8]) -> Result<SummaryTree, String> {
    SummaryTree::from_bytes(
        profile,
        binary_encoder(),
        Gf2_128V1::from_polynomial_bytes_mod(&[2]),
        bytes,
    )
    .map_err(debug_error)
}

fn database_namespace() -> ApplicationNamespace {
    ApplicationNamespace::derive(b"publication-database-v1")
}

fn database_schema() -> Result<DatabaseSchema, String> {
    DatabaseSchema::new(
        1,
        vec![
            DatabaseColumn::new("id", DatabaseColumnType::U64, false),
            DatabaseColumn::new("payload", DatabaseColumnType::Bytes, false),
        ],
        vec![0],
    )
    .map_err(debug_error)
}

fn database_row(id: u64, version: u64) -> DatabaseRow {
    DatabaseRow::new(
        version,
        vec![
            DatabaseValue::U64(id),
            DatabaseValue::Bytes(id.rotate_left(13).to_le_bytes().to_vec()),
        ],
    )
}

fn sparse_cycle(vertices: usize) -> Result<IncidenceGraph, String> {
    let mut builder = IncidenceGraphBuilder::new();
    let ids = (0..vertices)
        .map(|_| builder.add_vertex(Vec::new()))
        .collect::<Vec<_>>();
    for index in 0..vertices {
        builder
            .add_undirected_relation(
                ids[index],
                ids[(index + 1) % vertices],
                b"edge",
                b"cycle",
                1,
            )
            .map_err(debug_error)?;
    }
    builder.build().map_err(debug_error)
}

fn distinct_path(vertices: usize) -> Result<IncidenceGraph, String> {
    let mut builder = IncidenceGraphBuilder::new();
    let ids = (0..vertices)
        .map(|index| builder.add_vertex((index as u64).to_le_bytes().to_vec()))
        .collect::<Vec<_>>();
    for pair in ids.windows(2) {
        builder
            .add_undirected_relation(pair[0], pair[1], b"edge", b"path", 1)
            .map_err(debug_error)?;
    }
    builder.build().map_err(debug_error)
}

fn checksum_field<F: CanonicalEncoding>(value: F) -> u64 {
    checksum_bytes(value.to_canonical().as_ref())
}

fn checksum_bytes(bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .enumerate()
        .fold(0xcbf2_9ce4_8422_2325, |state, (index, byte)| {
            (state ^ u64::from(*byte) ^ index as u64).wrapping_mul(0x0000_0100_0000_01b3)
        })
}

fn debug_error(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(operation: &str, scale: usize) -> BenchmarkCell {
        BenchmarkCell {
            id: operation.into(),
            family: operation.split('.').next().unwrap().into(),
            operation: operation.into(),
            scale,
            scale_unit: "units".into(),
            payload_bytes: 16,
            dataset_size: None,
            baseline_cell: None,
            curve: None,
            strategy: None,
        }
    }

    #[test]
    fn every_registered_operation_prepares_and_runs() {
        for operation in SUPPORTED_OPERATIONS {
            let scale = if operation.contains("graph.exact") || operation.contains("dag") {
                8
            } else if operation.contains("tool.") {
                1
            } else {
                16
            };
            let mut benchmark_cell = cell(operation, scale);
            if operation.contains("summary-tree.") {
                benchmark_cell.dataset_size = Some(4_096);
            }
            let mut prepared = prepare(&benchmark_cell, 7)
                .unwrap_or_else(|error| panic!("prepare {operation}: {error}"));
            assert_ne!((prepared.run)(), u64::MAX, "{operation}");
            assert!(prepared.logical_units_per_action > 0);
        }
    }

    #[test]
    fn payload_generation_is_seeded_and_reproducible() {
        assert_eq!(payloads(4, 16, 7).unwrap(), payloads(4, 16, 7).unwrap());
        assert_ne!(payloads(4, 16, 7).unwrap(), payloads(4, 16, 8).unwrap());
    }
}
