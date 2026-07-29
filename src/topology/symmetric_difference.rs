use super::traits::HomomorphicAggregator;
use crate::algebra::traits::FiniteField;

/// Symmetric Difference Aggregator (Boolean Ring).
/// Replaces the mathematically inaccurate "Set" topology.
/// MATHEMATICAL LEMMA: In characteristic 2, A + A = 0.
/// This models the symmetric difference (A Δ B), where intersections are annihilated.
pub struct SymmetricDifferenceAggregator;

impl<F: FiniteField> HomomorphicAggregator<F> for SymmetricDifferenceAggregator {
    #[inline(always)]
    fn empty_state() -> F {
        F::zero()
    }

    /// Linear Embedding using Polynomial Block Evaluation.
    /// Evaluates chunks of 32 bytes as coefficients of a larger polynomial.
    fn embed_to_field(data: &[u8]) -> F {
        let mut result = F::zero();
        // REVERSE ITERATION: Anchors index 0 to Phi^0, preserving length-invariant linearity.
        for chunk in data.chunks(32).rev() {
            let mut buffer = [0u8; 32];
            buffer[..chunk.len()].copy_from_slice(chunk);
            let block = F::from_bytes_canonical(&buffer);
            result = result.shift_phase().add(&block);
        }
        result
    }

    /// Physical Cost: 1 cycle (Vector XOR).
    #[inline(always)]
    fn aggregate(state: &F, new_element: &F, _index: usize) -> F {
        state.add(new_element)
    }

    /// Physical Cost: 1 cycle (Vector XOR).
    #[inline(always)]
    fn remove(state: &F, element: &F) -> Option<F> {
        Some(state.add(element))
    }
}
