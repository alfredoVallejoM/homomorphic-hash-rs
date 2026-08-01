//! Pure selection of portable arithmetic strategies.

use crate::spec::model::{
    PortableDegreeClass, PortableOptimizationPlan, PortableReductionStrategy, ValidatedFieldSpec,
};

/// Minimum sparse-term budget, independent of field size.
const MINIMUM_SPARSE_TERM_BUDGET: usize = 8;

/// Maximum sparse terms accepted per 64-bit element limb.
const SPARSE_TERMS_PER_LIMB: usize = 2;

/// Highest tail degree supported by the bounded aligned fold.
const LOW_TAIL_MAXIMUM_DEGREE: usize = 32;

/// Deterministic selector for allocation-free portable arithmetic.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct PortableOptimizer;

impl PortableOptimizer {
    /// Derives one immutable plan exclusively from certified field data.
    pub(crate) fn plan(validated: &ValidatedFieldSpec) -> PortableOptimizationPlan {
        let descriptor = validated.normalized().descriptor();
        select(descriptor.degree(), descriptor.modulus_exponents())
    }
}

fn select(degree: usize, modulus: &[usize]) -> PortableOptimizationPlan {
    let limbs = degree.div_ceil(64);
    let tail = &modulus[1..];
    let degree_class = if degree.is_power_of_two() && degree.is_multiple_of(64) {
        PortableDegreeClass::PowerOfTwoLimbAligned
    } else if degree.is_multiple_of(64) {
        PortableDegreeClass::LimbAligned
    } else {
        PortableDegreeClass::Unaligned
    };
    let sparse_budget = MINIMUM_SPARSE_TERM_BUDGET.max(limbs * SPARSE_TERMS_PER_LIMB);
    let reduction = if degree.is_multiple_of(64)
        && tail
            .first()
            .is_some_and(|highest| *highest <= LOW_TAIL_MAXIMUM_DEGREE)
    {
        PortableReductionStrategy::LowTailFold
    } else if tail.len() <= sparse_budget {
        PortableReductionStrategy::SparseTermFold
    } else {
        PortableReductionStrategy::DenseWordFold
    };

    PortableOptimizationPlan::new(degree_class, reduction, modulus.len())
}

#[cfg(test)]
mod tests {
    use super::select;
    use crate::spec::model::{PortableDegreeClass, PortableReductionStrategy};

    #[test]
    fn prioritizes_power_of_two_aligned_low_tail_fields() {
        let plan = select(128, &[128, 7, 2, 1, 0]);
        assert_eq!(
            plan.degree_class(),
            PortableDegreeClass::PowerOfTwoLimbAligned
        );
        assert_eq!(plan.reduction(), PortableReductionStrategy::LowTailFold);
    }

    #[test]
    fn keeps_standard_unaligned_trinomials_on_the_sparse_path() {
        let plan = select(233, &[233, 74, 0]);
        assert_eq!(plan.degree_class(), PortableDegreeClass::Unaligned);
        assert_eq!(plan.reduction(), PortableReductionStrategy::SparseTermFold);
    }

    #[test]
    fn dense_moduli_use_word_folding_without_affecting_degree_class() {
        let mut modulus = vec![257];
        modulus.extend((0..=32).rev());
        let plan = select(257, &modulus);
        assert_eq!(plan.degree_class(), PortableDegreeClass::Unaligned);
        assert_eq!(plan.reduction(), PortableReductionStrategy::DenseWordFold);
    }
}
