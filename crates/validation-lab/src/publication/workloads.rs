use std::{
    fs,
    mem::MaybeUninit,
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

use algesum::{
    AdditiveDelta, AdditiveSignature, ApplicationNamespace, BidirectionalSequenceSignature,
    BinaryPolynomialEncoder, BoundedSetReconciler, CanonicalBudgetLimit, CanonicalGraphDag,
    CanonicalGraphDagLimits, CanonicalSearchBudget, CompactSignature, DatabaseApplyPolicy,
    DatabaseColumn, DatabaseColumnType, DatabaseRow, DatabaseSchema, DatabaseTransactionLimits,
    DatabaseTransactionLog, DatabaseValue, DeltaJournal, DeltaJournalLimits,
    DynamicAdditiveSignature, DynamicMultiEvaluationMultisetSignature,
    DynamicMultiEvaluationSequenceSignature, FastGraphLabeler, FileChunkProfile,
    GaloisSignature256, GraphDelta, GraphDeltaPolicy, GraphExecution, GraphSchemaId,
    GraphWorkspace, HomomorphicSummaryTree, IncidenceGraph, IncidenceGraphBuilder,
    IncrementalGraphWorkspace, LegacyAffineEncoderV1, Microcanon, MicrocanonOutcome,
    MicrocanonPath, MultiEvaluationMultisetSignature, MultiEvaluationSequenceSignature,
    MultisetAggregator, MultisetDelta, MultisetSignature, PartitionedDatabase, PrimeIntegerEncoder,
    ReconciliationLimits, RevisionedSignature, RowMutation, SequenceAppend, SequenceSignature,
    SequenceTrim, SummaryEditPolicy, SummaryRangeEdit, SummaryTreeLimits, TopoHasher,
    TrackedMultiset, TrackedSequence, TrackedSnapshotLimits, TransactionDelta, VertexId,
};
use microfield::{
    fill_fixed_base_powers,
    generator::{BinaryFieldFactory, PrimeFieldFactory, PrimeFieldManifest},
    pack_into_storage, required_mask_words, required_packed_bytes, BatchInvertPlan,
    BatchInvertWorkspace, BinaryPolynomialField, BitMaskViewMut, CanonicalEncoding,
    CoefficientLayout, DynBatch, DynField, Engine, Field, Fp251V1, Fp256GenericV1,
    FpGoldilocks64V1, Gf2_128V1, Gf2_256AltV1, Gf2_256HhV1, Invert, ManyPointsHornerPlan,
    ManyPolynomialsHornerPlan, PackedBatch, PrimeField, ProductScanPlan, ScanDirection, ScanMode,
    Square,
};

use super::model::{
    BenchmarkCell, GraphExactLimit, GraphExactOutcome, GraphExactPath, GraphExactTelemetry,
};

type BinaryEncoder = BinaryPolynomialEncoder;
type Database = PartitionedDatabase<Gf2_128V1, BinaryEncoder>;
type SummaryTree = HomomorphicSummaryTree<Gf2_128V1, BinaryEncoder>;
type PrimeAdditive = AdditiveSignature<Fp251V1, PrimeIntegerEncoder>;
type PrimeAdditiveJournal = DeltaJournal<AdditiveDelta<Fp251V1, PrimeIntegerEncoder>>;
type AdditiveJournalFixture = (ApplicationNamespace, PrimeAdditive, PrimeAdditiveJournal);

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
    "field.batch-portable-total",
    "field.packed-owned-total",
    "field.packed-view-total",
    "algorithm.horner-total",
    "algorithm.scan-total",
    "algorithm.batch-invert-total",
    "algorithm.powers-total",
    "algorithm.mask-total",
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
    "signature.base-remove-total",
    "signature.base-wire-total",
    "signature.base-restore-total",
    "signature.base-input-pattern-total",
    "signature.multi-k8-total",
    "signature.multi-k16-total",
    "signature.runtime-total",
    "signature.multi-wire-restore-total",
    "signature.tracked-update-total",
    "signature.compact-snapshot-total",
    "signature.tracked-snapshot-total",
    "delta.per-law-total",
    "journal.append-replay-total",
    "journal.failure-path",
    "delta.additive.end-to-end",
    "summary-tree.rebuild",
    "summary-tree.local-edit-total",
    "summary-tree.rebuild-edit-total",
    "summary-tree.sequential-batch-total",
    "summary-tree.bulk-batch-total",
    "summary-tree.adaptive-batch-total",
    "summary-tree.rebuild-batch-total",
    "file.chunk-build-total",
    "file.chunk-wire-total",
    "summary-tree.checkpoint-total",
    "summary-tree.restore-total",
    "summary-tree.grow-shrink-total",
    "database.rebuild",
    "database.transaction-end-to-end",
    "database.table-rebuild-total",
    "database.bulk-transaction-total",
    "database.adaptive-transaction-total",
    "database.selected-rebuild-total",
    "reconciliation.decode",
    "reconciliation.sketch-total",
    "reconciliation.combine-total",
    "reconciliation.wire-total",
    "reconciliation.limit-path",
    "database.row-wire-total",
    "database.mixed-mutation-total",
    "database.checkpoint-replay-total",
    "database.schema-payload-total",
    "database.route-telemetry",
    "graph.fast-prepared",
    "graph.exact",
    "graph.dag-reuse",
    "graph.full-label-reanalysis-total",
    "graph.incremental-label-update-total",
    "graph.full-topology-reanalysis-total",
    "graph.incremental-topology-update-total",
    "graph.build-prepare-total",
    "graph.channel-total",
    "graph.parallel-total",
    "graph.incremental-batch-total",
    "graph.memory-telemetry",
    "graph.exact-budget-matrix",
    "graph.exact-family-matrix",
    "graph.dag-persist-restore-total",
    "graph.dag-incremental-total",
    "tool.binary-manifest.parse",
    "tool.binary-manifest.generate",
    "field.runtime-lifecycle-total",
    "field.runtime-operation-total",
    "tool.prime-manifest-total",
    "tool.artifact-compile-consumer-total",
    "wire.family-roundtrip-total",
    "wire.failure-matrix",
    "package.consumer-total",
    "legacy.facade-semantic",
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
        "field.batch-portable-total" => field_batch_portable_total(cell, seed),
        "field.packed-owned-total" => field_packed_owned_total(cell, seed),
        "field.packed-view-total" => field_packed_view_total(cell, seed),
        "algorithm.horner-total" => algorithm_horner_total(cell, seed),
        "algorithm.scan-total" => algorithm_scan_total(cell, seed),
        "algorithm.batch-invert-total" => algorithm_batch_invert_total(cell, seed),
        "algorithm.powers-total" => algorithm_powers_total(cell, seed),
        "algorithm.mask-total" => algorithm_mask_total(cell, seed),
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
        "signature.base-remove-total" => signature_base_remove_total(cell, seed),
        "signature.base-wire-total" => signature_base_wire_total(cell, seed),
        "signature.base-restore-total" => signature_base_restore_total(cell, seed),
        "signature.base-input-pattern-total" => signature_base_input_pattern_total(cell, seed),
        "signature.multi-k8-total" => signature_multi_k8_total(cell, seed),
        "signature.multi-k16-total" => signature_multi_k16_total(cell, seed),
        "signature.runtime-total" => signature_runtime_total(cell, seed),
        "signature.multi-wire-restore-total" => signature_multi_wire_restore_total(cell, seed),
        "signature.tracked-update-total" => signature_tracked_update_total(cell, seed),
        "signature.compact-snapshot-total" => signature_compact_snapshot_total(cell, seed),
        "signature.tracked-snapshot-total" => signature_tracked_snapshot_total(cell, seed),
        "delta.per-law-total" => delta_per_law_total(cell, seed),
        "journal.append-replay-total" => journal_append_replay_total(cell, seed),
        "journal.failure-path" => journal_failure_path(cell, seed),
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
        "file.chunk-build-total" => file_chunk_build_total(cell, seed),
        "file.chunk-wire-total" => file_chunk_wire_total(cell, seed),
        "summary-tree.checkpoint-total" => summary_tree_checkpoint_total(cell, seed),
        "summary-tree.restore-total" => summary_tree_restore_total(cell, seed),
        "summary-tree.grow-shrink-total" => summary_tree_grow_shrink_total(cell, seed),
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
        "reconciliation.sketch-total" => reconciliation_sketch_total(cell),
        "reconciliation.combine-total" => reconciliation_combine_total(cell),
        "reconciliation.wire-total" => reconciliation_wire_total(cell),
        "reconciliation.limit-path" => reconciliation_limit_path(cell),
        "database.row-wire-total" => database_row_wire_total(cell),
        "database.mixed-mutation-total" => database_mixed_mutation_total(cell),
        "database.checkpoint-replay-total" => database_checkpoint_replay_total(cell),
        "database.schema-payload-total" => database_schema_payload_total(cell),
        "database.route-telemetry" => database_route_telemetry(cell),
        "graph.fast-prepared" => graph_fast(cell),
        "graph.exact" => graph_exact(cell),
        "graph.dag-reuse" => graph_dag_reuse(cell),
        "graph.full-label-reanalysis-total" => graph_full_label_reanalysis(cell),
        "graph.incremental-label-update-total" => graph_incremental_label_update(cell),
        "graph.full-topology-reanalysis-total" => graph_full_topology_reanalysis(cell),
        "graph.incremental-topology-update-total" => graph_incremental_topology_update(cell),
        "graph.build-prepare-total" => graph_build_prepare_total(cell),
        "graph.channel-total" => graph_channel_total(cell),
        "graph.parallel-total" => graph_parallel_total(cell),
        "graph.incremental-batch-total" => graph_incremental_batch_total(cell),
        "graph.memory-telemetry" => graph_memory_telemetry(cell),
        "graph.exact-budget-matrix" => graph_exact(cell),
        "graph.exact-family-matrix" => graph_exact(cell),
        "graph.dag-persist-restore-total" => graph_dag_persist_restore_total(cell),
        "graph.dag-incremental-total" => graph_dag_incremental_total(cell),
        "tool.binary-manifest.parse" => tool_binary_manifest(false),
        "tool.binary-manifest.generate" => tool_binary_manifest(true),
        "field.runtime-lifecycle-total" => field_runtime_lifecycle_total(cell, seed),
        "field.runtime-operation-total" => field_runtime_operation_total(cell, seed),
        "tool.prime-manifest-total" => tool_prime_manifest_total(cell),
        "tool.artifact-compile-consumer-total" => tool_artifact_compile_consumer_total(),
        "wire.family-roundtrip-total" => wire_family_roundtrip_total(cell, seed),
        "wire.failure-matrix" => wire_failure_matrix(cell, seed),
        "package.consumer-total" => tool_artifact_compile_consumer_total(),
        "legacy.facade-semantic" => legacy_facade_semantic(cell, seed),
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

fn field_batch_portable_total(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    let scale = cell.scale;
    let lhs = gf2_256_values(scale, seed, false)?;
    let rhs = gf2_256_values(scale, seed.rotate_left(19), false)?;
    let mut output = vec![Gf2_256HhV1::ZERO; scale];
    let engine = Engine::<Gf2_256HhV1>::portable();
    Ok(single_unit(move || {
        engine.mul_into(&mut output, &lhs, &rhs).unwrap();
        checksum_fields(&output)
    }))
}

fn field_packed_owned_total(cell: &BenchmarkCell, seed: u64) -> Result<PreparedOperation, String> {
    let scale = cell.scale;
    let lhs = gf2_256_values(scale, seed, false)?;
    let rhs = gf2_256_values(scale, seed.rotate_left(19), false)?;
    let engine = Engine::<Gf2_256HhV1>::builder()
        .expected_batch(scale)
        .detect()
        .map_err(debug_error)?;
    let mut packed_lhs = PackedBatch::from_aos(&engine, &lhs).map_err(debug_error)?;
    let mut packed_rhs = PackedBatch::from_aos(&engine, &rhs).map_err(debug_error)?;
    let mut packed_output = PackedBatch::new(&engine, scale).map_err(debug_error)?;
    let mut output = vec![Gf2_256HhV1::ZERO; scale];
    Ok(single_unit(move || {
        packed_lhs.pack_from(&lhs).unwrap();
        packed_rhs.pack_from(&rhs).unwrap();
        engine
            .mul_packed_into(&mut packed_output, &packed_lhs, &packed_rhs)
            .unwrap();
        packed_output.unpack_into(&mut output).unwrap();
        checksum_fields(&output)
    }))
}

fn field_packed_view_total(cell: &BenchmarkCell, seed: u64) -> Result<PreparedOperation, String> {
    let scale = cell.scale;
    let lhs = gf2_256_values(scale, seed, false)?;
    let rhs = gf2_256_values(scale, seed.rotate_left(19), false)?;
    let zeros = vec![Gf2_256HhV1::ZERO; scale];
    let engine = Engine::<Gf2_256HhV1>::builder()
        .expected_batch(scale)
        .detect()
        .map_err(debug_error)?;
    let plan = engine.packing_plan(scale).map_err(debug_error)?;
    let storage_bytes = required_packed_bytes(&plan).map_err(debug_error)?;
    let mut lhs_storage = vec![MaybeUninit::uninit(); storage_bytes];
    let mut rhs_storage = vec![MaybeUninit::uninit(); storage_bytes];
    let mut out_storage = vec![MaybeUninit::uninit(); storage_bytes];
    let mut output = zeros.clone();
    Ok(single_unit(move || {
        let packed_lhs = pack_into_storage(&engine, &mut lhs_storage, &lhs).unwrap();
        let packed_rhs = pack_into_storage(&engine, &mut rhs_storage, &rhs).unwrap();
        let mut packed_output = pack_into_storage(&engine, &mut out_storage, &zeros).unwrap();
        engine
            .mul_packed_view_into(
                &mut packed_output,
                &packed_lhs.as_view(),
                &packed_rhs.as_view(),
            )
            .unwrap();
        packed_output.unpack_into(&mut output).unwrap();
        checksum_fields(&output)
    }))
}

fn algorithm_horner_total(cell: &BenchmarkCell, seed: u64) -> Result<PreparedOperation, String> {
    let scale = cell.scale;
    let coefficient_count = cell.payload_bytes.max(1);
    let engine = Engine::<Gf2_256HhV1>::portable();
    let mut output = vec![Gf2_256HhV1::ZERO; scale];
    if cell
        .strategy
        .as_deref()
        .is_some_and(|strategy| strategy.contains("many-polynomials"))
    {
        let coefficients = gf2_256_values(
            scale
                .checked_mul(coefficient_count)
                .ok_or("Horner matrix size overflow")?,
            seed,
            false,
        )?;
        let point = gf2_256_values(1, seed.rotate_left(9), false)?[0];
        let plan = ManyPolynomialsHornerPlan::new(
            &engine,
            scale,
            coefficient_count,
            CoefficientLayout::PolynomialMajor,
        )
        .map_err(debug_error)?;
        Ok(single_unit(move || {
            plan.execute(&engine, &mut output, &coefficients, point)
                .unwrap();
            checksum_fields(&output)
        }))
    } else {
        let coefficients = gf2_256_values(coefficient_count, seed, false)?;
        let points = gf2_256_values(scale, seed.rotate_left(9), false)?;
        let plan =
            ManyPointsHornerPlan::new(&engine, scale, coefficient_count).map_err(debug_error)?;
        Ok(single_unit(move || {
            plan.execute(&engine, &mut output, &coefficients, &points)
                .unwrap();
            checksum_fields(&output)
        }))
    }
}

fn algorithm_scan_total(cell: &BenchmarkCell, seed: u64) -> Result<PreparedOperation, String> {
    let values = gf2_256_values(cell.scale, seed, false)?;
    let mut output = vec![Gf2_256HhV1::ZERO; cell.scale];
    let engine = Engine::<Gf2_256HhV1>::portable();
    let strategy = cell.strategy.as_deref().unwrap_or("prefix-inclusive");
    let direction = if strategy.contains("suffix") {
        ScanDirection::Suffix
    } else {
        ScanDirection::Prefix
    };
    let mode = if strategy.contains("exclusive") {
        ScanMode::Exclusive
    } else {
        ScanMode::Inclusive
    };
    let plan = ProductScanPlan::new(&engine, cell.scale, direction, mode);
    Ok(single_unit(move || {
        plan.execute(&engine, &mut output, &values).unwrap();
        checksum_fields(&output)
    }))
}

fn algorithm_batch_invert_total(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    let values = gf2_256_values(cell.scale, seed, true)?;
    let mut output = vec![Gf2_256HhV1::ZERO; cell.scale];
    let mut prefixes = vec![Gf2_256HhV1::ZERO; cell.scale];
    let mut mask_words = vec![0_u64; required_mask_words(cell.scale).map_err(debug_error)?];
    let engine = Engine::<Gf2_256HhV1>::portable();
    let plan = BatchInvertPlan::new(&engine, cell.scale).map_err(debug_error)?;
    Ok(single_unit(move || {
        let mut workspace = BatchInvertWorkspace::new(&mut prefixes);
        let mut mask = BitMaskViewMut::new(&mut mask_words, values.len()).unwrap();
        plan.execute(&engine, &mut output, &values, &mut mask, &mut workspace)
            .unwrap();
        checksum_fields(&output) ^ mask.count_ones() as u64
    }))
}

fn algorithm_powers_total(cell: &BenchmarkCell, seed: u64) -> Result<PreparedOperation, String> {
    let base = gf2_256_values(1, seed, false)?[0];
    let mut output = vec![Gf2_256HhV1::ZERO; cell.scale];
    Ok(single_unit(move || {
        fill_fixed_base_powers(&mut output, base);
        checksum_fields(&output)
    }))
}

fn algorithm_mask_total(cell: &BenchmarkCell, seed: u64) -> Result<PreparedOperation, String> {
    let scale = cell.scale;
    let mut words = vec![seed | 1; required_mask_words(scale).map_err(debug_error)?];
    Ok(single_unit(move || {
        for (index, word) in words.iter_mut().enumerate() {
            *word = seed.rotate_left(index as u32) ^ index as u64;
        }
        let mut mask = BitMaskViewMut::new(&mut words, scale).unwrap();
        let count = mask.count_ones() as u64;
        mask.clear();
        count ^ mask.len() as u64
    }))
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

fn signature_base_remove_total(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    if signature_law(cell) == "sequence" {
        let mut baseline =
            TrackedSequence::<Fp251V1, _>::new(prime_encoder(), Fp251V1::from_u64_mod(7))
                .map_err(debug_error)?;
        for item in &items {
            baseline.push(item).map_err(debug_error)?;
        }
        return Ok(single_unit(move || {
            let mut candidate = baseline.clone();
            while !candidate.signature().is_empty() {
                candidate.pop().unwrap();
            }
            checksum_field(candidate.signature().state())
        }));
    }
    let mut baseline = TrackedMultiset::<Fp251V1, _>::new(prime_encoder(), Fp251V1::ONE);
    for item in &items {
        baseline.insert(item).map_err(debug_error)?;
    }
    Ok(single_unit(move || {
        let mut candidate = baseline.clone();
        for item in &items {
            candidate.remove(item).unwrap();
        }
        checksum_field(candidate.signature().evaluated_product())
    }))
}

fn signature_base_wire_total(cell: &BenchmarkCell, seed: u64) -> Result<PreparedOperation, String> {
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    match signature_law(cell) {
        "sequence" => {
            let base = Fp251V1::from_u64_mod(7);
            let mut signature =
                SequenceSignature::<Fp251V1, _>::new(prime_encoder(), base).map_err(debug_error)?;
            signature
                .push_many(items.iter().map(Vec::as_slice))
                .map_err(debug_error)?;
            Ok(single_unit(move || {
                let bytes = signature.to_canonical_bytes();
                let restored =
                    SequenceSignature::from_canonical_bytes(prime_encoder(), base, &bytes).unwrap();
                checksum_field(restored.state()) ^ checksum_bytes(&bytes)
            }))
        }
        "bidirectional" => {
            let base = Fp251V1::from_u64_mod(7);
            let mut signature =
                BidirectionalSequenceSignature::<Fp251V1, _>::new(prime_encoder(), base)
                    .map_err(debug_error)?;
            signature
                .push_many(items.iter().map(Vec::as_slice))
                .map_err(debug_error)?;
            Ok(single_unit(move || {
                let bytes = signature.to_canonical_bytes();
                let restored = BidirectionalSequenceSignature::from_canonical_bytes(
                    prime_encoder(),
                    base,
                    &bytes,
                )
                .unwrap();
                checksum_field(restored.forward_state()) ^ checksum_bytes(&bytes)
            }))
        }
        "multiset" => {
            let offset = Fp251V1::ONE;
            let mut signature = MultisetSignature::<Fp251V1, _>::new(prime_encoder(), offset);
            signature
                .insert_many(items.iter().map(Vec::as_slice))
                .map_err(debug_error)?;
            Ok(single_unit(move || {
                let bytes = signature.to_canonical_bytes();
                let restored =
                    MultisetSignature::from_canonical_bytes(prime_encoder(), offset, &bytes)
                        .unwrap();
                checksum_field(restored.evaluated_product()) ^ checksum_bytes(&bytes)
            }))
        }
        _ => {
            let mut signature = AdditiveSignature::<Fp251V1, _>::new(prime_encoder());
            signature
                .absorb_many(items.iter().map(Vec::as_slice))
                .map_err(debug_error)?;
            Ok(single_unit(move || {
                let bytes = signature.to_canonical_bytes();
                let restored =
                    AdditiveSignature::<Fp251V1, _>::from_canonical_bytes(prime_encoder(), &bytes)
                        .unwrap();
                checksum_field(restored.state()) ^ checksum_bytes(&bytes)
            }))
        }
    }
}

fn signature_base_restore_total(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    match signature_law(cell) {
        "sequence" => {
            let base = Fp251V1::from_u64_mod(7);
            let mut signature =
                SequenceSignature::<Fp251V1, _>::new(prime_encoder(), base).map_err(debug_error)?;
            signature
                .push_many(items.iter().map(Vec::as_slice))
                .map_err(debug_error)?;
            let bytes = signature.to_canonical_bytes();
            Ok(single_unit(move || {
                checksum_field(
                    SequenceSignature::from_canonical_bytes(prime_encoder(), base, &bytes)
                        .unwrap()
                        .state(),
                )
            }))
        }
        "bidirectional" => {
            let base = Fp251V1::from_u64_mod(7);
            let mut signature =
                BidirectionalSequenceSignature::<Fp251V1, _>::new(prime_encoder(), base)
                    .map_err(debug_error)?;
            signature
                .push_many(items.iter().map(Vec::as_slice))
                .map_err(debug_error)?;
            let bytes = signature.to_canonical_bytes();
            Ok(single_unit(move || {
                let restored = BidirectionalSequenceSignature::from_canonical_bytes(
                    prime_encoder(),
                    base,
                    &bytes,
                )
                .unwrap();
                checksum_field(restored.forward_state()) ^ checksum_field(restored.reverse_state())
            }))
        }
        "multiset" => {
            let offset = Fp251V1::ONE;
            let mut signature = MultisetSignature::<Fp251V1, _>::new(prime_encoder(), offset);
            signature
                .insert_many(items.iter().map(Vec::as_slice))
                .map_err(debug_error)?;
            let bytes = signature.to_canonical_bytes();
            Ok(single_unit(move || {
                checksum_field(
                    MultisetSignature::from_canonical_bytes(prime_encoder(), offset, &bytes)
                        .unwrap()
                        .evaluated_product(),
                )
            }))
        }
        _ => {
            let mut signature = AdditiveSignature::<Fp251V1, _>::new(prime_encoder());
            signature
                .absorb_many(items.iter().map(Vec::as_slice))
                .map_err(debug_error)?;
            let bytes = signature.to_canonical_bytes();
            Ok(single_unit(move || {
                checksum_field(
                    AdditiveSignature::<Fp251V1, _>::from_canonical_bytes(prime_encoder(), &bytes)
                        .unwrap()
                        .state(),
                )
            }))
        }
    }
}

fn signature_base_input_pattern_total(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    let pattern = cell.strategy.as_deref().unwrap_or("distinct");
    let mut items = payloads(cell.scale, cell.payload_bytes, seed)?;
    if pattern.contains("repeated") && !items.is_empty() {
        let first = items[0].clone();
        items.fill(first);
    } else if pattern.contains("empty") {
        items.iter_mut().for_each(Vec::clear);
    } else if pattern.contains("skewed") {
        for (index, item) in items.iter_mut().enumerate() {
            item.truncate(if index % 16 == 0 {
                cell.payload_bytes
            } else {
                1
            });
        }
    }
    Ok(single_unit(move || {
        let mut signature = AdditiveSignature::<Fp251V1, _>::new(prime_encoder());
        signature
            .absorb_many(items.iter().map(Vec::as_slice))
            .unwrap();
        checksum_field(signature.state())
    }))
}

fn signature_multi_k8_total(cell: &BenchmarkCell, seed: u64) -> Result<PreparedOperation, String> {
    signature_multi_wide_total::<8>(cell, seed)
}

fn signature_multi_k16_total(cell: &BenchmarkCell, seed: u64) -> Result<PreparedOperation, String> {
    signature_multi_wide_total::<16>(cell, seed)
}

fn signature_multi_wide_total<const K: usize>(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    if signature_law(cell) == "sequence" {
        let bases = std::array::from_fn(|index| Fp251V1::from_u64_mod(index as u64 + 2));
        Ok(single_unit(move || {
            let mut signature =
                MultiEvaluationSequenceSignature::<Fp251V1, _, K>::new(prime_encoder(), bases)
                    .unwrap();
            signature
                .push_many(items.iter().map(Vec::as_slice))
                .unwrap();
            checksum_fields(signature.states())
        }))
    } else {
        let offsets = std::array::from_fn(|index| Fp251V1::from_u64_mod(index as u64 + 1));
        Ok(single_unit(move || {
            let mut signature =
                MultiEvaluationMultisetSignature::<Fp251V1, _, K>::new(prime_encoder(), offsets)
                    .unwrap();
            signature
                .insert_many(items.iter().map(Vec::as_slice))
                .unwrap();
            checksum_fields(&signature.evaluated_products())
        }))
    }
}

fn signature_runtime_total(cell: &BenchmarkCell, seed: u64) -> Result<PreparedOperation, String> {
    let field = DynField::builder("c3_signature_runtime_fp251")
        .prime("251")
        .build()
        .map_err(debug_error)?;
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    match signature_law(cell) {
        "sequence" => {
            let bases = (2_u64..6)
                .map(|value| field.reduce_bytes_mod_order(&value.to_le_bytes()))
                .collect::<Vec<_>>();
            Ok(single_unit(move || {
                let mut signature = DynamicMultiEvaluationSequenceSignature::new(
                    field.clone(),
                    prime_encoder(),
                    bases.clone(),
                )
                .unwrap();
                signature
                    .push_many(items.iter().map(Vec::as_slice))
                    .unwrap();
                checksum_dyn_fields(&field, signature.states())
            }))
        }
        "multiset" => {
            let offsets = (1_u64..=4)
                .map(|value| field.reduce_bytes_mod_order(&value.to_le_bytes()))
                .collect::<Vec<_>>();
            Ok(single_unit(move || {
                let mut signature = DynamicMultiEvaluationMultisetSignature::new(
                    field.clone(),
                    prime_encoder(),
                    offsets.clone(),
                )
                .unwrap();
                signature
                    .insert_many(items.iter().map(Vec::as_slice))
                    .unwrap();
                checksum_dyn_fields(&field, &signature.evaluated_products())
            }))
        }
        _ => Ok(single_unit(move || {
            let mut signature = DynamicAdditiveSignature::new(field.clone(), prime_encoder());
            signature
                .absorb_many(items.iter().map(Vec::as_slice))
                .unwrap();
            checksum_dyn(&field, signature.state())
        })),
    }
}

fn signature_multi_wire_restore_total(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    if signature_law(cell) == "sequence" {
        let bases = [
            Fp251V1::from_u64_mod(2),
            Fp251V1::from_u64_mod(3),
            Fp251V1::from_u64_mod(4),
            Fp251V1::from_u64_mod(5),
        ];
        let mut signature =
            MultiEvaluationSequenceSignature::<Fp251V1, _, 4>::new(prime_encoder(), bases)
                .map_err(debug_error)?;
        signature
            .push_many(items.iter().map(Vec::as_slice))
            .map_err(debug_error)?;
        let bytes = signature.to_canonical_bytes();
        Ok(single_unit(move || {
            let restored = MultiEvaluationSequenceSignature::from_canonical_bytes(
                prime_encoder(),
                bases,
                &bytes,
            )
            .unwrap();
            checksum_fields(restored.states())
        }))
    } else {
        let offsets = [
            Fp251V1::ONE,
            Fp251V1::from_u64_mod(2),
            Fp251V1::from_u64_mod(3),
            Fp251V1::from_u64_mod(4),
        ];
        let mut signature =
            MultiEvaluationMultisetSignature::<Fp251V1, _, 4>::new(prime_encoder(), offsets)
                .map_err(debug_error)?;
        signature
            .insert_many(items.iter().map(Vec::as_slice))
            .map_err(debug_error)?;
        let bytes = signature.to_canonical_bytes();
        Ok(single_unit(move || {
            checksum_fields(
                &MultiEvaluationMultisetSignature::from_canonical_bytes(
                    prime_encoder(),
                    offsets,
                    &bytes,
                )
                .unwrap()
                .evaluated_products(),
            )
        }))
    }
}

fn signature_tracked_update_total(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    if signature_law(cell) == "sequence" {
        let mut baseline =
            TrackedSequence::<Fp251V1, _>::new(prime_encoder(), Fp251V1::from_u64_mod(7))
                .map_err(debug_error)?;
        for item in &items {
            baseline.push(item).map_err(debug_error)?;
        }
        Ok(single_unit(move || {
            let mut candidate = baseline.clone();
            let removed = candidate.pop().unwrap();
            candidate.push(&removed).unwrap();
            checksum_field(candidate.signature().state())
        }))
    } else {
        let mut baseline = TrackedMultiset::<Fp251V1, _>::new(prime_encoder(), Fp251V1::ONE);
        for item in &items {
            baseline.insert(item).map_err(debug_error)?;
        }
        let selected = items
            .first()
            .cloned()
            .ok_or("tracked update needs one item")?;
        Ok(single_unit(move || {
            let mut candidate = baseline.clone();
            candidate.remove(&selected).unwrap();
            candidate.insert(&selected).unwrap();
            checksum_field(candidate.signature().evaluated_product())
        }))
    }
}

fn signature_compact_snapshot_total(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    if signature_law(cell) == "sequence" {
        let mut signature =
            SequenceSignature::<Fp251V1, _>::new(prime_encoder(), Fp251V1::from_u64_mod(7))
                .map_err(debug_error)?;
        signature
            .push_many(items.iter().map(Vec::as_slice))
            .map_err(debug_error)?;
        Ok(single_unit(move || {
            checksum_bytes(&signature.to_compact_snapshot().unwrap())
        }))
    } else if signature_law(cell) == "bidirectional" {
        let mut signature = BidirectionalSequenceSignature::<Fp251V1, _>::new(
            prime_encoder(),
            Fp251V1::from_u64_mod(7),
        )
        .map_err(debug_error)?;
        signature
            .push_many(items.iter().map(Vec::as_slice))
            .map_err(debug_error)?;
        Ok(single_unit(move || {
            checksum_bytes(&signature.to_compact_snapshot().unwrap())
        }))
    } else if signature_law(cell) == "multiset" {
        let mut signature = MultisetSignature::<Fp251V1, _>::new(prime_encoder(), Fp251V1::ONE);
        signature
            .insert_many(items.iter().map(Vec::as_slice))
            .map_err(debug_error)?;
        Ok(single_unit(move || {
            checksum_bytes(&signature.to_compact_snapshot().unwrap())
        }))
    } else {
        let mut signature = AdditiveSignature::<Fp251V1, _>::new(prime_encoder());
        signature
            .absorb_many(items.iter().map(Vec::as_slice))
            .map_err(debug_error)?;
        Ok(single_unit(move || {
            checksum_bytes(&signature.to_compact_snapshot().unwrap())
        }))
    }
}

fn signature_tracked_snapshot_total(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    if signature_law(cell) == "sequence" {
        let base = Fp251V1::from_u64_mod(7);
        let mut tracked =
            TrackedSequence::<Fp251V1, _>::new(prime_encoder(), base).map_err(debug_error)?;
        for item in &items {
            tracked.push(item).map_err(debug_error)?;
        }
        Ok(single_unit(move || {
            let bytes = tracked
                .to_snapshot_bytes_with_limits(TrackedSnapshotLimits::default())
                .unwrap();
            let restored =
                TrackedSequence::from_snapshot_bytes(prime_encoder(), base, &bytes).unwrap();
            checksum_field(restored.signature().state()) ^ checksum_bytes(&bytes)
        }))
    } else {
        let offset = Fp251V1::ONE;
        let mut tracked = TrackedMultiset::<Fp251V1, _>::new(prime_encoder(), offset);
        for item in &items {
            tracked.insert(item).map_err(debug_error)?;
        }
        Ok(single_unit(move || {
            let bytes = tracked
                .to_snapshot_bytes_with_limits(TrackedSnapshotLimits::default())
                .unwrap();
            let restored =
                TrackedMultiset::from_snapshot_bytes(prime_encoder(), offset, &bytes).unwrap();
            checksum_field(restored.signature().evaluated_product()) ^ checksum_bytes(&bytes)
        }))
    }
}

fn delta_per_law_total(cell: &BenchmarkCell, seed: u64) -> Result<PreparedOperation, String> {
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    let namespace = ApplicationNamespace::derive(b"c3-delta-per-law-v1");
    match signature_law(cell) {
        "multiset" => {
            let offset = Fp251V1::ONE;
            let empty = MultisetSignature::<Fp251V1, _>::new(prime_encoder(), offset);
            let mut added = empty.clone();
            added
                .insert_many(items.iter().map(Vec::as_slice))
                .map_err(debug_error)?;
            let delta =
                MultisetDelta::new(namespace, 0, empty.clone(), added).map_err(debug_error)?;
            Ok(single_unit(move || {
                let mut state = RevisionedSignature::new(namespace, empty.clone());
                state.apply(&delta).unwrap();
                checksum_field(state.state().evaluated_product())
            }))
        }
        "trim" | "append" => {
            let base = Fp251V1::from_u64_mod(7);
            let empty =
                SequenceSignature::<Fp251V1, _>::new(prime_encoder(), base).map_err(debug_error)?;
            let mut suffix = empty.clone();
            suffix
                .push_many(items.iter().map(Vec::as_slice))
                .map_err(debug_error)?;
            if signature_law(cell) == "trim" {
                let delta = SequenceTrim::new(namespace, 0, suffix.clone()).map_err(debug_error)?;
                Ok(single_unit(move || {
                    let mut state = RevisionedSignature::new(namespace, suffix.clone());
                    state.apply(&delta).unwrap();
                    checksum_field(state.state().state())
                }))
            } else {
                let delta = SequenceAppend::new(namespace, 0, suffix).map_err(debug_error)?;
                Ok(single_unit(move || {
                    let mut state = RevisionedSignature::new(namespace, empty.clone());
                    state.apply(&delta).unwrap();
                    checksum_field(state.state().state())
                }))
            }
        }
        _ => {
            let empty = AdditiveSignature::<Fp251V1, _>::new(prime_encoder());
            let mut added = empty.clone();
            added
                .absorb_many(items.iter().map(Vec::as_slice))
                .map_err(debug_error)?;
            let delta =
                AdditiveDelta::new(namespace, 0, empty.clone(), added).map_err(debug_error)?;
            Ok(single_unit(move || {
                let mut state = RevisionedSignature::new(namespace, empty.clone());
                state.apply(&delta).unwrap();
                checksum_field(state.state().state())
            }))
        }
    }
}

fn additive_journal(count: usize) -> Result<AdditiveJournalFixture, String> {
    let namespace = ApplicationNamespace::derive(b"c3-additive-journal-v1");
    let empty = AdditiveSignature::<Fp251V1, _>::new(prime_encoder());
    let mut journal = DeltaJournal::new();
    for revision in 0..count {
        let mut added = empty.clone();
        added.absorb(&revision.to_le_bytes()).map_err(debug_error)?;
        journal
            .append(
                AdditiveDelta::new(namespace, revision as u64, empty.clone(), added)
                    .map_err(debug_error)?,
            )
            .map_err(debug_error)?;
    }
    Ok((namespace, empty, journal))
}

fn journal_append_replay_total(
    cell: &BenchmarkCell,
    _seed: u64,
) -> Result<PreparedOperation, String> {
    let (namespace, empty, journal) = additive_journal(cell.scale)?;
    Ok(single_unit(move || {
        let bytes = journal.to_canonical_bytes().unwrap();
        let restored =
            DeltaJournal::from_canonical_bytes(&bytes, DeltaJournalLimits::default(), |entry| {
                AdditiveDelta::<Fp251V1, _>::from_canonical_bytes(prime_encoder(), entry)
            })
            .unwrap();
        let mut state = RevisionedSignature::new(namespace, empty.clone());
        let report = restored.replay(&mut state).unwrap();
        checksum_field(state.state().state()) ^ report.revision()
    }))
}

fn journal_failure_path(cell: &BenchmarkCell, _seed: u64) -> Result<PreparedOperation, String> {
    let (_, _, journal) = additive_journal(cell.scale)?;
    let mut bytes = journal.to_canonical_bytes().map_err(debug_error)?;
    if cell
        .strategy
        .as_deref()
        .is_some_and(|strategy| strategy.contains("corrupt"))
    {
        let index = bytes.len() / 2;
        bytes[index] ^= 0x80;
    } else {
        bytes.pop();
    }
    Ok(single_unit(move || {
        let error =
            DeltaJournal::from_canonical_bytes(&bytes, DeltaJournalLimits::default(), |entry| {
                AdditiveDelta::<Fp251V1, _>::from_canonical_bytes(prime_encoder(), entry)
            })
            .expect_err("invalid journal must fail closed");
        checksum_bytes(format!("{error:?}").as_bytes())
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

fn file_chunk_build_total(cell: &BenchmarkCell, seed: u64) -> Result<PreparedOperation, String> {
    let chunk_bytes = cell.payload_bytes.clamp(1, 8 * 1024 * 1024);
    let profile = FileChunkProfile::fixed(chunk_bytes).map_err(debug_error)?;
    let bytes = deterministic_bytes(cell.scale, seed);
    Ok(scaled(bytes.len(), move || {
        let tree = build_tree(profile, &bytes).unwrap();
        checksum_field(tree.root().evaluation()) ^ tree.chunk_count() as u64
    }))
}

fn file_chunk_wire_total(cell: &BenchmarkCell, seed: u64) -> Result<PreparedOperation, String> {
    let bytes = deterministic_bytes(cell.scale, seed);
    let chunk_bytes = cell.payload_bytes.max(1);
    Ok(scaled(bytes.len(), move || {
        bytes.chunks(chunk_bytes).fold(0_u64, |checksum, chunk| {
            checksum.rotate_left(7) ^ checksum_bytes(&frame_file_chunk(chunk).unwrap())
        })
    }))
}

fn summary_tree_checkpoint_total(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    let profile =
        FileChunkProfile::fixed(cell.payload_bytes.clamp(1, 1024 * 1024)).map_err(debug_error)?;
    let tree = build_tree(profile, &deterministic_bytes(cell.scale, seed))?;
    Ok(scaled(tree.byte_len(), move || {
        let bytes = tree.to_checkpoint_bytes().unwrap();
        checksum_bytes(&bytes)
    }))
}

fn summary_tree_restore_total(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    let profile =
        FileChunkProfile::fixed(cell.payload_bytes.clamp(1, 1024 * 1024)).map_err(debug_error)?;
    let tree = build_tree(profile, &deterministic_bytes(cell.scale, seed))?;
    let checkpoint = tree.to_checkpoint_bytes().map_err(debug_error)?;
    let expected = tree.root();
    Ok(scaled(tree.byte_len(), move || {
        let restored = SummaryTree::from_checkpoint_bytes(
            profile,
            binary_encoder(),
            Gf2_128V1::from_polynomial_bytes_mod(&[2]),
            &checkpoint,
            SummaryTreeLimits::default(),
        )
        .unwrap();
        assert_eq!(restored.root(), expected);
        checksum_field(restored.root().evaluation())
    }))
}

fn summary_tree_grow_shrink_total(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    let profile =
        FileChunkProfile::fixed(cell.payload_bytes.clamp(1, 1024 * 1024)).map_err(debug_error)?;
    let initial_len = cell.dataset_size.unwrap_or(cell.scale).max(1);
    let mut tree = build_tree(profile, &deterministic_bytes(initial_len, seed))?;
    let addition = deterministic_bytes(cell.scale.max(1), seed ^ 0xa5a5_5a5a);
    let mut grown = false;
    Ok(single_unit(move || {
        if grown {
            tree.truncate(initial_len).unwrap();
        } else {
            tree.append(&addition).unwrap();
        }
        grown = !grown;
        checksum_field(tree.root().evaluation()) ^ tree.byte_len() as u64
    }))
}

fn frame_file_chunk(chunk: &[u8]) -> Result<Vec<u8>, String> {
    let capacity = 14_usize
        .checked_add(chunk.len())
        .ok_or("chunk frame length overflow")?;
    let mut framed = Vec::with_capacity(capacity);
    framed.extend_from_slice(b"MFFC");
    framed.extend_from_slice(&1_u16.to_le_bytes());
    framed.extend_from_slice(&(chunk.len() as u64).to_le_bytes());
    framed.extend_from_slice(chunk);
    Ok(framed)
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

fn database_row_wire_total(cell: &BenchmarkCell) -> Result<PreparedOperation, String> {
    let schema = database_schema()?;
    let rows = (0..cell.scale.max(1))
        .map(|id| database_row(id as u64, 1))
        .collect::<Vec<_>>();
    Ok(scaled(rows.len(), move || {
        rows.iter().fold(0_u64, |checksum, row| {
            let wire = schema.encode_row(row).unwrap();
            let restored = schema.decode_row(&wire).unwrap();
            assert_eq!(&restored, row);
            checksum.rotate_left(5) ^ checksum_bytes(&wire)
        })
    }))
}

fn database_mixed_mutation_total(cell: &BenchmarkCell) -> Result<PreparedOperation, String> {
    let row_count = cell.dataset_size.unwrap_or(cell.scale.max(3)).max(3);
    let mutation_count = cell.scale.min(row_count).max(1);
    let schema = database_schema()?;
    let namespace = database_namespace();
    let rows = (0..row_count)
        .map(|id| database_row(id as u64, 1))
        .collect::<Vec<_>>();
    let baseline = Database::from_rows(
        namespace,
        schema.clone(),
        16,
        binary_encoder(),
        Gf2_128V1::ONE,
        rows,
    )
    .map_err(debug_error)?;
    let deletes = mutation_count / 3;
    let updates = mutation_count / 3;
    let inserts = mutation_count - deletes - updates;
    let mut mutations = Vec::with_capacity(mutation_count);
    mutations.extend((0..deletes).map(|id| RowMutation::Delete(database_row(id as u64, 1))));
    mutations.extend((deletes..deletes + updates).map(|id| RowMutation::Update {
        before: database_row(id as u64, 1),
        after: database_row(id as u64, 2),
    }));
    mutations.extend(
        (0..inserts).map(|index| RowMutation::Insert(database_row((row_count + index) as u64, 1))),
    );
    let transaction =
        TransactionDelta::new(namespace, &schema, 0, mutations).map_err(debug_error)?;
    Ok(single_unit(move || {
        let mut candidate = baseline.clone();
        candidate
            .apply_transaction(&transaction, DatabaseTransactionLimits::default())
            .unwrap();
        checksum_field(candidate.summary().unwrap().evaluation()) ^ candidate.row_count() as u64
    }))
}

fn database_checkpoint_replay_total(cell: &BenchmarkCell) -> Result<PreparedOperation, String> {
    let count = cell.scale.max(1);
    let schema = database_schema()?;
    let namespace = database_namespace();
    let mut log = DatabaseTransactionLog::new();
    for revision in 0..count {
        log.append(
            TransactionDelta::new(
                namespace,
                &schema,
                revision as u64,
                vec![RowMutation::Insert(database_row(revision as u64, 1))],
            )
            .map_err(debug_error)?,
        )
        .map_err(debug_error)?;
    }
    let wire = log.to_canonical_bytes().map_err(debug_error)?;
    let empty = Database::new(
        namespace,
        schema.clone(),
        16,
        binary_encoder(),
        Gf2_128V1::ONE,
    )
    .map_err(debug_error)?;
    Ok(scaled(count, move || {
        let restored = DatabaseTransactionLog::from_canonical_bytes(
            namespace,
            &schema,
            &wire,
            DatabaseTransactionLimits::default(),
        )
        .unwrap();
        let mut candidate = empty.clone();
        let report = restored
            .replay(&mut candidate, DatabaseTransactionLimits::default())
            .unwrap();
        assert_eq!(report.applied(), count as u64);
        checksum_field(candidate.summary().unwrap().evaluation())
    }))
}

fn database_schema_payload_total(cell: &BenchmarkCell) -> Result<PreparedOperation, String> {
    let wide = cell
        .strategy
        .as_deref()
        .is_some_and(|strategy| strategy.contains("wide"));
    let medium = cell
        .strategy
        .as_deref()
        .is_some_and(|strategy| strategy.contains("medium"));
    let extra_columns = if wide {
        16
    } else if medium {
        4
    } else {
        1
    };
    let mut columns = vec![DatabaseColumn::new("id", DatabaseColumnType::U64, false)];
    columns.extend((0..extra_columns).map(|index| {
        DatabaseColumn::new(format!("payload_{index}"), DatabaseColumnType::Bytes, true)
    }));
    let schema = DatabaseSchema::new(1, columns, vec![0]).map_err(debug_error)?;
    let payload = deterministic_bytes(cell.payload_bytes.max(1), cell.scale as u64);
    let mut values = vec![DatabaseValue::U64(cell.scale as u64)];
    values.extend((0..extra_columns).map(|index| {
        if index % 3 == 0 {
            DatabaseValue::Null
        } else {
            DatabaseValue::Bytes(payload.clone())
        }
    }));
    let row = DatabaseRow::new(1, values);
    Ok(single_unit(move || {
        let wire = schema.encode_row(&row).unwrap();
        assert_eq!(schema.decode_row(&wire).unwrap(), row);
        checksum_bytes(&wire)
    }))
}

fn database_route_telemetry(cell: &BenchmarkCell) -> Result<PreparedOperation, String> {
    let row_count = cell
        .dataset_size
        .unwrap_or(cell.scale)
        .max(cell.scale)
        .max(16);
    let schema = database_schema()?;
    let namespace = database_namespace();
    let rows = (0..row_count)
        .map(|id| database_row(id as u64, 1))
        .collect::<Vec<_>>();
    let baseline = Database::from_rows(
        namespace,
        schema.clone(),
        16,
        binary_encoder(),
        Gf2_128V1::ONE,
        rows,
    )
    .map_err(debug_error)?;
    let mutations = (0..cell.scale)
        .map(|id| RowMutation::Update {
            before: database_row(id as u64, 1),
            after: database_row(id as u64, 2),
        })
        .collect::<Vec<_>>();
    let transaction =
        TransactionDelta::new(namespace, &schema, 0, mutations).map_err(debug_error)?;
    let policy = DatabaseApplyPolicy::adaptive(usize::MAX, 1, 4).map_err(debug_error)?;
    Ok(single_unit(move || {
        let mut candidate = baseline.clone();
        let report = candidate
            .apply_transaction_with_policy(
                &transaction,
                DatabaseTransactionLimits::default(),
                policy,
                Vec::<DatabaseRow>::new,
            )
            .unwrap();
        checksum_field(candidate.summary().unwrap().evaluation())
            ^ report.touched_partitions() as u64
            ^ (report.rebuilt_partitions() as u64).rotate_left(13)
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

fn reconciliation_sets(universe: u16, difference: usize) -> (Vec<u16>, Vec<u16>) {
    let difference = difference.clamp(1, 64);
    let only_left = difference.div_ceil(2);
    let only_right = difference / 2;
    let base_size = usize::from(universe) / 2;
    let left = (0..base_size as u16).collect::<Vec<_>>();
    let mut right = left[only_left..].to_vec();
    right.extend((0..only_right).map(|index| base_size as u16 + index as u16));
    right.sort_unstable();
    (left, right)
}

fn reconciliation_sketch_total(cell: &BenchmarkCell) -> Result<PreparedOperation, String> {
    let universe = cell.dataset_size.unwrap_or(160).clamp(64, 160) as u16;
    let reconciler = BoundedSetReconciler::new(ReconciliationLimits::new(universe, 64, 64, 65_536))
        .map_err(debug_error)?;
    let set = (0..cell.scale.min(usize::from(universe)) as u16).collect::<Vec<_>>();
    Ok(scaled(set.len(), move || {
        checksum_bytes(&reconciler.sketch(&set).unwrap().to_canonical_bytes())
    }))
}

fn reconciliation_combine_total(cell: &BenchmarkCell) -> Result<PreparedOperation, String> {
    let difference = cell.scale.clamp(1, 64);
    let universe = cell.dataset_size.unwrap_or(160).clamp(64, 160) as u16;
    let reconciler = BoundedSetReconciler::new(ReconciliationLimits::new(universe, 64, 64, 65_536))
        .map_err(debug_error)?;
    let (left, right) = reconciliation_sets(universe, difference);
    Ok(scaled(difference, move || {
        let left_sketch = reconciler.sketch(&left).unwrap();
        let right_sketch = reconciler.sketch(&right).unwrap();
        let recovered = reconciler
            .reconcile(&left_sketch, &right_sketch, &right)
            .unwrap();
        assert_eq!(recovered.distance(), difference);
        checksum_bytes(&left_sketch.to_canonical_bytes())
            ^ checksum_bytes(&right_sketch.to_canonical_bytes())
    }))
}

fn reconciliation_wire_total(cell: &BenchmarkCell) -> Result<PreparedOperation, String> {
    let universe = cell.dataset_size.unwrap_or(160).clamp(64, 160) as u16;
    let reconciler = BoundedSetReconciler::new(ReconciliationLimits::new(universe, 64, 64, 65_536))
        .map_err(debug_error)?;
    let set = (0..cell.scale.min(usize::from(universe)) as u16).collect::<Vec<_>>();
    let sketch = reconciler.sketch(&set).map_err(debug_error)?;
    let wire = sketch.to_canonical_bytes();
    Ok(single_unit(move || {
        let restored = reconciler.sketch_from_canonical_bytes(&wire).unwrap();
        assert_eq!(restored, sketch);
        checksum_bytes(&restored.to_canonical_bytes())
    }))
}

fn reconciliation_limit_path(cell: &BenchmarkCell) -> Result<PreparedOperation, String> {
    let bound = cell.scale.clamp(1, 32);
    let universe = cell.dataset_size.unwrap_or(160).clamp(64, 160) as u16;
    let reconciler =
        BoundedSetReconciler::new(ReconciliationLimits::new(universe, bound, bound, 65_536))
            .map_err(debug_error)?;
    let (left, right) = reconciliation_sets(universe, (bound + 2).min(64));
    let left_sketch = reconciler.sketch(&left).map_err(debug_error)?;
    let right_sketch = reconciler.sketch(&right).map_err(debug_error)?;
    Ok(single_unit(move || {
        let error = reconciler
            .reconcile(&left_sketch, &right_sketch, &right)
            .expect_err("over-bound difference must fail closed");
        checksum_bytes(format!("{error:?}").as_bytes())
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

fn graph_for_linear_campaign(cell: &BenchmarkCell) -> Result<IncidenceGraph, String> {
    match cell.strategy.as_deref() {
        Some("regular-8" | "parallel-regular" | "memory-regular") => {
            regular_graph(cell.scale.max(8), 8, false)
        }
        Some("regular-32") => regular_graph(cell.scale.max(32), 32, false),
        Some("mesh") => mesh_graph(cell.scale.max(16)),
        Some("star") => star_graph(cell.scale.max(4)),
        _ => sparse_cycle(cell.scale.max(4)),
    }
}

fn graph_build_prepare_total(cell: &BenchmarkCell) -> Result<PreparedOperation, String> {
    let graph = graph_for_linear_campaign(cell)?;
    let vertices = graph.vertex_count();
    Ok(scaled(vertices, move || {
        let labeler = FastGraphLabeler::<Fp251V1, _, 2>::new(
            prime_encoder(),
            algesum::RefinementProfile::fast(),
        )
        .unwrap();
        let prepared = labeler.prepare(&graph).unwrap();
        checksum_bytes(prepared.signature_id().as_bytes())
    }))
}

fn graph_channel_total(cell: &BenchmarkCell) -> Result<PreparedOperation, String> {
    let graph: &'static IncidenceGraph = Box::leak(Box::new(graph_for_linear_campaign(cell)?));
    let labeler =
        FastGraphLabeler::<Fp251V1, _, 2>::new(prime_encoder(), algesum::RefinementProfile::fast())
            .map_err(debug_error)?;
    let prepared = labeler.prepare(graph).map_err(debug_error)?;
    let mut workspace = GraphWorkspace::new();
    workspace.reserve_for(graph.vertex_count(), 4);
    Ok(scaled(graph.vertex_count(), move || {
        let analysis = labeler
            .analyze_prepared_hybrid_with_workspace(
                &prepared,
                &mut workspace,
                GraphExecution::Sequential,
            )
            .unwrap();
        checksum_field(analysis.structural().signature().lanes()[0])
            ^ checksum_bytes(analysis.invariant_digest().as_bytes())
    }))
}

fn graph_parallel_total(cell: &BenchmarkCell) -> Result<PreparedOperation, String> {
    let graph: &'static IncidenceGraph = Box::leak(Box::new(graph_for_linear_campaign(cell)?));
    let labeler =
        FastGraphLabeler::<Fp251V1, _, 2>::new(prime_encoder(), algesum::RefinementProfile::fast())
            .map_err(debug_error)?;
    let prepared = labeler.prepare(graph).map_err(debug_error)?;
    let mut workspace = GraphWorkspace::new();
    workspace.reserve_for(graph.vertex_count(), 4);
    Ok(scaled(graph.vertex_count(), move || {
        let analysis = labeler
            .analyze_prepared_with_workspace(
                &prepared,
                &mut workspace,
                GraphExecution::Parallel {
                    minimum_vertices: 1,
                },
            )
            .unwrap();
        checksum_field(analysis.signature().lanes()[0])
    }))
}

fn graph_incremental_batch_total(cell: &BenchmarkCell) -> Result<PreparedOperation, String> {
    let vertices = cell.dataset_size.unwrap_or(cell.scale.max(64)).max(8);
    let edits = cell.scale.clamp(1, vertices);
    let graph = regular_graph(vertices, 8, false)?;
    let labeler =
        FastGraphLabeler::<Fp251V1, _, 2>::new(prime_encoder(), algesum::RefinementProfile::fast())
            .map_err(debug_error)?;
    let mut state = labeler.incremental_state(graph).map_err(debug_error)?;
    let mut workspace = IncrementalGraphWorkspace::new();
    workspace
        .reserve_for(vertices, state.graph().incidence_count(), 4)
        .map_err(debug_error)?;
    let mut generation = 0_u64;
    Ok(scaled(edits, move || {
        generation = generation.wrapping_add(1);
        let mut delta = GraphDelta::new().with_expected_revision(state.revision());
        for index in 0..edits {
            delta
                .set_vertex_label(
                    VertexId::new(index),
                    generation.wrapping_add(index as u64).to_le_bytes(),
                )
                .unwrap();
        }
        let report = labeler
            .apply_delta(
                &mut state,
                &delta,
                GraphDeltaPolicy::default(),
                &mut workspace,
            )
            .unwrap();
        report.estimated_vertex_rounds() as u64 ^ state.revision()
    }))
}

fn graph_memory_telemetry(cell: &BenchmarkCell) -> Result<PreparedOperation, String> {
    let graph = graph_for_linear_campaign(cell)?;
    let vertices = graph.vertex_count();
    let labeler =
        FastGraphLabeler::<Fp251V1, _, 2>::new(prime_encoder(), algesum::RefinementProfile::fast())
            .map_err(debug_error)?;
    Ok(scaled(vertices, move || {
        let analysis = labeler.analyze(&graph).unwrap();
        checksum_field(analysis.signature().lanes()[0])
    }))
}

fn graph_exact(cell: &BenchmarkCell) -> Result<PreparedOperation, String> {
    let strategy = cell.strategy.as_deref();
    let graph = match strategy {
        Some(
            "symmetric-cycle"
            | "symmetric-cycle-budget-1"
            | "budget-nodes"
            | "budget-cells"
            | "budget-bytes"
            | "budget-depth",
        ) => sparse_cycle(cell.scale.clamp(3, 16))?,
        Some("regular-8") => regular_graph(cell.scale.clamp(8, 16), 8, false)?,
        Some("disconnected-cycles") => disconnected_cycles(cell.scale.clamp(6, 16))?,
        Some("star") => star_graph(cell.scale.clamp(4, 16))?,
        _ => distinct_path(cell.scale.clamp(2, 16))?,
    };
    let schema = GraphSchemaId::derive(b"publication-exact-v1");
    let canonizer = Microcanon::new(schema);
    let node_budget = match strategy {
        Some("symmetric-cycle-budget-1") => 1,
        Some("budget-nodes") => cell.payload_bytes.max(1) as u64,
        _ => 1_000_000,
    };
    let budget = match strategy {
        Some("budget-cells") => CanonicalSearchBudget::new(node_budget)
            .with_max_retained_state_cells(cell.payload_bytes.max(1)),
        Some("budget-bytes") => CanonicalSearchBudget::new(node_budget)
            .with_max_retained_bytes(cell.payload_bytes.max(1)),
        Some("budget-depth") => {
            CanonicalSearchBudget::new(node_budget).with_max_depth(cell.payload_bytes)
        }
        _ => CanonicalSearchBudget::new(node_budget),
    };
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

fn graph_dag_persist_restore_total(cell: &BenchmarkCell) -> Result<PreparedOperation, String> {
    let node_count = cell.scale.clamp(1, 12);
    let schema = GraphSchemaId::derive(b"publication-dag-persistence-v1");
    let canonizer = Microcanon::new(schema);
    let budget = CanonicalSearchBudget::new(1_000_000);
    let mut dag = CanonicalGraphDag::new(schema);
    let mut dependency = None;
    for size in 1..=node_count {
        let graph = distinct_path(size + 1)?;
        let dependencies = dependency.into_iter().collect::<Vec<_>>();
        let outcome = dag
            .resolve(&graph, &canonizer, budget, &dependencies, None)
            .map_err(debug_error)?;
        dependency = match outcome {
            algesum::GraphDagResolveOutcome::Inserted { node, .. }
            | algesum::GraphDagResolveOutcome::Reused { node, .. } => Some(node),
            algesum::GraphDagResolveOutcome::Inconclusive => {
                return Err("DAG fixture was inconclusive".into())
            }
        };
    }
    let wire = dag.to_canonical_bytes();
    Ok(scaled(wire.len(), move || {
        let restored = CanonicalGraphDag::from_canonical_bytes(
            &wire,
            &canonizer,
            budget,
            CanonicalGraphDagLimits::default(),
        )
        .unwrap();
        assert_eq!(restored, dag);
        checksum_bytes(&restored.to_canonical_bytes())
    }))
}

fn graph_dag_incremental_total(cell: &BenchmarkCell) -> Result<PreparedOperation, String> {
    let vertices = cell.scale.clamp(3, 14);
    let schema = GraphSchemaId::derive(b"publication-dag-incremental-v1");
    let canonizer = Microcanon::new(schema);
    let budget = CanonicalSearchBudget::new(1_000_000);
    let original = distinct_path(vertices)?;
    let labeler =
        FastGraphLabeler::<Fp251V1, _, 2>::new(prime_encoder(), algesum::RefinementProfile::fast())
            .map_err(debug_error)?;
    let mut state = labeler
        .incremental_state(original.clone())
        .map_err(debug_error)?;
    let mut workspace = IncrementalGraphWorkspace::new();
    workspace
        .reserve_for(vertices, original.incidence_count(), 4)
        .map_err(debug_error)?;
    let mut dag = CanonicalGraphDag::new(schema);
    dag.resolve(&original, &canonizer, budget, &[], None)
        .map_err(debug_error)?;
    let mut generation = 0_u64;
    Ok(scaled(vertices, move || {
        generation = generation.wrapping_add(1);
        let mut delta = GraphDelta::new().with_expected_revision(state.revision());
        delta
            .set_vertex_label(
                VertexId::new((generation as usize) % vertices),
                generation.to_le_bytes(),
            )
            .unwrap();
        let report = labeler
            .apply_delta(
                &mut state,
                &delta,
                GraphDeltaPolicy::default(),
                &mut workspace,
            )
            .unwrap();
        let outcome = dag
            .resolve_after_delta(
                state.graph(),
                report,
                &canonizer,
                budget,
                &[],
                Some(dag.revision()),
            )
            .unwrap();
        match outcome {
            algesum::GraphDagResolveOutcome::Inserted { node, .. }
            | algesum::GraphDagResolveOutcome::Reused { node, .. } => node.as_u64(),
            algesum::GraphDagResolveOutcome::Inconclusive => {
                panic!("incremental DAG fixture was inconclusive")
            }
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

fn field_runtime_lifecycle_total(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    let binary = cell
        .strategy
        .as_deref()
        .is_some_and(|strategy| strategy.contains("binary"));
    let iterations = cell.scale;
    Ok(single_unit(move || {
        let mut checksum = 0_u64;
        for index in 0..iterations {
            let field = if binary {
                DynField::builder(format!("c3_runtime_binary_{index}"))
                    .binary(8, vec![8, 4, 3, 1, 0])
                    .build()
                    .unwrap()
            } else {
                DynField::builder(format!("c3_runtime_prime_{index}"))
                    .prime("251")
                    .build()
                    .unwrap()
            };
            let value =
                field.reduce_bytes_mod_order(&seed.wrapping_add(index as u64).to_le_bytes());
            checksum = checksum.rotate_left(5) ^ checksum_dyn(&field, &value);
        }
        checksum
    }))
}

fn field_runtime_operation_total(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    let field = DynField::builder("c3_runtime_fp251")
        .prime("251")
        .build()
        .map_err(debug_error)?;
    let values = (0..cell.scale)
        .map(|index| field.reduce_bytes_mod_order(&seed.wrapping_add(index as u64).to_le_bytes()))
        .collect::<Vec<_>>();
    let lhs = DynBatch::from_elements(&field, &values).map_err(debug_error)?;
    let rhs = lhs.clone();
    let mut output = DynBatch::zeroed(&field, cell.scale);
    let engine = field.engine();
    Ok(single_unit(move || {
        engine.mul_into(&mut output, &lhs, &rhs).unwrap();
        (0..output.len()).fold(0_u64, |checksum, index| {
            checksum.rotate_left(7)
                ^ checksum_dyn(&field, &output.element(index).expect("batch element"))
        })
    }))
}

fn tool_prime_manifest_total(cell: &BenchmarkCell) -> Result<PreparedOperation, String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../microfield/fields/fp65521_external_v1.toml");
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("read prime field manifest {}: {error}", path.display()))?;
    let generate = cell
        .strategy
        .as_deref()
        .is_some_and(|strategy| strategy.contains("generate"));
    Ok(single_unit(move || {
        if generate {
            let package = PrimeFieldFactory::from_manifest_toml(&source)
                .unwrap()
                .generate()
                .unwrap();
            checksum_bytes(package.rust_source())
        } else {
            let normalized = PrimeFieldManifest::parse_toml(&source)
                .unwrap()
                .normalize()
                .unwrap();
            checksum_bytes(normalized.modulus_decimal().as_bytes())
        }
    }))
}

fn tool_artifact_compile_consumer_total() -> Result<PreparedOperation, String> {
    static NEXT_TARGET: AtomicU64 = AtomicU64::new(0);

    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../microfield/test-fixtures/external-consumer/Cargo.toml");
    if !manifest.is_file() {
        return Err(format!("missing external consumer {}", manifest.display()));
    }
    let target = std::env::temp_dir().join(format!(
        "algesum-c3-consumer-check-{}-{}",
        std::process::id(),
        NEXT_TARGET.fetch_add(1, Ordering::Relaxed)
    ));
    Ok(PreparedOperation {
        logical_units_per_action: 1,
        maximum_batch_iterations: Some(1),
        graph_exact: None,
        run: Box::new(move || {
            if target.exists() {
                fs::remove_dir_all(&target).expect("remove isolated consumer target");
            }
            fs::create_dir_all(&target).expect("create isolated consumer target");
            let status = Command::new("cargo")
                .arg("check")
                .arg("--locked")
                .arg("--offline")
                .arg("--manifest-path")
                .arg(&manifest)
                .env("CARGO_TARGET_DIR", &target)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .expect("launch external consumer cargo check");
            assert!(status.success(), "external consumer compilation failed");
            checksum_bytes(manifest.as_os_str().as_encoded_bytes())
        }),
    })
}

fn wire_family_roundtrip_total(
    cell: &BenchmarkCell,
    seed: u64,
) -> Result<PreparedOperation, String> {
    match cell.strategy.as_deref().unwrap_or("additive") {
        "additive" | "sequence" | "bidirectional" | "multiset" => {
            signature_base_wire_total(cell, seed)
        }
        "multi-sequence" | "multi-multiset" => signature_multi_wire_restore_total(cell, seed),
        "summary-tree" => summary_tree_restore_total(cell, seed),
        "database-row" => database_row_wire_total(cell),
        "reconciliation" => reconciliation_wire_total(cell),
        "dag" => graph_dag_persist_restore_total(cell),
        "journal" => journal_append_replay_total(cell, seed),
        strategy => Err(format!("unsupported wire family {strategy:?}")),
    }
}

fn wire_failure_matrix(cell: &BenchmarkCell, seed: u64) -> Result<PreparedOperation, String> {
    type Rejector = Box<dyn Fn(&[u8]) -> bool>;

    let strategy = cell.strategy.as_deref().unwrap_or("additive");
    let (wire, rejects): (Vec<u8>, Rejector) = match strategy {
        "additive" => {
            let mut signature = AdditiveSignature::<Fp251V1, _>::new(prime_encoder());
            signature
                .absorb_many(payloads(cell.scale, cell.payload_bytes, seed)?.iter())
                .map_err(debug_error)?;
            (
                signature.to_canonical_bytes(),
                Box::new(|bytes| {
                    AdditiveSignature::<Fp251V1, _>::from_canonical_bytes(prime_encoder(), bytes)
                        .is_err()
                }),
            )
        }
        "multi-multiset" => {
            let offsets = [
                Fp251V1::ONE,
                Fp251V1::from_u64_mod(2),
                Fp251V1::from_u64_mod(3),
                Fp251V1::from_u64_mod(4),
            ];
            let mut signature =
                MultiEvaluationMultisetSignature::<Fp251V1, _, 4>::new(prime_encoder(), offsets)
                    .map_err(debug_error)?;
            signature
                .insert_many(payloads(cell.scale, cell.payload_bytes, seed)?.iter())
                .map_err(debug_error)?;
            (
                signature.to_canonical_bytes(),
                Box::new(move |bytes| {
                    MultiEvaluationMultisetSignature::<Fp251V1, _, 4>::from_canonical_bytes(
                        prime_encoder(),
                        offsets,
                        bytes,
                    )
                    .is_err()
                }),
            )
        }
        "summary-tree" => {
            let profile =
                FileChunkProfile::fixed(cell.payload_bytes.max(1)).map_err(debug_error)?;
            let tree = build_tree(profile, &deterministic_bytes(cell.scale.max(1), seed))?;
            (
                tree.to_checkpoint_bytes().map_err(debug_error)?,
                Box::new(move |bytes| {
                    SummaryTree::from_checkpoint_bytes(
                        profile,
                        binary_encoder(),
                        Gf2_128V1::from_polynomial_bytes_mod(&[2]),
                        bytes,
                        SummaryTreeLimits::default(),
                    )
                    .is_err()
                }),
            )
        }
        "database-row" => {
            let schema = database_schema()?;
            (
                schema
                    .encode_row(&database_row(1, seed))
                    .map_err(debug_error)?,
                Box::new(move |bytes| schema.decode_row(bytes).is_err()),
            )
        }
        "reconciliation" => {
            let reconciler = BoundedSetReconciler::new(ReconciliationLimits::new(64, 8, 8, 4096))
                .map_err(debug_error)?;
            let sketch = reconciler.sketch(&[1, 2, 3]).map_err(debug_error)?;
            (
                sketch.to_canonical_bytes(),
                Box::new(move |bytes| reconciler.sketch_from_canonical_bytes(bytes).is_err()),
            )
        }
        "dag" => {
            let schema = GraphSchemaId::derive(b"c3-wire-failure-dag-v1");
            let canonizer = Microcanon::new(schema);
            let budget = CanonicalSearchBudget::new(1_000_000);
            let mut dag = CanonicalGraphDag::new(schema);
            dag.resolve(&distinct_path(4)?, &canonizer, budget, &[], None)
                .map_err(debug_error)?;
            (
                dag.to_canonical_bytes(),
                Box::new(move |bytes| {
                    CanonicalGraphDag::from_canonical_bytes(
                        bytes,
                        &canonizer,
                        budget,
                        CanonicalGraphDagLimits::default(),
                    )
                    .is_err()
                }),
            )
        }
        other => return Err(format!("unsupported wire failure family {other:?}")),
    };
    Ok(scaled(wire.len(), move || {
        assert!(!wire.is_empty());
        assert!(rejects(&wire[..wire.len() - 1]));
        let mut corrupt = wire.clone();
        corrupt[0] ^= 0xff;
        assert!(rejects(&corrupt));
        checksum_bytes(&wire)
    }))
}

fn legacy_facade_semantic(cell: &BenchmarkCell, seed: u64) -> Result<PreparedOperation, String> {
    let items = payloads(cell.scale, cell.payload_bytes, seed)?;
    let mut generator = [0_u8; 32];
    generator[31] = 0x80;
    let offset = Gf2_256HhV1::from_canonical(&generator).map_err(debug_error)?;
    Ok(scaled(items.len(), move || {
        let mut legacy = TopoHasher::<GaloisSignature256, MultisetAggregator>::new();
        let mut maintained =
            MultisetSignature::<Gf2_256HhV1, _>::new(LegacyAffineEncoderV1, offset);
        for item in &items {
            legacy.update(item);
            maintained.insert(item).unwrap();
        }
        let legacy = legacy.finalize().to_canonical_bytes();
        let current = maintained.evaluated_product().to_canonical();
        assert_eq!(legacy, current);
        checksum_bytes(&legacy)
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

fn signature_law(cell: &BenchmarkCell) -> &str {
    let strategy = cell.strategy.as_deref().unwrap_or("additive");
    for law in ["bidirectional", "multiset", "sequence", "append", "trim"] {
        if strategy.contains(law) {
            return law;
        }
    }
    "additive"
}

fn gf2_256_values(
    count: usize,
    seed: u64,
    include_zeroes: bool,
) -> Result<Vec<Gf2_256HhV1>, String> {
    if count == 0 {
        return Err("GF(2^256) workload scale must be positive".into());
    }
    Ok((0..count)
        .map(|index| {
            if include_zeroes && index % 29 == 0 {
                return Gf2_256HhV1::ZERO;
            }
            let value = Gf2_256HhV1::from_polynomial_bytes_mod(&seed_bytes::<32>(
                seed.wrapping_add(index as u64),
            ));
            if value.is_zero() {
                Gf2_256HhV1::ONE
            } else {
                value
            }
        })
        .collect())
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

fn disconnected_cycles(vertices: usize) -> Result<IncidenceGraph, String> {
    let vertices = vertices.max(6);
    let left_count = vertices / 2;
    let right_count = vertices - left_count;
    let mut builder = IncidenceGraphBuilder::new();
    let ids = (0..vertices)
        .map(|_| builder.add_vertex(b"cycle"))
        .collect::<Vec<_>>();
    for (offset, count) in [(0, left_count), (left_count, right_count)] {
        for index in 0..count {
            builder
                .add_undirected_relation(
                    ids[offset + index],
                    ids[offset + (index + 1) % count],
                    b"edge",
                    b"disconnected-cycle",
                    1,
                )
                .map_err(debug_error)?;
        }
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

fn checksum_fields<F: CanonicalEncoding>(values: &[F]) -> u64 {
    values
        .iter()
        .enumerate()
        .fold(0_u64, |checksum, (index, value)| {
            checksum.rotate_left(7) ^ checksum_field(*value) ^ index as u64
        })
}

fn checksum_dyn(field: &DynField, value: &microfield::DynElement) -> u64 {
    let mut bytes = vec![0_u8; field.canonical_bytes()];
    field
        .encode(value, &mut bytes)
        .expect("matching runtime field");
    checksum_bytes(&bytes)
}

fn checksum_dyn_fields(field: &DynField, values: &[microfield::DynElement]) -> u64 {
    values
        .iter()
        .enumerate()
        .fold(0_u64, |checksum, (index, value)| {
            checksum.rotate_left(7) ^ checksum_dyn(field, value) ^ index as u64
        })
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
    use std::path::Path;

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
    fn c3_f3_s3_preflight_actions_are_repeatable() {
        let root = Path::new("../../validation/benchmarks/manifests/c3-f3-s3");
        for manifest in [
            "c3-f3-batch-packed-preflight-v1.json",
            "c3-f3-horner-preflight-v1.json",
            "c3-f3-derived-preflight-v1.json",
            "c3-f4-runtime-tools-preflight-v1.json",
            "c3-s1-base-signatures-preflight-v1.json",
            "c3-s2-multi-signatures-preflight-v1.json",
            "c3-s3-state-delta-journal-preflight-v1.json",
        ] {
            let manifest = crate::publication::load_manifest(&root.join(manifest)).unwrap();
            for benchmark_cell in manifest.cells {
                let mut prepared = prepare(&benchmark_cell, 0x1234_5678).unwrap_or_else(|error| {
                    panic!("prepare repeatability cell {}: {error}", benchmark_cell.id)
                });
                let first = (prepared.run)();
                let second = (prepared.run)();
                assert_eq!(first, second, "stateful C3 workload: {}", benchmark_cell.id);
            }
        }
    }

    #[test]
    fn c3_f3_scalar_layouts_compute_the_same_products() {
        let mut checksums = Vec::new();
        for operation in [
            "field.batch-portable-total",
            "field.packed-owned-total",
            "field.packed-view-total",
        ] {
            let mut prepared = prepare(&cell(operation, 257), 0xfeed_beef).unwrap();
            checksums.push((prepared.run)());
        }
        assert!(checksums.windows(2).all(|pair| pair[0] == pair[1]));
    }

    #[test]
    fn c3_t1_r1_d1_preflight_actions_are_reproducible_from_the_same_seed() {
        let root = Path::new("../../validation/benchmarks/manifests/c3-t1-r1-d1");
        for manifest in [
            "c3-t1-file-tree-state-preflight-v1.json",
            "c3-r1-bounded-reconciliation-preflight-v1.json",
            "c3-d1-database-state-preflight-v1.json",
        ] {
            let manifest = crate::publication::load_manifest(&root.join(manifest)).unwrap();
            for benchmark_cell in manifest.cells {
                let mut first = prepare(&benchmark_cell, 0x1234_5678).unwrap_or_else(|error| {
                    panic!("prepare first cell {}: {error}", benchmark_cell.id)
                });
                let mut second = prepare(&benchmark_cell, 0x1234_5678).unwrap_or_else(|error| {
                    panic!("prepare second cell {}: {error}", benchmark_cell.id)
                });
                assert_eq!(
                    (first.run)(),
                    (second.run)(),
                    "non-reproducible C3 workload: {}",
                    benchmark_cell.id
                );
            }
        }
    }

    #[test]
    fn c3_g1_g2_preflight_actions_are_reproducible_from_the_same_seed() {
        let root = Path::new("../../validation/benchmarks/manifests/c3-g1-g2");
        for manifest in [
            "c3-g1-linear-graph-pipeline-preflight-v1.json",
            "c3-g1-incremental-batch-preflight-v1.json",
            "c3-g2-exact-budgets-preflight-v1.json",
            "c3-g2-exact-families-preflight-v1.json",
            "c3-g2-dag-state-preflight-v1.json",
        ] {
            let manifest = crate::publication::load_manifest(&root.join(manifest)).unwrap();
            for benchmark_cell in manifest.cells {
                let mut first = prepare(&benchmark_cell, 0x1234_5678).unwrap_or_else(|error| {
                    panic!("prepare first cell {}: {error}", benchmark_cell.id)
                });
                let mut second = prepare(&benchmark_cell, 0x1234_5678).unwrap_or_else(|error| {
                    panic!("prepare second cell {}: {error}", benchmark_cell.id)
                });
                assert_eq!(
                    (first.run)(),
                    (second.run)(),
                    "non-reproducible C3 workload: {}",
                    benchmark_cell.id
                );
            }
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
