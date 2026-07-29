use super::traits::FiniteField;

/// 256-bit Topological Signature optimized for SIMD AVX2.
/// Strict 32-byte memory alignment forces LLVM to emit single-cycle
/// `vmovdqa` vector loads.
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
        // Characteristic 2 addition is a carry-less XOR.
        // LLVM auto-vectorizes this into a single `vpxor` YMM instruction.
        GaloisSignature256([
            self.0[0] ^ other.0[0],
            self.0[1] ^ other.0[1],
            self.0[2] ^ other.0[2],
            self.0[3] ^ other.0[3],
        ])
    }

    // =========================================================================
    // MULTIPLICATION: STRUCTURAL MODE (High Throughput / Default)
    // =========================================================================
    #[cfg(not(feature = "crypto_mode"))]
    fn mul(&self, other: &Self) -> Self {
        // Early exit: Singularity annihilation
        if self.is_zero() || other.is_zero() {
            return Self::zero();
        }

        let mut result = Self::zero();
        let mut base = *self;

        for i in 0..4 {
            let mut word = other.0[i];

            // Structural Optimization: Skip empty 64-bit blocks entirely.
            if word == 0 {
                for _ in 0..64 {
                    base = base.shift_phase();
                }
                continue;
            }

            for j in 0..64 {
                if (word & 1) == 1 {
                    result = result.add(&base);
                }

                base = base.shift_phase();
                word >>= 1;

                // Resolved Micro-optimization: Phase Compensation
                // If no bits remain in this word, we can exit the inner loop early
                // to save CPU cycles, BUT we must mathematically fast-forward the base
                // polynomial by the remaining steps to maintain strict positional
                // alignment for the next 64-bit block in the outer loop.
                if word == 0 {
                    let remaining_shifts = 63 - j;
                    for _ in 0..remaining_shifts {
                        base = base.shift_phase();
                    }
                    break;
                }
            }
        }
        result
    }

    // =========================================================================
    // MULTIPLICATION: CRYPTOGRAPHIC MODE (Constant-Time)
    // =========================================================================
    #[cfg(feature = "crypto_mode")]
    fn mul(&self, other: &Self) -> Self {
        let mut result = Self::zero();
        let mut base = *self;

        // Strict iteration. No early exits. No conditionals.
        for i in 0..4 {
            let mut word = other.0[i];
            for _ in 0..64 {
                let bit = word & 1;
                // Branchless mask: if bit=1 -> 0xFF..FF, if bit=0 -> 0x00..00
                let mask = 0u64.wrapping_sub(bit);

                result.0[0] ^= base.0[0] & mask;
                result.0[1] ^= base.0[1] & mask;
                result.0[2] ^= base.0[2] & mask;
                result.0[3] ^= base.0[3] & mask;

                base = base.shift_phase();
                word >>= 1;
            }
        }
        result
    }

    /// Multiplicative inverse using Fermat's Little Theorem.
    /// Exponent: 2^256 - 2.
    fn inv(&self) -> Option<Self> {
        if self.is_zero() {
            return None; // Non-invertible singularity
        }

        let mut base = *self;

        // 2^256 - 2 binary representation: 255 'ones' followed by 1 'zero'.
        // We skip the highest bit (already in `base`) and process 254 bits.
        for _ in 1..255 {
            base = base.mul(&base); // Square (Frobenius endomorphism)
            base = base.mul(self); // Multiply
        }

        // Final bit is '0', so we only apply the square step.
        base = base.mul(&base);

        Some(base)
    }

    // =========================================================================
    // PHASE SHIFT (Universal Branchless)
    // =========================================================================
    /// Shifts the polynomial left by 1 bit (Multiplication by 'x').
    /// Why branchless for both modes?
    /// Because in a dense hash, the overflow bit is '1' exactly 50% of the time.
    /// A conditional `if` would cause a CPU Branch Misprediction 50% of the time,
    /// causing a 15-cycle pipeline flush. The bitwise mask costs only 1 cycle.
    #[inline(always)]
    fn shift_phase(&self) -> Self {
        let carry0 = self.0[0] >> 63;
        let carry1 = self.0[1] >> 63;
        let carry2 = self.0[2] >> 63;
        let carry3 = self.0[3] >> 63; // Bit 255

        let mut res = GaloisSignature256([
            self.0[0] << 1,
            (self.0[1] << 1) | carry0,
            (self.0[2] << 1) | carry1,
            (self.0[3] << 1) | carry2,
        ]);

        // Branchless modular reduction: P(x) = x^256 + x^10 + x^5 + x^2 + 1 (0x425)
        let mask = 0u64.wrapping_sub(carry3);
        res.0[0] ^= 0x425 & mask;

        res
    }

    #[inline(always)]
    fn from_bytes_canonical(data: &[u8; 32]) -> Self {
        // Strict Little-Endian memory mapping to preserve structural determinism
        // across different CPU architectures (x86_64 vs ARM).
        GaloisSignature256([
            u64::from_le_bytes(data[0..8].try_into().unwrap()),
            u64::from_le_bytes(data[8..16].try_into().unwrap()),
            u64::from_le_bytes(data[16..24].try_into().unwrap()),
            u64::from_le_bytes(data[24..32].try_into().unwrap()),
        ])
    }
}
