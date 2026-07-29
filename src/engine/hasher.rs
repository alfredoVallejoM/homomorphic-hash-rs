use crate::algebra::traits::FiniteField;
use crate::topology::traits::HomomorphicAggregator;
use std::marker::PhantomData;

/// Universal Topological Hasher.
/// Zero-Cost Abstraction state machine. The compiler monomorphizes this struct
/// specifically for the chosen `FiniteField` and `HomomorphicAggregator`.
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

    /// Injects external entropy into the macroscopic topology.
    /// Branchless operation at the hasher level.
    #[inline(always)]
    pub fn update(&mut self, data: &[u8]) {
        let element = A::embed_to_field(data);
        self.state = A::aggregate(&self.state, &element, self.element_count);
        self.element_count += 1;
    }

    /// Finalizes the topology and crystalizes the Galois Signature.
    #[inline(always)]
    pub fn finalize(self) -> F {
        self.state
    }
}
