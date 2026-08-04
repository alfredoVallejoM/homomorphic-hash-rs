use super::traits::HomomorphicAggregator;
use crate::algebra::traits::FiniteField;

/// Legacy characteristic-two parity accumulator.
///
/// It models encoded multiplicity modulo two, not an exact set: encoder and
/// field collisions remain possible. Prefer [`crate::AdditiveSignature`].
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
        // Historical reverse chunk evaluation retained byte-for-byte.
        for chunk in data.chunks(32).rev() {
            let mut buffer = [0u8; 32];
            buffer[..chunk.len()].copy_from_slice(chunk);
            let block = F::from_bytes_canonical(&buffer);
            result = result.shift_phase().add(&block);
        }
        result
    }

    /// Adds the encoded term in characteristic two.
    #[inline(always)]
    fn aggregate(state: &F, new_element: &F, _index: usize) -> F {
        state.add(new_element)
    }

    /// Applies the same addition to derive an algebraic residual.
    #[inline(always)]
    fn remove(state: &F, element: &F) -> Option<F> {
        Some(state.add(element))
    }
}
