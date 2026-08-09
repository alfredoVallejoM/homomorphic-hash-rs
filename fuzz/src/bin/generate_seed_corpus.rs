//! Regenerates deterministic, valid RC.7 corpus seeds through public APIs.

use std::{fs, io, path::Path};

use homomorphic_hash_rs::{
    AdditiveDelta, AdditiveSignature, ApplicationNamespace, BidirectionalSequenceSignature,
    BinaryPolynomialEncoder, BoundedSetReconciler, CanonicalGraphDag, CanonicalSearchBudget,
    DatabaseColumn, DatabaseColumnType, DatabaseRow, DatabaseSchema, DatabaseTransactionLog,
    DatabaseValue, DeltaJournal, FileChunkProfile, GraphSchemaId, HomomorphicSummaryTree,
    IncidenceGraphBuilder, Microcanon, MultiEvaluationMultisetSignature,
    MultiEvaluationSequenceSignature, MultisetDelta, MultisetSignature, PrimeIntegerEncoder,
    ReconciliationLimits, RowMutation, SequenceAppend, SequenceSignature, SequenceTrim,
    SignatureDelta, TrackedMultiset, TrackedSequence, TransactionDelta,
};
use microfield::{BinaryPolynomialField, Field, Fp251V1, Gf2_128V1};

fn write_seed(directory: &Path, name: &str, bytes: &[u8]) -> io::Result<()> {
    fs::create_dir_all(directory)?;
    fs::write(directory.join(name), bytes)
}

fn encoder() -> PrimeIntegerEncoder {
    PrimeIntegerEncoder::new(0x5243_7001)
}

fn base() -> Fp251V1 {
    Fp251V1::from_u64_mod(7)
}

fn additive(items: &[&[u8]]) -> AdditiveSignature<Fp251V1, PrimeIntegerEncoder> {
    let mut signature = AdditiveSignature::new(encoder());
    signature.absorb_many(items.iter().copied()).unwrap();
    signature
}

fn sequence(items: &[&[u8]]) -> SequenceSignature<Fp251V1, PrimeIntegerEncoder> {
    let mut signature = SequenceSignature::new(encoder(), base()).unwrap();
    signature.push_many(items.iter().copied()).unwrap();
    signature
}

fn multiset(items: &[&[u8]]) -> MultisetSignature<Fp251V1, PrimeIntegerEncoder> {
    let mut signature = MultisetSignature::new(encoder(), Fp251V1::ONE);
    signature.insert_many(items.iter().copied()).unwrap();
    signature
}

