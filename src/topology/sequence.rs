use super::traits::HomomorphicAggregator;
use crate::algebra::traits::FiniteField;

/// Sequence Topology Aggregator.
/// Models directed causal geometries (e.g., directed paths).
/// Mathematically irreversible for arbitrary indices, reversible for LIFO pops.
pub struct SequenceAggregator;

impl<F: FiniteField> HomomorphicAggregator<F> for SequenceAggregator {
    #[inline(always)]
    fn empty_state() -> F {
        F::zero()
    }

    /// Pure Linear Embedding.
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

    /// Horner's Method: S_{t+1} = (S_t * Phase) + e
    #[inline(always)]
    fn aggregate(state: &F, new_element: &F, _index: usize) -> F {
        state.shift_phase().add(new_element)
    }

    /// Topological Rollback (LIFO Extraction).
    /// S_{t-1} = (S_t + e_{last}) * Phi^{-1}
    fn remove(state: &F, last_element: &F) -> Option<F> {
        let phase = F::one().shift_phase();
        let phase_inv = phase.inv()?;

        let un_added = state.add(last_element);
        Some(un_added.mul(&phase_inv))
    }
}
