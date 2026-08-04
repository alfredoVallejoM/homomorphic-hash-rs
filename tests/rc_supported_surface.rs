//! Executable contract for the RC.0 supported-surface inventory.

use std::collections::BTreeSet;

use microfield::{
    BinaryPolynomialField, CanonicalEncoding, Field, Fp251V1, Fp256GenericV1, FpGoldilocks64V1,
    Gf2_128V1, Gf2_256AltV1, Gf2_256HhV1, Invert, Pow, Square, StaticField,
};
use serde_json::Value;
use structural_field_fixture::Gf2_9StructuralFixture;

#[cfg(feature = "signatures")]
use homomorphic_hash_rs::{
    AdditiveSignature, BidirectionalSequenceSignature, CanonicalElementEncoder,
    MultiEvaluationMultisetSignature, MultiEvaluationSequenceSignature, MultisetSignature,
    SequenceSignature, SignatureLaw, TrackedMultiset, TrackedSequence,
};

fn assert_static_field_contract<F>()
where
    F: Field + Square + Invert + Pow + CanonicalEncoding + StaticField,
{
}

#[test]
fn rc_inventory_is_well_formed_and_unique() {
    let inventory: Value =
        serde_json::from_str(include_str!("../validation/rc/supported-surface-v1.json"))
            .expect("the frozen RC inventory must be valid JSON");
    assert_eq!(inventory["schema"], "microfield-rc-supported-surface-v1");
    assert_eq!(inventory["release_scope"], "internal-technical-rc");

    let capabilities = inventory["capabilities"]
        .as_array()
        .expect("capabilities must be an array");
    let mut ids = BTreeSet::new();
    let admitted = [
        "supported",
        "conditional",
        "experimental",
        "restricted",
        "planned",
        "legacy-adapter",
        "rejected",
    ];
    for capability in capabilities {
        let id = capability["id"].as_str().expect("capability id");
        let status = capability["status"].as_str().expect("capability status");
        let feature = capability["feature"].as_str().expect("capability feature");
        let contract = capability["contract"]
            .as_str()
            .expect("capability contract");
        assert!(ids.insert(id), "duplicated capability id: {id}");
        assert!(
            admitted.contains(&status),
            "unknown status for {id}: {status}"
        );
        assert!(!feature.is_empty(), "missing feature for {id}");
        assert!(!contract.is_empty(), "missing contract for {id}");
    }

    for required in [
        "field.static.binary",
        "field.generated.binary",
        "field.runtime",
        "signature.additive",
        "signature.sequence",
        "signature.multiset",
        "signature.builder",
        "signature.compact-snapshot",
        "signature.tracked-snapshot",
        "signature.residual",
        "protocol.data-delta",
        "protocol.delta-journal",
        "protocol.file-chunks",
        "protocol.summary-tree",
        "protocol.summary-checkpoint",
        "protocol.reconciliation",
        "protocol.database-rows",
        "protocol.database-transactions",
        "graph.filter",
        "graph.microcanon",
    ] {
        assert!(
            ids.contains(required),
            "missing required capability: {required}"
        );
    }
}

#[test]
fn maintained_binary_fields_satisfy_the_common_rc_contract() {
    assert_static_field_contract::<Gf2_128V1>();
    assert_static_field_contract::<Gf2_256HhV1>();
    assert_static_field_contract::<Gf2_256AltV1>();

    let ids = [
        Gf2_128V1::spec().field_id(),
        Gf2_256HhV1::spec().field_id(),
        Gf2_256AltV1::spec().field_id(),
    ];
    assert_ne!(ids[0], ids[1]);
    assert_ne!(ids[0], ids[2]);
    assert_ne!(ids[1], ids[2]);
}

