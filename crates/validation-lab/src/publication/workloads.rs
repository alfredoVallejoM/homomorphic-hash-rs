use std::fs;

use algesum::{
    AdditiveDelta, AdditiveSignature, ApplicationNamespace, BidirectionalSequenceSignature,
    BinaryPolynomialEncoder, BoundedSetReconciler, CanonicalBudgetLimit, CanonicalGraphDag,
    CanonicalSearchBudget, DatabaseApplyPolicy, DatabaseColumn, DatabaseColumnType, DatabaseRow,
    DatabaseSchema, DatabaseTransactionLimits, DatabaseValue, FastGraphLabeler, FileChunkProfile,
    GraphExecution, GraphSchemaId, GraphWorkspace, HomomorphicSummaryTree, IncidenceGraph,
    IncidenceGraphBuilder, IncrementalGraphWorkspace, Microcanon, MicrocanonOutcome,
    MicrocanonPath, MultiEvaluationMultisetSignature, MultiEvaluationSequenceSignature,
    MultisetSignature, PartitionedDatabase, PrimeIntegerEncoder, ReconciliationLimits,
    RevisionedSignature, RowMutation, SequenceSignature, SummaryEditPolicy, SummaryRangeEdit,
    TransactionDelta,
};
use microfield::{
    generator::BinaryFieldFactory, BinaryPolynomialField, CanonicalEncoding, Engine, Field,
    Fp251V1, Fp256GenericV1, FpGoldilocks64V1, Gf2_128V1, Gf2_256AltV1, Gf2_256HhV1, Invert,
    PrimeField, Square,
};

use super::model::{
    BenchmarkCell, GraphExactLimit, GraphExactOutcome, GraphExactPath, GraphExactTelemetry,
};

type BinaryEncoder = BinaryPolynomialEncoder;
type Database = PartitionedDatabase<Gf2_128V1, BinaryEncoder>;
type SummaryTree = HomomorphicSummaryTree<Gf2_128V1, BinaryEncoder>;

pub const SUPPORTED_OPERATIONS: &[&str] = &[
    "field.gf2-128.add-total",
    "field.gf2-128.mul-total",
    "field.gf2-128.square-total",
    "field.gf2-128.invert-total",
    "field.gf2-128.canonical-roundtrip-total",
    "field.gf2-256-hh.add-total",
    "field.gf2-256-hh.mul-total",
    "field.gf2-256-hh.square-total",
    "field.gf2-256-hh.invert-total",
    "field.gf2-256-hh.canonical-roundtrip-total",
    "field.gf2-256-alt.add-total",
    "field.gf2-256-alt.mul-total",
    "field.gf2-256-alt.square-total",
    "field.gf2-256-alt.invert-total",
    "field.gf2-256-alt.canonical-roundtrip-total",
    "field.fp251.add-total",
    "field.fp251.mul-total",
    "field.fp251.square-total",
    "field.fp251.invert-total",
    "field.fp251.canonical-roundtrip-total",
    "field.goldilocks.add-total",
    "field.goldilocks.mul-total",
    "field.goldilocks.square-total",
    "field.goldilocks.invert-total",
    "field.goldilocks.canonical-roundtrip-total",
    "field.fp256-generic.add-total",
    "field.fp256-generic.mul-total",
    "field.fp256-generic.square-total",
    "field.fp256-generic.invert-total",
    "field.fp256-generic.canonical-roundtrip-total",
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
    "signature.sequence.build-total",
    "signature.bidirectional.build-total",
    "signature.multiset.build-total",
    "signature.multi-multiset-k2.build-total",
    "signature.multi-sequence-k2.build-total",
    "signature.additive.merge-total",
    "signature.sequence.concatenate-total",
    "signature.bidirectional.concatenate-total",
    "signature.multiset.merge-total",
    "signature.multi-multiset-k2.merge-total",
    "signature.multi-sequence-k2.concatenate-total",
    "signature.multi-multiset-k3.build-total",
    "signature.multi-multiset-k4.build-total",
    "signature.multi-sequence-k3.build-total",
    "signature.multi-sequence-k4.build-total",
    "signature.multi-multiset-k3.merge-total",
    "signature.multi-multiset-k4.merge-total",
    "signature.multi-sequence-k3.concatenate-total",
    "signature.multi-sequence-k4.concatenate-total",
    "signature.multi-multiset-k4.fragmented-merge-total",
    "signature.multi-sequence-k4.fragmented-concatenate-total",
    "delta.additive.end-to-end",
    "summary-tree.rebuild",
    "summary-tree.local-edit-total",
    "summary-tree.rebuild-edit-total",
    "summary-tree.sequential-batch-total",
    "summary-tree.bulk-batch-total",
    "summary-tree.adaptive-batch-total",
    "summary-tree.rebuild-batch-total",
    "database.rebuild",
    "database.transaction-end-to-end",
    "database.table-rebuild-total",
    "database.bulk-transaction-total",
    "database.adaptive-transaction-total",
    "database.selected-rebuild-total",
    "reconciliation.decode",
    "graph.fast-prepared",
    "graph.exact",
    "graph.dag-reuse",
    "graph.full-label-reanalysis-total",
    "graph.incremental-label-update-total",
    "graph.full-topology-reanalysis-total",
    "graph.incremental-topology-update-total",
    "tool.binary-manifest.parse",
    "tool.binary-manifest.generate",
];

pub struct PreparedOperation {
    pub logical_units_per_action: u64,
    pub maximum_batch_iterations: Option<u64>,
    pub graph_exact: Option<GraphExactTelemetry>,
    pub run: Box<dyn FnMut() -> u64>,
}

