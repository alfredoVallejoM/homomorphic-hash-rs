use crate::algebra::traits::FiniteField;
use crate::topology::traits::HomomorphicAggregator;
use std::marker::PhantomData;

/// Legacy state-machine facade over [`HomomorphicAggregator`].
///
/// It remains monomorphized, but carries no field/encoder/signature identity.
/// New code should store one of the identified types in [`crate::structural`].
pub struct TopoHasher<F: FiniteField, A: HomomorphicAggregator<F>> {
    state: F,
    element_count: usize,
    _marker: PhantomData<A>,
}

impl<F: FiniteField, A: HomomorphicAggregator<F>> TopoHasher<F, A> {
    /// Initializes the hasher using the strict topological vacuum defined by Layer 2.
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            state: A::empty_state(),
            element_count: 0,
            _marker: PhantomData,
        }
    }

    /// Embeds and aggregates one byte string.
    #[inline(always)]
    pub fn update(&mut self, data: &[u8]) {
        let element = A::embed_to_field(data);
        self.state = A::aggregate(&self.state, &element, self.element_count);
        self.element_count += 1;
    }

    /// Returns the legacy field state without metadata.
    #[inline(always)]
    pub fn finalize(self) -> F {
        self.state
    }
}

impl<F: FiniteField, A: HomomorphicAggregator<F>> Default for TopoHasher<F, A> {
    fn default() -> Self {
        Self::new()
    }
}