#[test]
fn maintained_prime_and_generated_fields_satisfy_the_common_rc_contract() {
    assert_static_field_contract::<Fp251V1>();
    assert_static_field_contract::<FpGoldilocks64V1>();
    assert_static_field_contract::<Fp256GenericV1>();
    assert_static_field_contract::<Gf2_9StructuralFixture>();
    assert_eq!(Gf2_9StructuralFixture::MODULUS_DEGREE, 9);

    let ids = [
        Fp251V1::spec().field_id(),
        FpGoldilocks64V1::spec().field_id(),
        Fp256GenericV1::spec().field_id(),
        Gf2_9StructuralFixture::spec().field_id(),
    ];
    for left in 0..ids.len() {
        for right in left + 1..ids.len() {
            assert_ne!(ids[left], ids[right]);
        }
    }
}

#[cfg(feature = "signatures")]
#[test]
fn signatures_feature_exposes_all_maintained_static_families_without_graph_api() {
    let encoder = CanonicalElementEncoder;
    let base_a = Gf2_128V1::from_polynomial_bytes_mod(&[2]);
    let base_b = Gf2_128V1::from_polynomial_bytes_mod(&[3]);

    let additive = AdditiveSignature::<Gf2_128V1, _>::new(encoder);
    let sequence = SequenceSignature::<Gf2_128V1, _>::new(encoder, base_a).unwrap();
    let bidirectional =
        BidirectionalSequenceSignature::<Gf2_128V1, _>::new(encoder, base_a).unwrap();
    let multiset = MultisetSignature::<Gf2_128V1, _>::new(encoder, Gf2_128V1::ONE);
    let multi_multiset = MultiEvaluationMultisetSignature::<Gf2_128V1, _, 2>::new(
        encoder,
        [Gf2_128V1::ZERO, Gf2_128V1::ONE],
    )
    .unwrap();
    let multi_sequence =
        MultiEvaluationSequenceSignature::<Gf2_128V1, _, 2>::new(encoder, [base_a, base_b])
            .unwrap();
    let tracked_sequence = TrackedSequence::<Gf2_128V1, _>::new(encoder, base_a).unwrap();
    let tracked_multiset = TrackedMultiset::<Gf2_128V1, _>::new(encoder, Gf2_128V1::ONE);

    assert_eq!(additive.context().law(), SignatureLaw::Additive);
    assert_eq!(sequence.context().law(), SignatureLaw::Sequence);
    assert_eq!(
        bidirectional.context().law(),
        SignatureLaw::BidirectionalSequence
    );
    assert_eq!(multiset.context().law(), SignatureLaw::Multiset);
    assert_eq!(
        multi_multiset.context().law(),
        SignatureLaw::MultiEvaluationMultiset
    );
    assert_eq!(
        multi_sequence.context().law(),
        SignatureLaw::MultiEvaluationSequence
    );
    assert!(tracked_sequence.assurance().tracks_source_values());
    assert!(tracked_multiset.assurance().tracks_source_values());
}

#[cfg(any(feature = "dynamic-signatures", feature = "dynamic-fields"))]
#[test]
fn dynamic_signatures_feature_exposes_runtime_families_without_graph_requirement() {
    use homomorphic_hash_rs::{
        DynamicAdditiveSignature, DynamicBidirectionalSequenceSignature,
        DynamicMultiEvaluationMultisetSignature, DynamicMultiEvaluationSequenceSignature,
        DynamicMultisetSignature, DynamicSequenceSignature,
    };

    fn type_is_public<T>() {}

    type_is_public::<DynamicAdditiveSignature<CanonicalElementEncoder>>();
    type_is_public::<DynamicSequenceSignature<CanonicalElementEncoder>>();
    type_is_public::<DynamicBidirectionalSequenceSignature<CanonicalElementEncoder>>();
    type_is_public::<DynamicMultisetSignature<CanonicalElementEncoder>>();
    type_is_public::<DynamicMultiEvaluationMultisetSignature<CanonicalElementEncoder>>();
    type_is_public::<DynamicMultiEvaluationSequenceSignature<CanonicalElementEncoder>>();

    let runtime = microfield::DynField::builder("rc_runtime_gf2_9")
        .binary(9, vec![9, 4, 0])
        .build()
        .unwrap();
    assert_eq!(
        runtime.field_id(),
        Gf2_9StructuralFixture::spec().field_id()
    );
}