pub fn prepare(cell: &BenchmarkCell, seed: u64) -> Result<PreparedOperation, String> {
    if let Some(operation) = prepare_c3_field_primitive(cell, seed)? {
        return Ok(operation);
    }
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
        "signature.sequence.build-total" => signature_total(signature_sequence(cell, seed)?),
        "signature.bidirectional.build-total" => {
            signature_total(signature_bidirectional(cell, seed)?)
        }
        "signature.multiset.build-total" => signature_total(signature_multiset(cell, seed)?),
        "signature.multi-multiset-k2.build-total" => {
            signature_total(signature_multi_multiset(cell, seed)?)
        }
        "signature.multi-sequence-k2.build-total" => {
            signature_total(signature_multi_sequence(cell, seed)?)
        }
        "signature.additive.merge-total" => signature_additive_merge_total(cell, seed),
        "signature.sequence.concatenate-total" => signature_sequence_concatenate_total(cell, seed),
        "signature.bidirectional.concatenate-total" => {
            signature_bidirectional_concatenate_total(cell, seed)
        }
        "signature.multiset.merge-total" => signature_multiset_merge_total(cell, seed),
        "signature.multi-multiset-k2.merge-total" => {
            signature_multi_multiset_merge_total(cell, seed)
        }
        "signature.multi-sequence-k2.concatenate-total" => {
            signature_multi_sequence_concatenate_total(cell, seed)
        }
        "signature.multi-multiset-k3.build-total" => signature_multi_multiset_k3(cell, seed),
        "signature.multi-multiset-k4.build-total" => signature_multi_multiset_k4(cell, seed),
        "signature.multi-sequence-k3.build-total" => signature_multi_sequence_k3(cell, seed),
        "signature.multi-sequence-k4.build-total" => signature_multi_sequence_k4(cell, seed),
        "signature.multi-multiset-k3.merge-total" => signature_multi_multiset_merge_k3(cell, seed),
        "signature.multi-multiset-k4.merge-total" => signature_multi_multiset_merge_k4(cell, seed),
        "signature.multi-sequence-k3.concatenate-total" => {
            signature_multi_sequence_concatenate_k3(cell, seed)
        }
        "signature.multi-sequence-k4.concatenate-total" => {
            signature_multi_sequence_concatenate_k4(cell, seed)
        }
        "signature.multi-multiset-k4.fragmented-merge-total" => {
            signature_multi_multiset_fragmented_k4(cell, seed)
        }
        "signature.multi-sequence-k4.fragmented-concatenate-total" => {
            signature_multi_sequence_fragmented_k4(cell, seed)
        }
        "delta.additive.end-to-end" => delta_additive(cell, seed),
        "summary-tree.rebuild" => summary_tree_rebuild(cell, seed),
        "summary-tree.local-edit-total" => summary_tree_local_edit_total(cell, seed),
        "summary-tree.rebuild-edit-total" => summary_tree_rebuild_edit_total(cell, seed),
        "summary-tree.sequential-batch-total" => {
            summary_tree_batch(cell, seed, SummaryBatchMode::Sequential)
        }
        "summary-tree.bulk-batch-total" => summary_tree_batch(cell, seed, SummaryBatchMode::Bulk),
        "summary-tree.adaptive-batch-total" => {
            summary_tree_batch(cell, seed, SummaryBatchMode::Adaptive)
        }
        "summary-tree.rebuild-batch-total" => {
            summary_tree_batch(cell, seed, SummaryBatchMode::Rebuild)
        }
        "database.rebuild" => database_rebuild(cell, seed),
        "database.transaction-end-to-end" => database_transaction_end_to_end(cell, seed),
        "database.table-rebuild-total" => database_table_rebuild_total(cell, seed),
        "database.bulk-transaction-total" => {
            database_selected_transaction(cell, DatabaseBatchMode::Bulk)
        }
        "database.adaptive-transaction-total" => {
            database_selected_transaction(cell, DatabaseBatchMode::Adaptive)
        }
        "database.selected-rebuild-total" => database_selected_rebuild(cell),
        "reconciliation.decode" => reconciliation_decode(cell),
        "graph.fast-prepared" => graph_fast(cell),
        "graph.exact" => graph_exact(cell),
        "graph.dag-reuse" => graph_dag_reuse(cell),
        "graph.full-label-reanalysis-total" => graph_full_label_reanalysis(cell),
        "graph.incremental-label-update-total" => graph_incremental_label_update(cell),
        "graph.full-topology-reanalysis-total" => graph_full_topology_reanalysis(cell),
        "graph.incremental-topology-update-total" => graph_incremental_topology_update(cell),
        "tool.binary-manifest.parse" => tool_binary_manifest(false),
        "tool.binary-manifest.generate" => tool_binary_manifest(true),
        other => Err(format!(
            "unsupported publication benchmark operation {other:?}"
        )),
    }
}

#[derive(Clone, Copy)]
enum FieldPrimitive {
    Add,
    Mul,
    Square,
    Invert,
    CanonicalRoundtrip,
}

