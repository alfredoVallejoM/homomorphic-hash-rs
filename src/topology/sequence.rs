use super::traits::HomomorphicAggregator;
use crate::algebra::traits::FiniteField;

/// Legacy Horner sequence accumulator.
///
/// It omits length and ignores the explicit index. Prefer
/// [`crate::SequenceSignature`], which records length and supports exact
/// concatenation, or [`crate::TrackedSequence`] for checked `pop`.
pub struct SequenceAggregator;

impl<F: FiniteField> HomomorphicAggregator<F> for SequenceAggregator {
    #[inline(always)]
    fn empty_state() -> F {
        F::zero()
    }

    /// Pure Linear Embedding.
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

    /// Horner step `S_(t+1) = S_t * phase + e`.
    #[inline(always)]
    fn aggregate(state: &F, new_element: &F, _index: usize) -> F {
        state.shift_phase().add(new_element)
    }

    /// Derives `S_(t-1) = (S_t + e_assumed) * Phi^-1`.
    ///
    /// The compact state cannot establish that the supplied element was last.
    fn remove(state: &F, last_element: &F) -> Option<F> {
        let phase = F::one().shift_phase();
        let phase_inv = phase.inv()?;

        let un_added = state.add(last_element);
        Some(un_added.mul(&phase_inv))
    }
}
