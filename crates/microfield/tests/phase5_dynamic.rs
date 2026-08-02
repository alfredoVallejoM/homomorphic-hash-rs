//! Phase 5 differential and adversarial tests for dynamic contexts.

#![cfg(feature = "dynamic")]

use microfield::{
    DynBatch, DynBatchError, DynField, DynFieldError, DynValidationLimits, Fp251V1,
    PocklingtonCertificate, StaticField, ValidationAssurance,
};

fn encode(field: &DynField, value: &microfield::DynElement) -> Vec<u8> {
    let mut bytes = vec![0_u8; field.canonical_bytes()];
    field.encode(value, &mut bytes).unwrap();
    bytes
}

#[test]
fn dynamic_prime_identity_matches_maintained_static_presentation() {
    let field = DynField::builder("renamed_fp251")
        .prime("251")
        .build()
        .unwrap();
    assert_eq!(field.field_id(), Fp251V1::spec().field_id());
    assert_eq!(field.assurance(), ValidationAssurance::Proven);
    assert_eq!(field.canonical_bytes(), 1);
}

#[test]
fn fp251_dynamic_arithmetic_is_exhaustive() {
    let field = DynField::builder("fp251_dynamic")
        .prime("251")
        .build()
        .unwrap();
    for lhs in 0_u16..251 {
        let left = field.decode(&[u8::try_from(lhs).unwrap()]).unwrap();
        for rhs in 0_u16..251 {
            let right = field.decode(&[u8::try_from(rhs).unwrap()]).unwrap();
            assert_eq!(
                encode(&field, &field.add(&left, &right).unwrap()),
                [((lhs + rhs) % 251) as u8]
            );
            assert_eq!(
                encode(&field, &field.mul(&left, &right).unwrap()),
                [((lhs * rhs) % 251) as u8]
            );
        }
        if lhs != 0 {
            let inverse = field.invert(&left).unwrap();
            assert_eq!(field.mul(&left, &inverse).unwrap(), field.one());
        }
    }
    assert!(matches!(
        field.decode(&[251]),
        Err(DynFieldError::NonCanonicalValue)
    ));
}

#[test]
fn binary_aes_field_is_exhaustively_closed_and_invertible() {
    let field = DynField::builder("gf2_8_aes")
        .binary(8, vec![8, 4, 3, 1, 0])
        .build()
        .unwrap();
    for value in 1_u16..=255 {
        let element = field.decode(&[u8::try_from(value).unwrap()]).unwrap();
        let inverse = field.invert(&element).unwrap();
        assert_eq!(field.mul(&element, &inverse).unwrap(), field.one());
        assert_eq!(
            field.square(&element).unwrap(),
            field.mul(&element, &element).unwrap()
        );
    }
    assert_eq!(field.add(&field.one(), &field.one()).unwrap(), field.zero());
}

#[test]
fn scalar_and_batch_field_mismatches_are_rejected_atomically() {
    let fp251 = DynField::builder("fp251_a").prime("251").build().unwrap();
    let fp257 = DynField::builder("fp257_b").prime("257").build().unwrap();
    let left = fp251.decode(&[7]).unwrap();
    let foreign = fp257.decode(&[7, 0]).unwrap();
    assert!(matches!(
        fp251.mul(&left, &foreign),
        Err(DynFieldError::FieldMismatch { .. })
    ));

    let lhs = DynBatch::from_elements(&fp251, std::slice::from_ref(&left)).unwrap();
    let rhs = DynBatch::from_elements(&fp257, &[foreign]).unwrap();
    let mut out = DynBatch::from_elements(&fp251, &[fp251.decode(&[99]).unwrap()]).unwrap();
    let before = encode(&fp251, &out.element(0).unwrap());
    assert_eq!(
        fp251.engine().mul_into(&mut out, &lhs, &rhs),
        Err(DynBatchError::FieldMismatch)
    );
    assert_eq!(encode(&fp251, &out.element(0).unwrap()), before);
}

#[test]
fn dynamic_batch_matches_scalar_and_inverts_with_one_context_check() {
    let field = DynField::builder("fp65521_dynamic")
        .prime("65521")
        .build()
        .unwrap();
    let values = (1_u16..=129)
        .map(|value| field.decode(&value.to_le_bytes()).unwrap())
        .collect::<Vec<_>>();
    let batch = DynBatch::from_elements(&field, &values).unwrap();
    let mut squared = DynBatch::zeroed(&field, values.len());
    field.engine().square_into(&mut squared, &batch).unwrap();
    for (index, value) in values.iter().enumerate() {
        assert_eq!(
            squared.element(index).unwrap(),
            field.square(value).unwrap()
        );
    }

    let mut inverses = DynBatch::zeroed(&field, values.len());
    field
        .engine()
        .invert_batch_into(&mut inverses, &batch)
        .unwrap();
    let mut products = DynBatch::zeroed(&field, values.len());
    field
        .engine()
        .mul_into(&mut products, &batch, &inverses)
        .unwrap();
    for index in 0..values.len() {
        assert_eq!(products.element(index).unwrap(), field.one());
    }
}