fn prepare_c3_field_primitive(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<Option<PreparedOperation>, String> {
    let Some((field, operation)) = cell
        .operation
        .strip_prefix("field.")
        .and_then(|rest| rest.rsplit_once('.'))
    else {
        return Ok(None);
    };
    let primitive = match operation {
        "add-total" => FieldPrimitive::Add,
        "mul-total" => FieldPrimitive::Mul,
        "square-total" => FieldPrimitive::Square,
        "invert-total" => FieldPrimitive::Invert,
        "canonical-roundtrip-total" => FieldPrimitive::CanonicalRoundtrip,
        _ => return Ok(None),
    };
    let scale = cell.scale;
    let prepared = match field {
        "gf2-128" => field_primitive_total(
            scale,
            Gf2_128V1::from_polynomial_bytes_mod(&seed_bytes::<16>(seed)),
            Gf2_128V1::from_polynomial_bytes_mod(&seed_bytes::<16>(seed.rotate_left(31))),
            primitive,
        )?,
        "gf2-256-hh" => field_primitive_total(
            scale,
            Gf2_256HhV1::from_polynomial_bytes_mod(&seed_bytes::<32>(seed)),
            Gf2_256HhV1::from_polynomial_bytes_mod(&seed_bytes::<32>(seed.rotate_left(31))),
            primitive,
        )?,
        "gf2-256-alt" => field_primitive_total(
            scale,
            Gf2_256AltV1::from_polynomial_bytes_mod(&seed_bytes::<32>(seed)),
            Gf2_256AltV1::from_polynomial_bytes_mod(&seed_bytes::<32>(seed.rotate_left(31))),
            primitive,
        )?,
        "fp251" => field_primitive_total(
            scale,
            Fp251V1::from_u64_mod(seed % 250 + 1),
            Fp251V1::from_u64_mod(seed.rotate_left(31) % 250 + 1),
            primitive,
        )?,
        "goldilocks" => field_primitive_total(
            scale,
            FpGoldilocks64V1::from_u64_mod(seed | 1),
            FpGoldilocks64V1::from_u64_mod(seed.rotate_left(31) | 1),
            primitive,
        )?,
        "fp256-generic" => field_primitive_total(
            scale,
            Fp256GenericV1::from_bytes_mod_order(&seed_bytes::<32>(seed)),
            Fp256GenericV1::from_bytes_mod_order(&seed_bytes::<32>(seed.rotate_left(31))),
            primitive,
        )?,
        _ => return Ok(None),
    };
    Ok(Some(prepared))
}

fn field_primitive_total<F>(
    scale: usize,
    mut left: F,
    mut right: F,
    primitive: FieldPrimitive,
) -> Result<PreparedOperation, String>
where
    F: Field + Square + Invert + CanonicalEncoding,
{
    if scale == 0 {
        return Err("field primitive scale must be positive".into());
    }
    if left.is_zero() {
        left = F::ONE;
    }
    if right.is_zero() {
        right = F::ONE;
    }
    let left = vec![left; scale];
    let rhs = vec![right; scale];
    let mut output = vec![F::ZERO; scale];
    Ok(single_unit(move || {
        for ((destination, value), operand) in output.iter_mut().zip(&left).zip(&rhs) {
            *destination = match primitive {
                FieldPrimitive::Add => value.add(*operand),
                FieldPrimitive::Mul => value.mul(*operand),
                FieldPrimitive::Square => value.square(),
                FieldPrimitive::Invert => value.invert().expect("inputs are non-zero"),
                FieldPrimitive::CanonicalRoundtrip => {
                    F::from_canonical(&value.to_canonical()).expect("self-encoding is canonical")
                }
            };
        }
        output
            .iter()
            .enumerate()
            .fold(0_u64, |checksum, (index, value)| {
                checksum.rotate_left(7) ^ checksum_field(*value) ^ index as u64
            })
    }))
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
        graph_exact: None,
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

fn signature_total(mut operation: PreparedOperation) -> Result<PreparedOperation, String> {
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
        checksum_field(
            std::hint::black_box(&left)
                .combine(std::hint::black_box(&right))
                .unwrap()
                .state(),
        )
    }))
}

fn signature_sequence_concatenate_total(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    let midpoint = items.len() / 2;
    let mut left = SequenceSignature::<Fp251V1, _>::new(prime_encoder(), Fp251V1::from_u64_mod(7))
        .map_err(debug_error)?;
    left.push_many(items[..midpoint].iter().map(Vec::as_slice))
        .map_err(debug_error)?;
    let mut right = SequenceSignature::<Fp251V1, _>::new(prime_encoder(), Fp251V1::from_u64_mod(7))
        .map_err(debug_error)?;
    right
        .push_many(items[midpoint..].iter().map(Vec::as_slice))
        .map_err(debug_error)?;
    Ok(single_unit(move || {
        checksum_field(
            std::hint::black_box(&left)
                .concatenate(std::hint::black_box(&right))
                .unwrap()
                .state(),
        )
    }))
}

fn signature_bidirectional_concatenate_total(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    let midpoint = items.len() / 2;
    let mut left = BidirectionalSequenceSignature::<Fp251V1, _>::new(
        prime_encoder(),
        Fp251V1::from_u64_mod(7),
    )
    .map_err(debug_error)?;
    left.push_many(items[..midpoint].iter().map(Vec::as_slice))
        .map_err(debug_error)?;
    let mut right = BidirectionalSequenceSignature::<Fp251V1, _>::new(
        prime_encoder(),
        Fp251V1::from_u64_mod(7),
    )
    .map_err(debug_error)?;
    right
        .push_many(items[midpoint..].iter().map(Vec::as_slice))
        .map_err(debug_error)?;
    Ok(single_unit(move || {
        let combined = std::hint::black_box(&left)
            .concatenate(std::hint::black_box(&right))
            .unwrap();
        checksum_field(combined.forward_state()) ^ checksum_field(combined.reverse_state())
    }))
}

fn signature_multiset_merge_total(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    let midpoint = items.len() / 2;
    let mut left = MultisetSignature::<Fp251V1, _>::new(prime_encoder(), Fp251V1::ONE);
    left.insert_many(items[..midpoint].iter().map(Vec::as_slice))
        .map_err(debug_error)?;
    let mut right = MultisetSignature::<Fp251V1, _>::new(prime_encoder(), Fp251V1::ONE);
    right
        .insert_many(items[midpoint..].iter().map(Vec::as_slice))
        .map_err(debug_error)?;
    Ok(single_unit(move || {
        checksum_field(
            std::hint::black_box(&left)
                .combine(std::hint::black_box(&right))
                .unwrap()
                .evaluated_product(),
        )
    }))
}

fn signature_multi_multiset_merge_total(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    let midpoint = items.len() / 2;
    let points = [Fp251V1::ONE, Fp251V1::from_u64_mod(2)];
    let mut left = MultiEvaluationMultisetSignature::<Fp251V1, _, 2>::new(prime_encoder(), points)
        .map_err(debug_error)?;
    left.insert_many(items[..midpoint].iter().map(Vec::as_slice))
        .map_err(debug_error)?;
    let mut right = MultiEvaluationMultisetSignature::<Fp251V1, _, 2>::new(prime_encoder(), points)
        .map_err(debug_error)?;
    right
        .insert_many(items[midpoint..].iter().map(Vec::as_slice))
        .map_err(debug_error)?;
    Ok(single_unit(move || {
        std::hint::black_box(&left)
            .combine(std::hint::black_box(&right))
            .unwrap()
            .evaluated_products()
            .into_iter()
            .fold(0, |sum, value| sum ^ checksum_field(value))
    }))
}

