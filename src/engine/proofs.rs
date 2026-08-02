//! Legacy algebraic residual API.
//!
//! This module intentionally makes no cryptographic or membership claim. For
//! every non-zero factor in a field, division can manufacture a remainder that
//! recomposes the original state. New code should prefer
//! [`crate::structural::AlgebraicResidual`], whose identity and counters are
//! explicit.

use crate::algebra::traits::FiniteField;
use crate::topology::traits::HomomorphicAggregator;

/// Constant-size remainder of one legacy aggregation equation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TopologicalWitness<F: FiniteField> {
    /// State which recomposes with the candidate under the selected adapter.
    pub state_remainder: F,
}

/// Preferred name for [`TopologicalWitness`].
pub type AlgebraicRemainder<F> = TopologicalWitness<F>;

/// Derives algebraic remainders under a legacy aggregation law.
pub struct ResidualGenerator;

impl ResidualGenerator {
    /// Divides or subtracts the candidate according to the adapter.
    ///
    /// Success means only that the adapter's inverse operation is defined. It
    /// does not establish historical membership or sequence position.
    pub fn derive<F: FiniteField, A: HomomorphicAggregator<F>>(
        macro_state: &F,
        element_data: &[u8],
    ) -> Option<AlgebraicRemainder<F>> {
        let embedded_element = A::embed_to_field(element_data);
        let remainder = A::remove(macro_state, &embedded_element)?;
        Some(TopologicalWitness {
            state_remainder: remainder,
        })
    }
}

/// Checks the forward equation represented by a legacy residual.
pub struct ResidualVerifier;

impl ResidualVerifier {
    /// Recomposes `remainder ⊙ candidate` and compares it with `macro_state`.
    ///
    /// A `true` result validates this equation only. In particular, a caller
    /// can derive such an equation for candidates that were never inserted
    /// whenever the inverse exists.
    pub fn verify_equation<F: FiniteField, A: HomomorphicAggregator<F>>(
        macro_state: &F,
        element_data: &[u8],
        remainder: &AlgebraicRemainder<F>,
        chronological_index: usize,
    ) -> bool {
        let embedded_element = A::embed_to_field(element_data);
        A::aggregate(
            &remainder.state_remainder,
            &embedded_element,
            chronological_index,
        ) == *macro_state
    }
}

/// Compatibility facade retaining the historical name.
pub struct ProofGenerator;

impl ProofGenerator {
    /// Legacy alias of [`ResidualGenerator::derive`].
    ///
    /// Despite its historical name, the result is not an inclusion proof.
    pub fn generate_inclusion_proof<F: FiniteField, A: HomomorphicAggregator<F>>(
        macro_state: &F,
        element_data: &[u8],
    ) -> Option<TopologicalWitness<F>> {
        ResidualGenerator::derive::<F, A>(macro_state, element_data)
    }
}

/// Compatibility facade retaining the historical name.
pub struct ProofVerifier;

impl ProofVerifier {
    /// Legacy alias of [`ResidualVerifier::verify_equation`].
    ///
    /// Despite its historical name, `true` does not prove membership.
    pub fn verify_inclusion<F: FiniteField, A: HomomorphicAggregator<F>>(
        macro_state: &F,
        element_data: &[u8],
        witness: &TopologicalWitness<F>,
        chronological_index: usize,
    ) -> bool {
        ResidualVerifier::verify_equation::<F, A>(
            macro_state,
            element_data,
            witness,
            chronological_index,
        )
    }

    /// Feature-compatible equation check using characteristic-two equality.
    ///
    /// The feature and method are retained only for source compatibility; no
    /// timing or side-channel guarantee is made.
    #[cfg(feature = "crypto_mode")]
    pub fn verify_inclusion_isochronous<F: FiniteField, A: HomomorphicAggregator<F>>(
        macro_state: &F,
        element_data: &[u8],
        witness: &TopologicalWitness<F>,
        chronological_index: usize,
    ) -> bool {
        let embedded_element = A::embed_to_field(element_data);
        let reconstructed_state = A::aggregate(
            &witness.state_remainder,
            &embedded_element,
            chronological_index,
        );
        reconstructed_state.add(macro_state) == F::zero()
    }
}