#[test]
fn inline_storage_boundary_is_observable_without_exposing_limbs() {
    let inline = DynField::builder("gf2_256_inline")
        .binary(256, vec![256, 10, 5, 2, 0])
        .build()
        .unwrap();
    assert!(inline.one().storage().is_inline());
    assert_eq!(inline.one().storage().limb_count(), 4);

    // The NIST B-571 polynomial is irreducible and requires nine limbs.
    let heap = DynField::builder("gf2_571_heap")
        .binary(571, vec![571, 10, 5, 2, 0])
        .build()
        .unwrap();
    assert!(!heap.one().storage().is_inline());
    assert_eq!(heap.one().storage().limb_count(), 9);
}

#[test]
fn probable_prime_assurance_survives_manifest_export() {
    let field = DynField::builder("mersenne127_runtime")
        .prime("170141183460469231731687303715884105727")
        .assurance(ValidationAssurance::ProbablePrime { rounds: 32 })
        .build()
        .unwrap();
    assert_eq!(
        field.assurance(),
        ValidationAssurance::ProbablePrime { rounds: 32 }
    );
    let exported = field.export_manifest();
    assert!(exported.contains("assurance = \"probable_prime\""));
    assert!(exported.contains("rounds = 32"));
}

#[test]
fn probable_prime_rejects_a_semantically_conflicting_certificate() {
    let certificate = PocklingtonCertificate {
        algorithm: "pocklington-v1".to_owned(),
        known_factor_product: "2".to_owned(),
        cofactor: "1".to_owned(),
        factors: Vec::new(),
    };
    assert!(matches!(
        DynField::builder("probable_with_certificate")
            .prime("170141183460469231731687303715884105727")
            .assurance(ValidationAssurance::ProbablePrime { rounds: 32 })
            .pocklington_certificate(certificate)
            .build(),
        Err(DynFieldError::InvalidDefinition(_))
    ));
}

#[test]
fn zero_in_batch_inversion_preserves_the_output() {
    let field = DynField::builder("fp257_zero_case")
        .prime("257")
        .build()
        .unwrap();
    let input = DynBatch::from_elements(&field, &[field.one(), field.zero()]).unwrap();
    let marker = field.decode(&[42, 0]).unwrap();
    let mut out = DynBatch::from_elements(&field, &[marker.clone(), marker.clone()]).unwrap();
    assert!(matches!(
        field.engine().invert_batch_into(&mut out, &input),
        Err(DynBatchError::Arithmetic(DynFieldError::DivisionByZero))
    ));
    assert_eq!(out.element(0).unwrap(), marker);
    assert_eq!(out.element(1).unwrap(), marker);
}

#[test]
fn length_and_validation_limits_fail_before_output_mutation() {
    let field = DynField::builder("fp251_lengths")
        .prime("251")
        .build()
        .unwrap();
    let mut wrong = [0xa5_u8; 2];
    assert!(matches!(
        field.encode(&field.one(), &mut wrong),
        Err(DynFieldError::LengthMismatch { .. })
    ));
    assert_eq!(wrong, [0xa5; 2]);

    let lhs = DynBatch::from_elements(&field, &[field.one(), field.one()]).unwrap();
    let rhs = DynBatch::from_elements(&field, &[field.one()]).unwrap();
    let marker = field.decode(&[42]).unwrap();
    let mut out = DynBatch::from_elements(&field, &[marker.clone(), marker.clone()]).unwrap();
    assert!(matches!(
        field.engine().add_into(&mut out, &lhs, &rhs),
        Err(DynBatchError::LengthMismatch { .. })
    ));
    assert_eq!(out.element(0).unwrap(), marker);

    let limits = DynValidationLimits {
        maximum_degree: 7,
        ..DynValidationLimits::default()
    };
    assert!(matches!(
        DynField::builder("gf2_8_too_large")
            .binary(8, vec![8, 4, 3, 1, 0])
            .limits(limits)
            .build(),
        Err(DynFieldError::LimitExceeded { .. })
    ));
    assert!(
        DynField::builder("weak_probable")
            .prime("170141183460469231731687303715884105727")
            .assurance(ValidationAssurance::ProbablePrime { rounds: 4 })
            .build()
            .is_err()
    );
}
