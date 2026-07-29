use super::traits::HomomorphicAggregator;
use crate::algebra::traits::FiniteField;

/// Multiset Topology Aggregator.
/// Evaluates a polynomial where injected elements act as roots.
/// H = Product(X_g + phi(x_i))
pub struct MultisetAggregator;

impl MultisetAggregator {
    /// Affine Generator Constant X_g = x^255.
    /// This resides in an orthogonal dimension to all valid embedded data.
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
            buffer[31] &= 0x7F; // AFFINE SUBSPACE AXIOM
            let block = F::from_bytes_canonical(&buffer);
            result = result.shift_phase().add(&block);
        }
        result
    }

    /// H = S * (X_g + e)
    /// Safe from zero-divisors by algebraic design.
    #[inline(always)]
    fn aggregate(state: &F, new_element: &F, _index: usize) -> F {
        let root = new_element.add(&Self::generator_constant::<F>());
        state.mul(&root)
    }

    /// Extracs an element using Fermat's Little Theorem.
    fn remove(state: &F, element: &F) -> Option<F> {
        let root = element.add(&Self::generator_constant::<F>());
        root.inv().map(|inverse| state.mul(&inverse))
    }
}
