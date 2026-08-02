use super::traits::HomomorphicAggregator;
use crate::algebra::traits::FiniteField;

/// Legacy multiset product `H = Product(X_g + phi(x_i))`.
///
/// The field value cannot verify membership and collapses after a zero factor.
/// Prefer [`crate::MultisetSignature`] or [`crate::TrackedMultiset`].
pub struct MultisetAggregator;

impl MultisetAggregator {
    /// Historical affine offset `X_g = x^255`.
    #[inline(always)]
    fn generator_constant<F: FiniteField>() -> F {
        let mut buffer = [0u8; 32];
        buffer[31] = 0x80; // Sets bit 255 to 1, all others 0.
        F::from_bytes_canonical(&buffer)
    }
}

impl<F: FiniteField> HomomorphicAggregator<F> for MultisetAggregator {
    #[inline(always)]
    fn empty_state() -> F {
        F::one()
    }

    fn embed_to_field(data: &[u8]) -> F {
        let mut result = F::zero();
        for chunk in data.chunks(32).rev() {
            let mut buffer = [0u8; 32];
            buffer[..chunk.len()].copy_from_slice(chunk);
            // Historical masking rule retained only for byte compatibility.
            buffer[31] &= 0x7F;
            let block = F::from_bytes_canonical(&buffer);
            result = result.shift_phase().add(&block);
        }
        result
    }

    /// Computes `H = S * (X_g + e)`.
    #[inline(always)]
    fn aggregate(state: &F, new_element: &F, _index: usize) -> F {
        let root = new_element.add(&Self::generator_constant::<F>());
        state.mul(&root)
    }

    /// Derives an algebraic quotient; it cannot establish that the element was
    /// previously inserted.
    fn remove(state: &F, element: &F) -> Option<F> {
        let root = element.add(&Self::generator_constant::<F>());
        root.inv().map(|inverse| state.mul(&inverse))
    }
}
