use super::traits::FiniteField;
use microfield::{BinaryPolynomialField, CanonicalEncoding, Field, Gf2_256HhV1, Invert};

/// Byte-compatible aligned wrapper for the original 256-bit field API.
///
/// Arithmetic delegates to [`Gf2_256HhV1`]. Alignment is retained solely for
/// ABI compatibility and is not itself a SIMD or latency guarantee.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(align(32))]
pub struct GaloisSignature256(pub [u64; 4]);

impl GaloisSignature256 {
    /// Checks if the signature represents the topological singularity (0).
    #[inline(always)]
    pub fn is_zero(&self) -> bool {
        // Bitwise OR allows the compiler to fuse registers efficiently
        (self.0[0] | self.0[1] | self.0[2] | self.0[3]) == 0
    }

    /// Converts the compatibility representation into the maintained
    /// Microfield presentation with identical canonical bytes.
    #[must_use]
    pub fn to_microfield(self) -> Gf2_256HhV1 {
        Gf2_256HhV1::from_canonical(&self.to_canonical_bytes())
            .expect("all 256-bit binary-field encodings are canonical")
    }

    /// Reconstructs the legacy aligned wrapper without exposing Microfield's
    /// private limbs.
    #[must_use]
    pub fn from_microfield(value: Gf2_256HhV1) -> Self {
        Self::from_canonical_bytes(value.to_canonical())
    }

    /// Returns the frozen little-endian legacy representation.
    #[must_use]
    pub fn to_canonical_bytes(self) -> [u8; 32] {
        let mut bytes = [0_u8; 32];
        for (limb, chunk) in self.0.into_iter().zip(bytes.chunks_exact_mut(8)) {
            chunk.copy_from_slice(&limb.to_le_bytes());
        }
        bytes
    }

    fn from_canonical_bytes(bytes: [u8; 32]) -> Self {
        let mut limbs = [0_u64; 4];
        for (limb, chunk) in limbs.iter_mut().zip(bytes.chunks_exact(8)) {
            *limb = u64::from_le_bytes(chunk.try_into().expect("chunk width is exact"));
        }
        Self(limbs)
    }
}

impl FiniteField for GaloisSignature256 {
    #[inline(always)]
    fn zero() -> Self {
        GaloisSignature256([0, 0, 0, 0])
    }

    #[inline(always)]
    fn one() -> Self {
        GaloisSignature256([1, 0, 0, 0])
    }

    #[inline(always)]
    fn add(&self, other: &Self) -> Self {
        Self::from_microfield(self.to_microfield().add(other.to_microfield()))
    }

    fn mul(&self, other: &Self) -> Self {
        Self::from_microfield(self.to_microfield().mul(other.to_microfield()))
    }

    /// Multiplicative inverse delegated to Microfield's generated schedule.
    fn inv(&self) -> Option<Self> {
        self.to_microfield().invert().map(Self::from_microfield)
    }

    /// Multiplies by the polynomial-basis element `x`.
    #[inline(always)]
    fn shift_phase(&self) -> Self {
        Self::from_microfield(self.to_microfield().mul_by_x())
    }

    #[inline(always)]
    fn from_bytes_canonical(data: &[u8; 32]) -> Self {
        Self::from_canonical_bytes(*data)
    }
}

impl From<Gf2_256HhV1> for GaloisSignature256 {
    fn from(value: Gf2_256HhV1) -> Self {
        Self::from_microfield(value)
    }
}

impl From<GaloisSignature256> for Gf2_256HhV1 {
    fn from(value: GaloisSignature256) -> Self {
        value.to_microfield()
    }
}
