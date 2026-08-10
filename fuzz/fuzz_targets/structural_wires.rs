#![no_main]

use algesum::{
    AdditiveDelta, AdditiveSignature, ApplicationNamespace, BidirectionalSequenceSignature,
    BinaryPolynomialEncoder, BoundedSetReconciler, DatabaseColumn, DatabaseColumnType,
    DatabaseSchema, DatabaseTransactionLimits, DatabaseTransactionLog, DeltaJournal,
    DeltaJournalLimits, FileChunkProfile, HomomorphicSummaryTree, MultiEvaluationMultisetSignature,
    MultiEvaluationSequenceSignature, MultisetDelta, MultisetSignature, PrimeIntegerEncoder,
    ReconciliationLimits, SequenceAppend, SequenceSignature, SequenceTrim, SummaryTreeLimits,
    TrackedMultiset, TrackedSequence, TransactionDelta,
};
use libfuzzer_sys::fuzz_target;
use microfield::{BinaryPolynomialField, Field, Fp251V1, Gf2_128V1};

fn prime_encoder() -> PrimeIntegerEncoder {
    PrimeIntegerEncoder::new(0x5243_7001)
}

fn base() -> Fp251V1 {
    Fp251V1::from_u64_mod(7)
}

fn database_schema() -> DatabaseSchema {
    DatabaseSchema::new(
        1,
        vec![
            DatabaseColumn::new("id", DatabaseColumnType::U64, false),
            DatabaseColumn::new("payload", DatabaseColumnType::Bytes, false),
            DatabaseColumn::new("note", DatabaseColumnType::Text, true),
        ],
        vec![0],
    )
    .expect("constant fuzz schema is valid")
}

fuzz_target!(|data: &[u8]| {
    let encoder = prime_encoder();
    let _ = AdditiveSignature::<Fp251V1, _>::from_canonical_bytes(encoder, data);
    let _ = SequenceSignature::<Fp251V1, _>::from_canonical_bytes(encoder, base(), data);
    let _ =
        BidirectionalSequenceSignature::<Fp251V1, _>::from_canonical_bytes(encoder, base(), data);
    let _ = MultisetSignature::<Fp251V1, _>::from_canonical_bytes(encoder, Fp251V1::ONE, data);
    let _ = MultiEvaluationSequenceSignature::<Fp251V1, _, 2>::from_canonical_bytes(
        encoder,
        [Fp251V1::from_u64_mod(7), Fp251V1::from_u64_mod(11)],
        data,
    );
    let _ = MultiEvaluationMultisetSignature::<Fp251V1, _, 2>::from_canonical_bytes(
        encoder,
        [Fp251V1::ZERO, Fp251V1::ONE],
        data,
    );
    let _ = TrackedSequence::<Fp251V1, _>::from_snapshot_bytes(encoder, base(), data);
    let _ = TrackedMultiset::<Fp251V1, _>::from_snapshot_bytes(encoder, Fp251V1::ONE, data);

    let _ = AdditiveDelta::<Fp251V1, _>::from_canonical_bytes(encoder, data);
    let _ = MultisetDelta::<Fp251V1, _>::from_canonical_bytes(encoder, Fp251V1::ONE, data);
    let _ = SequenceAppend::<Fp251V1, _>::from_canonical_bytes(encoder, base(), data);
    let _ = SequenceTrim::<Fp251V1, _>::from_canonical_bytes(encoder, base(), data);
    let _ = DeltaJournal::<AdditiveDelta<Fp251V1, PrimeIntegerEncoder>>::from_canonical_bytes(
        data,
        DeltaJournalLimits::default(),
        |entry| AdditiveDelta::from_canonical_bytes(prime_encoder(), entry),
    );

    let namespace = ApplicationNamespace::derive(b"rc7-fuzz-database-v1");
    let schema = database_schema();
    let limits = DatabaseTransactionLimits::default();
    let _ = schema.decode_row(data);
    let _ = TransactionDelta::from_canonical_bytes(namespace, &schema, data, limits);
    let _ = DatabaseTransactionLog::from_canonical_bytes(namespace, &schema, data, limits);

    let profile = FileChunkProfile::fixed(23).expect("constant chunk profile is valid");
    let binary_encoder = BinaryPolynomialEncoder::new(0x5243_7002);
    let summary_base = Gf2_128V1::from_polynomial_bytes_mod(&[2]);
    let _ = HomomorphicSummaryTree::from_checkpoint_bytes(
        profile,
        binary_encoder,
        summary_base,
        data,
        SummaryTreeLimits::default(),
    );

    let reconciler = BoundedSetReconciler::new(ReconciliationLimits::new(64, 6, 8, 1_024))
        .expect("constant reconciliation profile is valid");
    let _ = reconciler.sketch_from_canonical_bytes(data);
});