fn structural_seeds(directory: &Path) -> io::Result<()> {
    let items: [&[u8]; 3] = [b"alpha", b"beta", b"alpha"];
    let namespace = ApplicationNamespace::derive(b"rc7-fuzz-database-v1");

    let additive_signature = additive(&items);
    write_seed(
        directory,
        "valid-additive-mfsg",
        &additive_signature.to_canonical_bytes(),
    )?;
    let sequence_signature = sequence(&items);
    write_seed(
        directory,
        "valid-sequence-mfsg",
        &sequence_signature.to_canonical_bytes(),
    )?;
    let multiset_signature = multiset(&items);
    write_seed(
        directory,
        "valid-multiset-mfsg",
        &multiset_signature.to_canonical_bytes(),
    )?;

    let mut bidirectional = BidirectionalSequenceSignature::new(encoder(), base()).unwrap();
    bidirectional.push_many(items).unwrap();
    write_seed(
        directory,
        "valid-bidirectional-mfsg",
        &bidirectional.to_canonical_bytes(),
    )?;
    let mut multi_sequence = MultiEvaluationSequenceSignature::<Fp251V1, _, 2>::new(
        encoder(),
        [base(), Fp251V1::from_u64_mod(11)],
    )
    .unwrap();
    multi_sequence.push_many(items).unwrap();
    write_seed(
        directory,
        "valid-multi-sequence-mfsg",
        &multi_sequence.to_canonical_bytes(),
    )?;
    let mut multi_multiset = MultiEvaluationMultisetSignature::<Fp251V1, _, 2>::new(
        encoder(),
        [Fp251V1::ZERO, Fp251V1::ONE],
    )
    .unwrap();
    multi_multiset.insert_many(items).unwrap();
    write_seed(
        directory,
        "valid-multi-multiset-mfsg",
        &multi_multiset.to_canonical_bytes(),
    )?;

    let mut tracked_sequence = TrackedSequence::new(encoder(), base()).unwrap();
    for item in items {
        tracked_sequence.push(item).unwrap();
    }
    write_seed(
        directory,
        "valid-tracked-sequence-mfts",
        &tracked_sequence.to_snapshot_bytes().unwrap(),
    )?;
    let mut tracked_multiset = TrackedMultiset::new(encoder(), Fp251V1::ONE);
    for item in items {
        tracked_multiset.insert(item).unwrap();
    }
    write_seed(
        directory,
        "valid-tracked-multiset-mfts",
        &tracked_multiset.to_snapshot_bytes().unwrap(),
    )?;

    let additive_delta = AdditiveDelta::new(namespace, 0, additive(&[]), additive(&items)).unwrap();
    write_seed(
        directory,
        "valid-additive-mfde",
        &additive_delta.to_canonical_bytes(),
    )?;
    let multiset_delta = MultisetDelta::new(namespace, 0, multiset(&[]), multiset(&items)).unwrap();
    write_seed(
        directory,
        "valid-multiset-mfde",
        &multiset_delta.to_canonical_bytes(),
    )?;
    let append = SequenceAppend::new(namespace, 0, sequence(&items)).unwrap();
    write_seed(
        directory,
        "valid-sequence-append-mfde",
        &append.to_canonical_bytes(),
    )?;
    let trim = SequenceTrim::new(namespace, 0, sequence(&items)).unwrap();
    write_seed(
        directory,
        "valid-sequence-trim-mfde",
        &trim.to_canonical_bytes(),
    )?;
    let mut journal = DeltaJournal::new();
    journal.append(additive_delta).unwrap();
    write_seed(
        directory,
        "valid-additive-mfdj",
        &journal.to_canonical_bytes().unwrap(),
    )?;

    let schema = DatabaseSchema::new(
        1,
        vec![
            DatabaseColumn::new("id", DatabaseColumnType::U64, false),
            DatabaseColumn::new("payload", DatabaseColumnType::Bytes, false),
            DatabaseColumn::new("note", DatabaseColumnType::Text, true),
        ],
        vec![0],
    )
    .unwrap();
    let row = DatabaseRow::new(
        1,
        vec![
            DatabaseValue::U64(7),
            DatabaseValue::Bytes(b"payload".to_vec()),
            DatabaseValue::Text("seed".to_owned()),
        ],
    );
    write_seed(
        directory,
        "valid-row-mfrw",
        &schema.encode_row(&row).unwrap(),
    )?;
    let transaction =
        TransactionDelta::new(namespace, &schema, 0, vec![RowMutation::Insert(row)]).unwrap();
    write_seed(
        directory,
        "valid-transaction-mftx",
        &transaction.to_canonical_bytes(),
    )?;
    let mut transaction_log = DatabaseTransactionLog::new();
    transaction_log.append(transaction).unwrap();
    write_seed(
        directory,
        "valid-transaction-log-mftl",
        &transaction_log.to_canonical_bytes().unwrap(),
    )?;

    let profile = FileChunkProfile::fixed(23).unwrap();
    let tree = HomomorphicSummaryTree::from_bytes(
        profile,
        BinaryPolynomialEncoder::new(0x5243_7002),
        Gf2_128V1::from_polynomial_bytes_mod(&[2]),
        b"a deterministic summary-tree corpus seed",
    )
    .unwrap();
    write_seed(
        directory,
        "valid-summary-tree-mfst",
        &tree.to_checkpoint_bytes().unwrap(),
    )?;

    let reconciler = BoundedSetReconciler::new(ReconciliationLimits::new(64, 6, 8, 1_024)).unwrap();
    let sketch = reconciler.sketch(&[1, 3, 8, 13]).unwrap();
    write_seed(
        directory,
        "valid-reconciliation-mfrs",
        &sketch.to_canonical_bytes(),
    )
}

fn graph_seeds(directory: &Path) -> io::Result<()> {
    let mut builder = IncidenceGraphBuilder::new();
    let first = builder.add_vertex(b"same");
    let second = builder.add_vertex(b"same");
    builder
        .add_undirected_relation(first, second, b"edge", b"seed", 2)
        .unwrap();
    let graph = builder.build().unwrap();
    let schema = GraphSchemaId::derive(b"rc7-fuzz-graph-v1");
    let canonizer = Microcanon::new(schema);
    let budget = CanonicalSearchBudget::new(20_000);
    let form = match canonizer.canonicalize(&graph, budget).unwrap() {
        homomorphic_hash_rs::MicrocanonOutcome::Exact { form, .. } => form,
        homomorphic_hash_rs::MicrocanonOutcome::Inconclusive { report } => {
            panic!("constant graph exceeded corpus budget: {report:?}")
        }
    };
    write_seed(directory, "valid-canonical-graph-mfc2", form.bytes())?;

    let mut dag = CanonicalGraphDag::new(schema);
    dag.resolve(&graph, &canonizer, budget, &[], None).unwrap();
    write_seed(
        directory,
        "valid-canonical-dag-mfgd",
        &dag.to_canonical_bytes(),
    )
}

fn main() -> io::Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
    structural_seeds(&root.join("structural_wires"))?;
    graph_seeds(&root.join("graph_wires"))
}
