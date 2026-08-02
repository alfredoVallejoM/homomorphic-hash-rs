//! Internal static contracts consumed by maintained prime-field types.

use crate::{PrimeField, PrimeReductionPlan, PrimeRepresentationKind, StaticField};

/// Generated constants needed by generic prime algorithms.
///
/// This trait is deliberately crate-private: concrete public types preserve
/// their exact layout and consumers never receive raw modulus limbs or private
/// representation data.
pub(crate) trait PrimeFieldSpec: PrimeField + StaticField {
    const LIMBS: usize;
    const MODULUS: &'static [u64];
    const REPRESENTATION: PrimeRepresentationKind;
    const REDUCTION: PrimeReductionPlan;
}

/// Separates formation of a double-width product from modular reduction.
pub(crate) trait PrimeWideProduct: PrimeField {
    type Wide;

    fn mul_wide(self, rhs: Self) -> Self::Wide;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Fp251V1, Fp256GenericV1, FpGoldilocks64V1};

    fn assert_spec<F: PrimeFieldSpec>(limbs: usize, representation: PrimeRepresentationKind) {
        assert_eq!(F::LIMBS, limbs);
        assert_eq!(F::MODULUS.len(), limbs);
        assert_eq!(F::REPRESENTATION, representation);
        let _ = F::REDUCTION;
    }

    #[test]
    fn maintained_types_satisfy_one_static_internal_contract() {
        assert_spec::<Fp251V1>(1, PrimeRepresentationKind::CanonicalResidue);
        assert_spec::<FpGoldilocks64V1>(1, PrimeRepresentationKind::CanonicalResidue);
        assert_spec::<Fp256GenericV1>(
            4,
            PrimeRepresentationKind::Montgomery {
                radix_bits: 64,
                limbs: 4,
            },
        );
    }
}