fn signature_multi_sequence_concatenate_total(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    let midpoint = items.len() / 2;
    let bases = [Fp251V1::from_u64_mod(7), Fp251V1::from_u64_mod(11)];
    let mut left = MultiEvaluationSequenceSignature::<Fp251V1, _, 2>::new(prime_encoder(), bases)
        .map_err(debug_error)?;
    left.push_many(items[..midpoint].iter().map(Vec::as_slice))
        .map_err(debug_error)?;
    let mut right = MultiEvaluationSequenceSignature::<Fp251V1, _, 2>::new(prime_encoder(), bases)
        .map_err(debug_error)?;
    right
        .push_many(items[midpoint..].iter().map(Vec::as_slice))
        .map_err(debug_error)?;
    Ok(single_unit(move || {
        std::hint::black_box(&left)
            .concatenate(std::hint::black_box(&right))
            .unwrap()
            .states()
            .iter()
            .fold(0, |sum, value| sum ^ checksum_field(*value))
    }))
}

fn signature_multi_multiset_k3(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    signature_multi_multiset_build_k(
        cell,
        seed,
        [
            Fp251V1::ONE,
            Fp251V1::from_u64_mod(2),
            Fp251V1::from_u64_mod(3),
        ],
    )
}

fn signature_multi_multiset_k4(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    signature_multi_multiset_build_k(
        cell,
        seed,
        [
            Fp251V1::ONE,
            Fp251V1::from_u64_mod(2),
            Fp251V1::from_u64_mod(3),
            Fp251V1::from_u64_mod(4),
        ],
    )
}

fn signature_multi_sequence_k3(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    signature_multi_sequence_build_k(
        cell,
        seed,
        [
            Fp251V1::from_u64_mod(7),
            Fp251V1::from_u64_mod(11),
            Fp251V1::from_u64_mod(13),
        ],
    )
}

fn signature_multi_sequence_k4(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    signature_multi_sequence_build_k(
        cell,
        seed,
        [
            Fp251V1::from_u64_mod(7),
            Fp251V1::from_u64_mod(11),
            Fp251V1::from_u64_mod(13),
            Fp251V1::from_u64_mod(17),
        ],
    )
}

