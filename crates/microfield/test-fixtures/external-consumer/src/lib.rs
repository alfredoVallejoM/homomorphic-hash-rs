//! External consumer proving the build-time factory contract.

#![no_std]

#[cfg(test)]
extern crate std;

/// Mixing independently generated nominal fields is a type error.
///
/// ```compile_fail
/// use microfield_external_consumer::{Gf2_9Fixture, Gf2_233Fixture};
/// let small = Gf2_9Fixture::default();
/// let large = Gf2_233Fixture::default();
/// let _: Gf2_9Fixture = small + large;
/// ```
///
/// Private limbs cannot be constructed or read by a consumer.
///
/// ```compile_fail
/// use microfield_external_consumer::Gf2_9Fixture;
/// let value = Gf2_9Fixture([1]);
/// ```
pub struct CompileFailContracts;

include!(concat!(env!("OUT_DIR"), "/gf2_9_fixture.rs"));

mod generated_10_dense {
    include!(concat!(env!("OUT_DIR"), "/gf2_10_dense_fixture.rs"));
}

mod generated_233 {
    include!(concat!(env!("OUT_DIR"), "/gf2_233_fixture.rs"));
}

mod generated_128 {
    include!(concat!(env!("OUT_DIR"), "/gf2_128_external_fixture.rs"));
}

mod generated_192 {
    include!(concat!(env!("OUT_DIR"), "/gf2_192_external_fixture.rs"));
}

