/// Top-down Topological Shield Contract.
pub trait TopologicalMask {
    /// Generates the zero-dimensional empty mask.
    fn empty() -> Self;

    /// Maps a canonical variable index to a specific bit in the hypercube.
    fn from_variable_index(index: usize) -> Self;

    /// Combines two topologies (Bitwise OR).
    fn union(&self, other: &Self) -> Self;

    /// Categorical Implication Lemma: checks if `self` is fully contained in `other`.
    /// Mathematically: (self AND other) == self
    fn is_subset_of(&self, other: &Self) -> bool;
}

/// Physical implementation optimized for AVX2 (256-bit).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(align(32))]
pub struct TopoBloomMask(pub [u64; 4]);

impl TopologicalMask for TopoBloomMask {
    #[inline(always)]
    fn empty() -> Self {
        TopoBloomMask([0, 0, 0, 0])
    }

    /// Pure spatial mapping. Zero entropy.
    /// Maps variable 'i' to bit 'i mod 256'.
    #[inline(always)]
    fn from_variable_index(index: usize) -> Self {
        let mut mask = [0u64; 4];
        let bit = index % 256;
        mask[bit / 64] |= 1 << (bit % 64);
        TopoBloomMask(mask)
    }

    #[inline(always)]
    fn union(&self, other: &Self) -> Self {
        TopoBloomMask([
            self.0[0] | other.0[0],
            self.0[1] | other.0[1],
            self.0[2] | other.0[2],
            self.0[3] | other.0[3],
        ])
    }

    #[inline(always)]
    fn is_subset_of(&self, other: &Self) -> bool {
        (self.0[0] & other.0[0]) == self.0[0]
            && (self.0[1] & other.0[1]) == self.0[1]
            && (self.0[2] & other.0[2]) == self.0[2]
            && (self.0[3] & other.0[3]) == self.0[3]
    }
}