fn signature_multi_multiset_build_k<const K: usize>(
    cell: &BenchmarkCell,
    seed: u64,
    points: [Fp251V1; K],
) -> Result<PreparedOperation, String> {
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    Ok(single_unit(move || {
        let mut signature =
            MultiEvaluationMultisetSignature::<Fp251V1, _, K>::new(prime_encoder(), points)
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

fn signature_multi_sequence_build_k<const K: usize>(
    cell: &BenchmarkCell,
    seed: u64,
    bases: [Fp251V1; K],
) -> Result<PreparedOperation, String> {
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    Ok(single_unit(move || {
        let mut signature =
            MultiEvaluationSequenceSignature::<Fp251V1, _, K>::new(prime_encoder(), bases).unwrap();
        signature
            .push_many(items.iter().map(Vec::as_slice))
            .unwrap();
        signature
            .states()
            .iter()
            .fold(0, |sum, value| sum ^ checksum_field(*value))
    }))
}

fn signature_multi_multiset_merge_k3(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    signature_multi_multiset_merge_k(
        cell,
        seed,
        [
            Fp251V1::ONE,
            Fp251V1::from_u64_mod(2),
            Fp251V1::from_u64_mod(3),
        ],
    )
}

fn signature_multi_multiset_merge_k4(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    signature_multi_multiset_merge_k(
        cell,
        seed,
        [
            Fp251V1::ONE,
            Fp251V1::from_u64_mod(2),
            Fp251V1::from_u64_mod(3),
            Fp251V1::from_u64_mod(4),
        ],
    )
}

fn signature_multi_multiset_merge_k<const K: usize>(
    cell: &BenchmarkCell,
    seed: u64,
    points: [Fp251V1; K],
) -> Result<PreparedOperation, String> {
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    let midpoint = items.len() / 2;
    let mut left = MultiEvaluationMultisetSignature::<Fp251V1, _, K>::new(prime_encoder(), points)
        .map_err(debug_error)?;
    left.insert_many(items[..midpoint].iter().map(Vec::as_slice))
        .map_err(debug_error)?;
    let mut right = MultiEvaluationMultisetSignature::<Fp251V1, _, K>::new(prime_encoder(), points)
        .map_err(debug_error)?;
    right
        .insert_many(items[midpoint..].iter().map(Vec::as_slice))
        .map_err(debug_error)?;
    Ok(single_unit(move || {
        std::hint::black_box(&left)
            .combine(std::hint::black_box(&right))
            .unwrap()
            .evaluated_products()
            .into_iter()
            .fold(0, |sum, value| sum ^ checksum_field(value))
    }))
}

fn signature_multi_sequence_concatenate_k3(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    signature_multi_sequence_concatenate_k(
        cell,
        seed,
        [
            Fp251V1::from_u64_mod(7),
            Fp251V1::from_u64_mod(11),
            Fp251V1::from_u64_mod(13),
        ],
    )
}

fn signature_multi_sequence_concatenate_k4(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    signature_multi_sequence_concatenate_k(
        cell,
        seed,
        [
            Fp251V1::from_u64_mod(7),
            Fp251V1::from_u64_mod(11),
            Fp251V1::from_u64_mod(13),
            Fp251V1::from_u64_mod(17),
        ],
    )
}

fn signature_multi_sequence_concatenate_k<const K: usize>(
    cell: &BenchmarkCell,
    seed: u64,
    bases: [Fp251V1; K],
) -> Result<PreparedOperation, String> {
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    let midpoint = items.len() / 2;
    let mut left = MultiEvaluationSequenceSignature::<Fp251V1, _, K>::new(prime_encoder(), bases)
        .map_err(debug_error)?;
    left.push_many(items[..midpoint].iter().map(Vec::as_slice))
        .map_err(debug_error)?;
    let mut right = MultiEvaluationSequenceSignature::<Fp251V1, _, K>::new(prime_encoder(), bases)
        .map_err(debug_error)?;
    right
        .push_many(items[midpoint..].iter().map(Vec::as_slice))
        .map_err(debug_error)?;
    Ok(single_unit(move || {
        std::hint::black_box(&left)
            .concatenate(std::hint::black_box(&right))
            .unwrap()
            .states()
            .iter()
            .fold(0, |sum, value| sum ^ checksum_field(*value))
    }))
}

fn signature_multi_multiset_fragmented_k4(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    let total = cell
        .dataset_size
        .ok_or("fragmented signature requires dataset_size")?;
    let fragments = cell.scale;
    if fragments < 2 || total == 0 || !total.is_multiple_of(fragments) {
        return Err("fragment count must divide a non-empty dataset_size".into());
    }
    let points = [
        Fp251V1::ONE,
        Fp251V1::from_u64_mod(2),
        Fp251V1::from_u64_mod(3),
        Fp251V1::from_u64_mod(4),
    ];
    let items_a = payloads(total, cell.payload_bytes, seed)?;
    let items_b = payloads(total, cell.payload_bytes, seed ^ 0xa5a5_5a5a_1357_2468)?;
    let chunk_size = total / fragments;
    let build = |items: &[Vec<u8>]| {
        items
            .chunks(chunk_size)
            .map(|chunk| {
                let mut signature =
                    MultiEvaluationMultisetSignature::<Fp251V1, _, 4>::new(prime_encoder(), points)
                        .map_err(debug_error)?;
                signature
                    .insert_many(chunk.iter().map(Vec::as_slice))
                    .map_err(debug_error)?;
                Ok(signature)
            })
            .collect::<Result<Vec<_>, String>>()
    };
    let signatures_a = build(&items_a)?;
    let signatures_b = build(&items_b)?;
    let mut alternate = false;
    Ok(single_unit(move || {
        alternate = !alternate;
        let signatures = std::hint::black_box(if alternate {
            &signatures_a
        } else {
            &signatures_b
        });
        let mut combined = signatures[0].clone();
        for signature in &signatures[1..] {
            combined = combined.combine(signature).unwrap();
        }
        combined
            .evaluated_products()
            .into_iter()
            .fold(0, |sum, value| sum ^ checksum_field(value))
    }))
}

fn signature_multi_sequence_fragmented_k4(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    let total = cell
        .dataset_size
        .ok_or("fragmented signature requires dataset_size")?;
    let fragments = cell.scale;
    if fragments < 2 || total == 0 || !total.is_multiple_of(fragments) {
        return Err("fragment count must divide a non-empty dataset_size".into());
    }
    let bases = [
        Fp251V1::from_u64_mod(7),
        Fp251V1::from_u64_mod(11),
        Fp251V1::from_u64_mod(13),
        Fp251V1::from_u64_mod(17),
    ];
    let items_a = payloads(total, cell.payload_bytes, seed)?;
    let items_b = payloads(total, cell.payload_bytes, seed ^ 0xa5a5_5a5a_1357_2468)?;
    let chunk_size = total / fragments;
    let build = |items: &[Vec<u8>]| {
        items
            .chunks(chunk_size)
            .map(|chunk| {
                let mut signature =
                    MultiEvaluationSequenceSignature::<Fp251V1, _, 4>::new(prime_encoder(), bases)
                        .map_err(debug_error)?;
                signature
                    .push_many(chunk.iter().map(Vec::as_slice))
                    .map_err(debug_error)?;
                Ok(signature)
            })
            .collect::<Result<Vec<_>, String>>()
    };
    let signatures_a = build(&items_a)?;
    let signatures_b = build(&items_b)?;
    let mut alternate = false;
    Ok(single_unit(move || {
        alternate = !alternate;
        let signatures = std::hint::black_box(if alternate {
            &signatures_a
        } else {
            &signatures_b
        });
        let mut combined = signatures[0].clone();
        for signature in &signatures[1..] {
            combined = combined.concatenate(signature).unwrap();
        }
        combined
            .states()
            .iter()
            .fold(0, |sum, value| sum ^ checksum_field(*value))
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

#[derive(Clone, Copy)]
enum SummaryBatchMode {
    Sequential,
    Bulk,
    Adaptive,
    Rebuild,
}

fn summary_tree_batch(
    cell: &BenchmarkCell,
    seed: u64,
    mode: SummaryBatchMode,
) -> Result<PreparedOperation, String> {
    let total = cell.dataset_size.unwrap_or(cell.scale);
    let profile = FileChunkProfile::fixed(4_096).map_err(debug_error)?;
    if total < 4_096 || !total.is_multiple_of(4_096) || cell.payload_bytes > 4_096 {
        return Err("bulk summary dataset must contain complete 4096-byte chunks".into());
    }
    let leaf_count = total / 4_096;
    if cell.scale > leaf_count {
        return Err("bulk summary scale exceeds available leaves".into());
    }
    let clustered = cell
        .strategy
        .as_deref()
        .is_some_and(|strategy| strategy.contains("clustered"));
    let leaf_indexes = if clustered {
        (0..cell.scale).collect::<Vec<_>>()
    } else {
        (0..cell.scale)
            .map(|index| index * leaf_count / cell.scale)
            .collect::<Vec<_>>()
    };
    let width = cell.payload_bytes;
    let positions = leaf_indexes
        .into_iter()
        .map(|leaf| leaf * 4_096 + (4_096 - width) / 2)
        .collect::<Vec<_>>();
    let original = deterministic_bytes(total, seed);
    let mut byte = 0xa5_u8;

    match mode {
        SummaryBatchMode::Rebuild => {
            let mut bytes = original;
            Ok(single_unit(move || {
                byte ^= 0xff;
                for position in &positions {
                    bytes[*position..*position + width].fill(byte);
                }
                checksum_field(build_tree(profile, &bytes).unwrap().root().evaluation())
            }))
        }
        selected => {
            let mut tree = build_tree(profile, &original)?;
            let edits_a = positions
                .iter()
                .map(|position| {
                    SummaryRangeEdit::new(*position..*position + width, vec![byte; width])
                })
                .collect::<Vec<_>>();
            let edits_b = positions
                .iter()
                .map(|position| {
                    SummaryRangeEdit::new(*position..*position + width, vec![byte ^ 0xff; width])
                })
                .collect::<Vec<_>>();
            let mut alternate = false;
            Ok(single_unit(move || {
                alternate = !alternate;
                let edits = if alternate { &edits_b } else { &edits_a };
                match selected {
                    SummaryBatchMode::Sequential => {
                        for edit in edits {
                            tree.replace_range(edit.range().clone(), edit.replacement())
                                .unwrap();
                        }
                    }
                    SummaryBatchMode::Bulk => {
                        tree.replace_ranges_with_policy(edits, SummaryEditPolicy::default())
                            .unwrap();
                    }
                    SummaryBatchMode::Adaptive => {
                        tree.replace_ranges_with_policy(
                            edits,
                            SummaryEditPolicy::adaptive(usize::MAX, 3, 4).unwrap(),
                        )
                        .unwrap();
                    }
                    SummaryBatchMode::Rebuild => unreachable!(),
                }
                checksum_field(tree.root().evaluation())
            }))
        }
    }
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
        graph_exact: None,
        run: Box::new(move || {
            let mutations = (0..mutation_count)
                .map(|id| algesum::RowMutation::Update {
                    before: database_row(id as u64, revision + 1),
                    after: database_row(id as u64, revision + 2),
                })
                .collect();
            let transaction =
                algesum::TransactionDelta::new(namespace, &schema, revision, mutations).unwrap();
            database
                .apply_transaction(&transaction, algesum::DatabaseTransactionLimits::default())
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

#[derive(Clone, Copy)]
enum DatabaseBatchMode {
    Bulk,
    Adaptive,
}

fn database_selected_ids(
    cell: &BenchmarkCell,
    schema: &DatabaseSchema,
    row_count: usize,
) -> Result<Vec<u64>, String> {
    if cell.scale > row_count || row_count == 0 {
        return Err("invalid selected database mutation/dataset scale".into());
    }
    let clustered = cell
        .strategy
        .as_deref()
        .is_some_and(|strategy| strategy.contains("clustered"));
    if !clustered {
        return Ok((0..cell.scale as u64).collect());
    }
    let mut partitions = vec![Vec::new(); 16];
    for id in 0..row_count as u64 {
        let key = schema.row_key(&database_row(id, 1)).map_err(debug_error)?;
        let partition =
            (u64::from_le_bytes(key.as_bytes()[..8].try_into().expect("key prefix")) % 16) as usize;
        partitions[partition].push(id);
    }
    let mut selected = Vec::with_capacity(cell.scale);
    for partition in partitions {
        let remaining = cell.scale - selected.len();
        selected.extend(partition.into_iter().take(remaining));
        if selected.len() == cell.scale {
            break;
        }
    }
    Ok(selected)
}

fn database_selected_transaction(
    cell: &BenchmarkCell,
    mode: DatabaseBatchMode,
) -> Result<PreparedOperation, String> {
    let row_count = cell.dataset_size.unwrap_or(cell.scale);
    let schema = database_schema()?;
    let namespace = database_namespace();
    let selected = database_selected_ids(cell, &schema, row_count)?;
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
    Ok(PreparedOperation {
        logical_units_per_action: 1,
        maximum_batch_iterations: Some(1),
        graph_exact: None,
        run: Box::new(move || {
            let mutations = selected
                .iter()
                .map(|id| RowMutation::Update {
                    before: database_row(*id, revision + 1),
                    after: database_row(*id, revision + 2),
                })
                .collect::<Vec<_>>();
            let transaction =
                TransactionDelta::new(namespace, &schema, revision, mutations).unwrap();
            match mode {
                DatabaseBatchMode::Bulk => {
                    database
                        .apply_transaction(&transaction, DatabaseTransactionLimits::default())
                        .unwrap();
                }
                DatabaseBatchMode::Adaptive => {
                    database
                        .apply_transaction_with_policy(
                            &transaction,
                            DatabaseTransactionLimits::default(),
                            DatabaseApplyPolicy::adaptive(usize::MAX, 3, 4).unwrap(),
                            || -> Vec<DatabaseRow> {
                                panic!("partition-adaptive benchmark requested global rows")
                            },
                        )
                        .unwrap();
                }
            }
            revision += 1;
            checksum_field(database.summary().unwrap().evaluation())
        }),
    })
}

fn database_selected_rebuild(cell: &BenchmarkCell) -> Result<PreparedOperation, String> {
    let row_count = cell.dataset_size.unwrap_or(cell.scale);
    let schema = database_schema()?;
    let namespace = database_namespace();
    let selected = database_selected_ids(cell, &schema, row_count)?;
    let rows = (0..row_count)
        .map(|id| database_row(id as u64, 1))
        .collect::<Vec<_>>();
    let mut version = 1_u64;
    Ok(single_unit(move || {
        version += 1;
        let mut candidate = rows.clone();
        for id in &selected {
            candidate[*id as usize] = database_row(*id, version);
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
    let graph: &'static IncidenceGraph = Box::leak(Box::new(match cell.strategy.as_deref() {
        Some("regular-8") => regular_graph(cell.scale.max(8), 8, false)?,
        Some("regular-32") => regular_graph(cell.scale.max(32), 32, false)?,
        Some("mesh") => mesh_graph(cell.scale.max(16))?,
        Some("star") => star_graph(cell.scale.max(4))?,
        _ => sparse_cycle(cell.scale.max(4))?,
    }));
    let labeler =
        FastGraphLabeler::<Fp251V1, _, 2>::new(prime_encoder(), algesum::RefinementProfile::fast())
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
    let graph = match cell.strategy.as_deref() {
        Some("symmetric-cycle" | "symmetric-cycle-budget-1") => {
            sparse_cycle(cell.scale.clamp(3, 16))?
        }
        _ => distinct_path(cell.scale.clamp(2, 16))?,
    };
    let schema = GraphSchemaId::derive(b"publication-exact-v1");
    let canonizer = Microcanon::new(schema);
    let node_budget = if cell.strategy.as_deref() == Some("symmetric-cycle-budget-1") {
        1
    } else {
        1_000_000
    };
    let budget = CanonicalSearchBudget::new(node_budget);
    let probe = canonizer
        .canonicalize(&graph, budget)
        .map_err(debug_error)?;
    let report = probe.report();
    let telemetry = GraphExactTelemetry {
        outcome: if matches!(&probe, MicrocanonOutcome::Exact { .. }) {
            GraphExactOutcome::Exact
        } else {
            GraphExactOutcome::Inconclusive
        },
        node_budget,
        explored_nodes: report.explored_nodes(),
        leaf_count: report.leaf_count(),
        maximum_depth: report.maximum_depth(),
        path: match report.path() {
            MicrocanonPath::ExactRefinementDiscrete => GraphExactPath::ExactRefinementDiscrete,
            MicrocanonPath::WeakComponentDecomposition => {
                GraphExactPath::WeakComponentDecomposition
            }
            MicrocanonPath::IndividualizationRefinement => {
                GraphExactPath::IndividualizationRefinement
            }
        },
        exhausted_limit: report.exhausted_limit().map(|limit| match limit {
            CanonicalBudgetLimit::SearchNodes => GraphExactLimit::SearchNodes,
            CanonicalBudgetLimit::RetainedStateCells => GraphExactLimit::RetainedStateCells,
            CanonicalBudgetLimit::RetainedBytes => GraphExactLimit::RetainedBytes,
            CanonicalBudgetLimit::SearchDepth => GraphExactLimit::SearchDepth,
            CanonicalBudgetLimit::ElapsedTime => GraphExactLimit::ElapsedTime,
        }),
    };
    let mut prepared = scaled(graph.vertex_count(), move || {
        match canonizer.canonicalize(&graph, budget).unwrap() {
            MicrocanonOutcome::Exact { form, .. } => checksum_bytes(form.bytes()),
            MicrocanonOutcome::Inconclusive { report } => {
                0x494e_434f_4e43_4c55 ^ report.explored_nodes()
            }
        }
    });
    prepared.graph_exact = Some(telemetry);
    Ok(prepared)
}

fn graph_full_label_reanalysis(cell: &BenchmarkCell) -> Result<PreparedOperation, String> {
    let vertices = cell.scale.max(8);
    let base = regular_graph(vertices, 8, false)?;
    let changed = regular_graph(vertices, 8, true)?;
    let labeler =
        FastGraphLabeler::<Fp251V1, _, 2>::new(prime_encoder(), algesum::RefinementProfile::fast())
            .map_err(debug_error)?;
    let mut toggle = false;
    Ok(single_unit(move || {
        toggle = !toggle;
        let graph = if toggle {
            changed.clone()
        } else {
            base.clone()
        };
        checksum_field(labeler.analyze(&graph).unwrap().signature().lanes()[0])
    }))
}

fn graph_incremental_label_update(cell: &BenchmarkCell) -> Result<PreparedOperation, String> {
    let vertices = cell.scale.max(8);
    let base = regular_graph(vertices, 8, false)?;
    let changed = regular_graph(vertices, 8, true)?;
    let labeler =
        FastGraphLabeler::<Fp251V1, _, 2>::new(prime_encoder(), algesum::RefinementProfile::fast())
            .map_err(debug_error)?;
    let mut state = labeler
        .incremental_state(base.clone())
        .map_err(debug_error)?;
    let mut workspace = IncrementalGraphWorkspace::new();
    workspace
        .reserve_for(vertices, base.incidence_count(), 4)
        .map_err(debug_error)?;
    let mut toggle = false;
    Ok(single_unit(move || {
        toggle = !toggle;
        let graph = if toggle {
            changed.clone()
        } else {
            base.clone()
        };
        labeler
            .update_incremental(&mut state, graph, &mut workspace)
            .unwrap()
            .recomputed_vertex_rounds() as u64
    }))
}

fn graph_full_topology_reanalysis(cell: &BenchmarkCell) -> Result<PreparedOperation, String> {
    let vertices = cell.scale.max(16);
    let base = regular_graph_variant(vertices, 8, false, false)?;
    let changed = regular_graph_variant(vertices, 8, false, true)?;
    let labeler =
        FastGraphLabeler::<Fp251V1, _, 2>::new(prime_encoder(), algesum::RefinementProfile::fast())
            .map_err(debug_error)?;
    let mut toggle = false;
    Ok(single_unit(move || {
        toggle = !toggle;
        let graph = if toggle { &changed } else { &base };
        checksum_field(labeler.analyze(graph).unwrap().signature().lanes()[0])
    }))
}

fn graph_incremental_topology_update(cell: &BenchmarkCell) -> Result<PreparedOperation, String> {
    let vertices = cell.scale.max(16);
    let base = regular_graph_variant(vertices, 8, false, false)?;
    let changed = regular_graph_variant(vertices, 8, false, true)?;
    let labeler =
        FastGraphLabeler::<Fp251V1, _, 2>::new(prime_encoder(), algesum::RefinementProfile::fast())
            .map_err(debug_error)?;
    let mut state = labeler
        .incremental_state(base.clone())
        .map_err(debug_error)?;
    let mut workspace = IncrementalGraphWorkspace::new();
    workspace
        .reserve_for(
            vertices,
            base.incidence_count().max(changed.incidence_count()),
            4,
        )
        .map_err(debug_error)?;
    let mut toggle = false;
    Ok(single_unit(move || {
        toggle = !toggle;
        let graph = if toggle {
            changed.clone()
        } else {
            base.clone()
        };
        labeler
            .update_incremental(&mut state, graph, &mut workspace)
            .unwrap()
            .recomputed_vertex_rounds() as u64
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
            algesum::GraphDagResolveOutcome::Reused { node, .. } => node.as_u64(),
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
        graph_exact: None,
        run: Box::new(action),
    }
}

fn scaled(scale: usize, action: impl FnMut() -> u64 + 'static) -> PreparedOperation {
    PreparedOperation {
        logical_units_per_action: scale.max(1) as u64,
        maximum_batch_iterations: None,
        graph_exact: None,
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

fn regular_graph(
    vertices: usize,
    degree: usize,
    changed_label: bool,
) -> Result<IncidenceGraph, String> {
    regular_graph_variant(vertices, degree, changed_label, false)
}

fn regular_graph_variant(
    vertices: usize,
    degree: usize,
    changed_label: bool,
    topology_edit: bool,
) -> Result<IncidenceGraph, String> {
    let mut builder = IncidenceGraphBuilder::new();
    let ids = (0..vertices)
        .map(|index| {
            let label = if changed_label && index == vertices / 2 {
                b"changed".to_vec()
            } else {
                (index % 17).to_le_bytes().to_vec()
            };
            builder.add_vertex(label)
        })
        .collect::<Vec<_>>();
    for source in 0..vertices {
        for step in 1..=degree / 2 {
            builder
                .add_undirected_relation(
                    ids[source],
                    ids[(source + step) % vertices],
                    b"edge",
                    b"regular",
                    1,
                )
                .map_err(debug_error)?;
        }
    }
    if topology_edit {
        builder
            .add_undirected_relation(ids[0], ids[vertices / 2], b"edge", b"topology-edit", 1)
            .map_err(debug_error)?;
    }
    builder.build().map_err(debug_error)
}

fn mesh_graph(vertices: usize) -> Result<IncidenceGraph, String> {
    let width = (vertices as f64).sqrt().floor() as usize;
    let width = width.max(2);
    let height = vertices.div_ceil(width);
    let mut builder = IncidenceGraphBuilder::new();
    let ids = (0..vertices)
        .map(|_| builder.add_vertex(Vec::new()))
        .collect::<Vec<_>>();
    for index in 0..vertices {
        let column = index % width;
        let row = index / width;
        if column + 1 < width && index + 1 < vertices {
            builder
                .add_undirected_relation(ids[index], ids[index + 1], b"edge", b"mesh-x", 1)
                .map_err(debug_error)?;
        }
        if row + 1 < height && index + width < vertices {
            builder
                .add_undirected_relation(ids[index], ids[index + width], b"edge", b"mesh-y", 1)
                .map_err(debug_error)?;
        }
    }
    builder.build().map_err(debug_error)
}

fn star_graph(vertices: usize) -> Result<IncidenceGraph, String> {
    let mut builder = IncidenceGraphBuilder::new();
    let ids = (0..vertices)
        .map(|_| builder.add_vertex(Vec::new()))
        .collect::<Vec<_>>();
    for leaf in 1..vertices {
        builder
            .add_undirected_relation(ids[0], ids[leaf], b"edge", b"star", 1)
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
            let scale = if operation.contains("summary-tree.") && operation.contains("batch") {
                1
            } else if operation.contains("graph.exact") || operation.contains("dag") {
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
            if operation.contains("fragmented-") {
                benchmark_cell.dataset_size = Some(benchmark_cell.scale);
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

    #[test]
    fn c3_field_primitive_actions_are_repeatable() {
        for operation in SUPPORTED_OPERATIONS.iter().copied().filter(|operation| {
            operation.starts_with("field.")
                && (operation.ends_with("add-total")
                    || operation.ends_with("mul-total")
                    || operation.ends_with("square-total")
                    || operation.ends_with("invert-total")
                    || operation.ends_with("canonical-roundtrip-total"))
        }) {
            let benchmark_cell = cell(operation, 17);
            let mut prepared = prepare(&benchmark_cell, 0x1234_5678).unwrap();
            let first = (prepared.run)();
            let second = (prepared.run)();
            assert_eq!(first, second, "stateful C3 primitive workload: {operation}");
        }
    }

    #[test]
    fn exact_graph_operation_exposes_outcome_and_budget() {
        let mut exact = cell("graph.exact", 12);
        exact.strategy = Some("symmetric-cycle-budget-1".into());
        let prepared = prepare(&exact, 7).unwrap();
        let telemetry = prepared.graph_exact.unwrap();
        assert_eq!(telemetry.outcome, GraphExactOutcome::Inconclusive);
        assert_eq!(telemetry.node_budget, 1);
        assert!(telemetry.explored_nodes <= telemetry.node_budget);
        assert_eq!(
            telemetry.exhausted_limit,
            Some(GraphExactLimit::SearchNodes)
        );
        assert_eq!(telemetry.path, GraphExactPath::IndividualizationRefinement);
    }

    #[test]
    fn topology_benchmark_pair_is_differentially_exact() {
        let base = regular_graph_variant(64, 8, false, false).unwrap();
        let changed = regular_graph_variant(64, 8, false, true).unwrap();
        let labeler = FastGraphLabeler::<Fp251V1, _, 2>::new(
            prime_encoder(),
            algesum::RefinementProfile::fast(),
        )
        .unwrap();
        let mut state = labeler.incremental_state(base.clone()).unwrap();
        let mut workspace = IncrementalGraphWorkspace::new();
        labeler
            .update_incremental(&mut state, changed.clone(), &mut workspace)
            .unwrap();
        assert_eq!(
            state.analysis().to_owned(),
            labeler.analyze(&changed).unwrap()
        );
        labeler
            .update_incremental(&mut state, base.clone(), &mut workspace)
            .unwrap();
        assert_eq!(state.analysis().to_owned(), labeler.analyze(&base).unwrap());
    }
}