pub use generated_10_dense::Gf2_10DenseFixture;
pub use generated_128::Gf2_128ExternalFixture;
pub use generated_192::Gf2_192ExternalFixture;
pub use generated_233::Gf2_233Fixture;

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use microfield::{
        BackendId, BinaryPolynomialField, CanonicalEncoding, CpuCapabilities, DecodeError, Engine,
        EngineBuildError, ExecutionPolicy, ExtensionField, Field, Invert, PackedBatch, Pow,
        ScheduleKind, Square, StaticField,
    };
    use std::format;

    use super::{
        Gf2_9Fixture, Gf2_10DenseFixture, Gf2_128ExternalFixture, Gf2_192ExternalFixture,
        Gf2_233Fixture,
    };

    const MODULUS: u32 = (1 << 9) | (1 << 4) | 1;

    fn element(value: u16) -> Gf2_9Fixture {
        Gf2_9Fixture::from_canonical(&value.to_le_bytes()).expect("value is below 2^9")
    }

    fn dense_element(value: u16) -> Gf2_10DenseFixture {
        Gf2_10DenseFixture::from_canonical(&value.to_le_bytes()).expect("value is below 2^10")
    }

    fn reference_multiply(lhs: u16, rhs: u16) -> u16 {
        let mut product = 0_u32;
        for bit in 0..9 {
            if (rhs >> bit) & 1 != 0 {
                product ^= u32::from(lhs) << bit;
            }
        }
        for bit in (9..=16).rev() {
            if product & (1 << bit) != 0 {
                product ^= MODULUS << (bit - 9);
            }
        }
        product as u16
    }

    fn reference_multiply_dense(lhs: u16, rhs: u16) -> u16 {
        const DENSE_MODULUS: u32 = (1 << 11) - 1;
        let mut product = 0_u32;
        for bit in 0..10 {
            if (rhs >> bit) & 1 != 0 {
                product ^= u32::from(lhs) << bit;
            }
        }
        for bit in (10..=18).rev() {
            if product & (1 << bit) != 0 {
                product ^= DENSE_MODULUS << (bit - 10);
            }
        }
        product as u16
    }

    fn hex_233(source: &str) -> Gf2_233Fixture {
        assert_eq!(source.len(), 60);
        let mut bytes = [0_u8; 30];
        for (index, pair) in source.as_bytes().chunks_exact(2).enumerate() {
            bytes[index] = (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]);
        }
        Gf2_233Fixture::from_canonical(&bytes).expect("committed Sage value is canonical")
    }

    fn hex_nibble(value: u8) -> u8 {
        match value {
            b'0'..=b'9' => value - b'0',
            b'a'..=b'f' => value - b'a' + 10,
            _ => panic!("invalid committed hexadecimal"),
        }
    }

    #[test]
    fn exhaustive_encoding_square_and_inverse_laws() {
        for raw in 0_u16..512 {
            let value = element(raw);
            assert_eq!(value.to_canonical(), raw.to_le_bytes());
            assert_eq!(value.square(), element(reference_multiply(raw, raw)));
            assert_eq!(value.frobenius(9), value);
            if raw == 0 {
                assert_eq!(value.invert(), None);
            } else {
                assert_eq!(
                    value * value.invert().expect("non-zero inverse"),
                    Gf2_9Fixture::ONE
                );
            }
        }
    }

    #[test]
    fn multiplication_matches_an_independent_model() {
        for lhs in 0_u16..512 {
            for rhs in (0_u16..512).step_by(17) {
                assert_eq!(
                    element(lhs) * element(rhs),
                    element(reference_multiply(lhs, rhs)),
                    "lhs={lhs:#x}, rhs={rhs:#x}"
                );
            }
        }
    }

    #[test]
    fn dense_generated_reduction_matches_an_independent_model() {
        let lhs_limit = if cfg!(miri) { 64 } else { 1024 };
        let rhs_step = if cfg!(miri) { 127 } else { 31 };
        for lhs in 0_u16..lhs_limit {
            for rhs in (0_u16..1024).step_by(rhs_step) {
                assert_eq!(
                    dense_element(lhs) * dense_element(rhs),
                    dense_element(reference_multiply_dense(lhs, rhs)),
                    "lhs={lhs:#x}, rhs={rhs:#x}"
                );
            }
            let value = dense_element(lhs);
            assert_eq!(
                value.square(),
                dense_element(reference_multiply_dense(lhs, lhs))
            );
            if lhs == 0 {
                assert_eq!(value.invert(), None);
            } else {
                assert_eq!(
                    value * value.invert().expect("non-zero inverse"),
                    Gf2_10DenseFixture::ONE
                );
            }
        }
    }

    #[test]
    fn polynomial_reduction_accepts_inputs_far_wider_than_the_field() {
        let bytes = [0x35, 0xa7, 0xfe, 0x18, 0x91, 0x44, 0xff];
        let mut expected = Gf2_9Fixture::ZERO;
        for byte in bytes.iter().rev().copied() {
            for bit in (0..8).rev() {
                expected = expected.mul_by_x();
                if (byte >> bit) & 1 != 0 {
                    expected += Gf2_9Fixture::ONE;
                }
            }
        }
        assert_eq!(Gf2_9Fixture::from_polynomial_bytes_mod(&bytes), expected);
    }

    #[test]
    fn decoder_rejects_length_and_every_nonzero_padding_pattern() {
        assert_eq!(
            Gf2_9Fixture::from_canonical_slice(&[0]),
            Err(DecodeError::LengthMismatch {
                expected: 2,
                actual: 1
            })
        );
        for high in 2_u8..=u8::MAX {
            assert_eq!(
                Gf2_9Fixture::from_canonical(&[0, high]),
                Err(DecodeError::NonCanonicalValue)
            );
        }
    }

    #[test]
    fn external_field_uses_the_public_batch_facade() {
        let engine = Engine::<Gf2_9Fixture>::builder()
            .policy(ExecutionPolicy::Throughput)
            .expected_batch(3)
            .capabilities(CpuCapabilities::portable_only())
            .build()
            .expect("ABI 3 keeps explicit-only ISA profiles off automatic selection");
        assert_eq!(engine.backend_id(), BackendId::Portable);
        let lhs = [element(3), element(0x101), element(0x1ff)];
        let rhs = [element(7), element(0x55), element(0x101)];
        let mut out = [Gf2_9Fixture::ZERO; 3];
        engine
            .mul_into(&mut out, &lhs, &rhs)
            .expect("matching lengths");
        for index in 0..out.len() {
            assert_eq!(out[index], lhs[index] * rhs[index]);
        }

        let detected = Engine::<Gf2_9Fixture>::builder()
            .policy(ExecutionPolicy::Throughput)
            .expected_batch(3)
            .detect()
            .expect("explicit-only profiles leave portable available");
        assert_eq!(detected.backend_id(), BackendId::Portable);

        assert_current_architecture_profile(&lhs, &rhs);
    }

    fn assert_current_architecture_profile(lhs: &[Gf2_9Fixture], rhs: &[Gf2_9Fixture]) {
        #[cfg(target_arch = "x86_64")]
        let backend = BackendId::X86Pclmul;
        #[cfg(target_arch = "aarch64")]
        let backend = BackendId::Aarch64Pmull;
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        let backend = BackendId::X86Pclmul;

        let selected = Engine::<Gf2_9Fixture>::builder()
            .force_backend(backend)
            .detect();
        #[cfg(target_arch = "x86_64")]
        let available = std::arch::is_x86_feature_detected!("pclmulqdq");
        #[cfg(target_arch = "aarch64")]
        let available = std::arch::is_aarch64_feature_detected!("neon")
            && std::arch::is_aarch64_feature_detected!("pmull");
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        let available = false;

        if !available {
            let expected = if cfg!(any(target_arch = "x86_64", target_arch = "aarch64")) {
                EngineBuildError::BackendUnsupportedByCpu(backend)
            } else {
                EngineBuildError::BackendNotCompiled(backend)
            };
            assert!(matches!(selected, Err(error) if error == expected));
            return;
        }

        let isa = selected.expect("the verified profile supports the current ISA");
        assert_eq!(isa.backend_id(), backend);
        assert!(!isa.metadata().automatic_selection());
        let mut out = [Gf2_9Fixture::ZERO; 3];
        isa.mul_into(&mut out, lhs, rhs).expect("matching lengths");
        for index in 0..out.len() {
            assert_eq!(out[index], lhs[index] * rhs[index]);
        }
        isa.square_assign(&mut out);
        for index in 0..out.len() {
            assert_eq!(out[index], (lhs[index] * rhs[index]).square());
        }
    }

    #[test]
    fn power_of_two_external_profile_is_nominal_and_field_correct() {
        assert_eq!(Gf2_128ExternalFixture::spec().degree(), 128);
        let mut lhs_bytes = [0_u8; 16];
        let mut rhs_bytes = [0_u8; 16];
        for index in 0..16 {
            lhs_bytes[index] = (index as u8).wrapping_mul(37).wrapping_add(3);
            rhs_bytes[index] = (index as u8).wrapping_mul(91).wrapping_add(7);
        }
        let lhs = Gf2_128ExternalFixture::from_canonical(&lhs_bytes).expect("full-width value");
        let rhs = Gf2_128ExternalFixture::from_canonical(&rhs_bytes).expect("full-width value");
        assert_eq!(lhs * rhs, rhs * lhs);
        assert_eq!(lhs.square(), lhs * lhs);
        assert_eq!(
            lhs * lhs.invert().expect("sample is non-zero"),
            Gf2_128ExternalFixture::ONE
        );
    }

    #[test]
    fn verified_isa_bridge_covers_every_generated_reduction_class() {
        assert_profile_batch(
            &[element(0), element(1), element(0x101), element(0x1ff)],
            &[element(7), element(0x55), element(0x101), element(3)],
            ScheduleKind::DataDependent,
        );
        assert_profile_batch(
            &[
                dense_element(0),
                dense_element(1),
                dense_element(0x201),
                dense_element(0x3ff),
            ],
            &[
                dense_element(7),
                dense_element(0x155),
                dense_element(0x201),
                dense_element(3),
            ],
            ScheduleKind::DataDependent,
        );

        let mut a = [0_u8; 16];
        let mut b = [0_u8; 16];
        for index in 0..16 {
            a[index] = (index as u8).wrapping_mul(37).wrapping_add(3);
            b[index] = (index as u8).wrapping_mul(91).wrapping_add(7);
        }
        let a = Gf2_128ExternalFixture::from_canonical(&a).expect("full-width value");
        let b = Gf2_128ExternalFixture::from_canonical(&b).expect("full-width value");
        assert_profile_batch(
            &[Gf2_128ExternalFixture::ZERO, a, b, a + b],
            &[b, a, a + b, a],
            ScheduleKind::Fixed,
        );

        let mut a = [0_u8; 24];
        let mut b = [0_u8; 24];
        for index in 0..24 {
            a[index] = (index as u8).wrapping_mul(53).wrapping_add(11);
            b[index] = (index as u8).wrapping_mul(79).wrapping_add(5);
        }
        let a = Gf2_192ExternalFixture::from_canonical(&a).expect("full-width value");
        let b = Gf2_192ExternalFixture::from_canonical(&b).expect("full-width value");
        assert_profile_batch(
            &[Gf2_192ExternalFixture::ZERO, a, b, a + b],
            &[b, a, a + b, a],
            ScheduleKind::Fixed,
        );

        let a = hex_233("cd240c95c64a5798e1632bccb09f98c5667208e4dbf6180232d0a67d2701");
        let b = hex_233("51b3180bb22ec32b52fee524ba63353058344218073f87761795005e1c00");
        assert_profile_batch(
            &[Gf2_233Fixture::ZERO, a, b, a + b],
            &[b, a, a + b, a],
            ScheduleKind::DataDependent,
        );
    }

    fn assert_profile_batch<F>(lhs: &[F], rhs: &[F], expected_schedule: ScheduleKind)
    where
        F: microfield::__private::PortableField + StaticField + core::fmt::Debug,
    {
        let portable = Engine::<F>::portable();
        let mut expected = std::vec![F::ZERO; lhs.len()];
        portable
            .mul_into(&mut expected, lhs, rhs)
            .expect("equal lengths");
        let packed_lhs = PackedBatch::from_aos(&portable, lhs).expect("external packed lhs");
        let packed_rhs = PackedBatch::from_aos(&portable, rhs).expect("external packed rhs");
        let mut packed_out = PackedBatch::new(&portable, lhs.len()).expect("external packed out");
        portable
            .mul_packed_into(&mut packed_out, &packed_lhs, &packed_rhs)
            .expect("matching external plans");
        let mut packed_actual = std::vec![F::ZERO; lhs.len()];
        packed_out
            .unpack_into(&mut packed_actual)
            .expect("matching external output");
        assert_eq!(packed_actual, expected);

        #[cfg(target_arch = "x86_64")]
        {
            if std::arch::is_x86_feature_detected!("pclmulqdq") {
                assert_profile_backend(BackendId::X86Pclmul, lhs, rhs, expected_schedule, portable);
            }
            if std::arch::is_x86_feature_detected!("pclmulqdq")
                && std::arch::is_x86_feature_detected!("avx2")
                && std::arch::is_x86_feature_detected!("vpclmulqdq")
            {
                assert_profile_backend(
                    BackendId::X86Vpclmul,
                    lhs,
                    rhs,
                    expected_schedule,
                    portable,
                );
            }
        }
        #[cfg(target_arch = "aarch64")]
        if std::arch::is_aarch64_feature_detected!("neon")
            && std::arch::is_aarch64_feature_detected!("pmull")
        {
            assert_profile_backend(
                BackendId::Aarch64Pmull,
                lhs,
                rhs,
                expected_schedule,
                portable,
            );
        }
    }

    fn assert_profile_backend<F>(
        backend: BackendId,
        lhs: &[F],
        rhs: &[F],
        expected_schedule: ScheduleKind,
        portable: Engine<F>,
    ) where
        F: microfield::__private::PortableField + StaticField + core::fmt::Debug,
    {
        let isa = Engine::<F>::builder()
            .force_backend(backend)
            .detect()
            .expect("the generated profile registers a current-target strategy");
        assert!(!isa.metadata().automatic_selection());
        assert_eq!(isa.metadata().schedule(), expected_schedule);

        let fixed = Engine::<F>::builder()
            .policy(ExecutionPolicy::FixedSchedule)
            .force_backend(backend)
            .detect();
        if expected_schedule == ScheduleKind::Fixed {
            assert_eq!(
                fixed
                    .expect("a fixed generated reduction satisfies the policy")
                    .backend_id(),
                backend
            );
        } else {
            assert!(matches!(
                fixed,
                Err(EngineBuildError::PolicyUnsatisfied(
                    ExecutionPolicy::FixedSchedule
                ))
            ));
        }

        let mut expected = std::vec![F::ZERO; lhs.len()];
        portable
            .mul_into(&mut expected, lhs, rhs)
            .expect("equal lengths");
        let mut actual = std::vec![F::ZERO; lhs.len()];
        isa.mul_into(&mut actual, lhs, rhs).expect("equal lengths");
        assert_eq!(actual, expected);

        let isa_lhs = PackedBatch::from_aos(&isa, lhs).expect("external ISA packed lhs");
        let isa_rhs = PackedBatch::from_aos(&isa, rhs).expect("external ISA packed rhs");
        let mut isa_out = PackedBatch::new(&isa, lhs.len()).expect("external ISA packed out");
        isa.mul_packed_into(&mut isa_out, &isa_lhs, &isa_rhs)
            .expect("matching external ISA plans");
        isa_out
            .unpack_into(&mut actual)
            .expect("matching external ISA output");
        assert_eq!(actual, expected);

        portable
            .square_into(&mut expected, lhs)
            .expect("equal lengths");
        isa.square_into(&mut actual, lhs).expect("equal lengths");
        assert_eq!(actual, expected);

        let mut expected_assign = lhs.to_vec();
        let mut actual_assign = lhs.to_vec();
        portable
            .mul_assign(&mut expected_assign, rhs)
            .expect("equal lengths");
        isa.mul_assign(&mut actual_assign, rhs)
            .expect("equal lengths");
        assert_eq!(actual_assign, expected_assign);
        isa.square_assign(&mut actual_assign);
        for (actual, product) in actual_assign.iter().zip(expected_assign) {
            assert_eq!(*actual, product.square());
        }
    }

    #[test]
    fn layout_metadata_and_formatting_are_stable() {
        assert_eq!(size_of::<Gf2_9Fixture>(), 8);
        assert_eq!(align_of::<Gf2_9Fixture>(), 8);
        assert_eq!(Gf2_9Fixture::spec().degree(), 9);
        assert_eq!(Gf2_9Fixture::spec().canonical_bytes(), 2);
        assert_eq!(Gf2_9Fixture::spec().name(), "gf2_9_fixture");
        assert_eq!(format!("{}", element(1)), "0001");
        assert_eq!(format!("{:?}", element(1)), "Gf2_9Fixture(0x0001)");
    }

    #[test]
    fn multi_limb_degree_233_field_obeys_boundary_and_field_laws() {
        let mut left_bytes = [0_u8; 30];
        let mut right_bytes = [0_u8; 30];
        for index in 0..30 {
            left_bytes[index] = (index as u8).wrapping_mul(37).wrapping_add(11);
            right_bytes[index] = (index as u8).wrapping_mul(91).wrapping_add(7);
        }
        left_bytes[29] &= 1;
        right_bytes[29] &= 1;
        let left = Gf2_233Fixture::from_canonical(&left_bytes).expect("canonical sample");
        let right = Gf2_233Fixture::from_canonical(&right_bytes).expect("canonical sample");

        assert_eq!(left * Gf2_233Fixture::ONE, left);
        assert_eq!(left * right, right * left);
        assert_eq!(left.square(), left * left);
        assert_eq!(
            left * left.invert().expect("sample is non-zero"),
            Gf2_233Fixture::ONE
        );
        assert_eq!(left.frobenius(233), left);

        let mut highest_basis = [0_u8; 30];
        highest_basis[29] = 1;
        let wrapped = Gf2_233Fixture::from_canonical(&highest_basis)
            .expect("x^232 is canonical")
            .mul_by_x()
            .to_canonical();
        let mut expected_tail = [0_u8; 30];
        expected_tail[0] = 1;
        expected_tail[9] = 1 << 2;
        assert_eq!(wrapped, expected_tail);

        assert_eq!(size_of::<Gf2_233Fixture>(), 32);
        assert_eq!(align_of::<Gf2_233Fixture>(), 8);
        assert_eq!(Gf2_233Fixture::spec().degree(), 233);
        let mut padding = [0_u8; 30];
        padding[29] = 2;
        assert_eq!(
            Gf2_233Fixture::from_canonical(&padding),
            Err(DecodeError::NonCanonicalValue)
        );
    }

    #[test]
    fn degree_233_matches_committed_sage_10_7_vectors() {
        // Generated with tools/sage/generate_vectors.sage under laboratorio_np.
        let lhs = hex_233("cd240c95c64a5798e1632bccb09f98c5667208e4dbf6180232d0a67d2701");
        let rhs = hex_233("51b3180bb22ec32b52fee524ba63353058344218073f87761795005e1c00");
        assert_eq!(
            lhs + rhs,
            hex_233("9c97149e746494b3b39dcee80afcadf53e464afcdcc99f742545a6233b01")
        );
        assert_eq!(
            lhs * rhs,
            hex_233("94c5b35faad8c45d7bb5bd2a2434366d918c3c4f91523222ffb8677a9900")
        );
        assert_eq!(
            lhs.square(),
            hex_233("f458691a1ed30e7410cdcc5644a3279ed546213ecd2650f022efd74bca01")
        );
        assert_eq!(
            lhs.invert().expect("non-zero Sage operand"),
            hex_233("3b0152cfc484bd2414dd08e49e50590115a31db9987ff3b39dc389a0d301")
        );
        assert_eq!(
            rhs.pow(&[65_537]),
            hex_233("0a536c6545f7739f0b7e042b3a7c4c1dedc9a5fd89ca662aed2abbff5800")
        );
        assert_eq!(
            lhs.mul_by_x(),
            hex_233("9b49182a8d95ae30c3c35698613f318bcde410c8b7ed310464a04dfb4e00")
        );
    }
}
