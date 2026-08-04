//! Immutable CPU capabilities used during engine construction.

use crate::BackendId;

const X86_PCLMULQDQ: u16 = 1 << 0;
const X86_AVX2: u16 = 1 << 1;
const X86_VPCLMULQDQ: u16 = 1 << 2;
const AARCH64_NEON: u16 = 1 << 3;
const AARCH64_PMULL: u16 = 1 << 4;
const X86_BMI2: u16 = 1 << 5;
const X86_ADX: u16 = 1 << 6;
const X86_AVX512F: u16 = 1 << 7;
const X86_AVX512IFMA: u16 = 1 << 8;

/// Architecture family relevant to Microfield's compiled batch backends.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum Architecture {
    /// A 64-bit x86 target.
    X86_64,
    /// A 64-bit Arm target.
    Aarch64,
    /// A target without a Microfield ISA backend.
    Other,
}

impl Architecture {
    /// Returns the architecture selected by the Rust compilation target.
    #[must_use]
    pub const fn current() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            Self::X86_64
        }
        #[cfg(target_arch = "aarch64")]
        {
            Self::Aarch64
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            Self::Other
        }
    }
}

/// Trusted, immutable snapshot of the CPU features relevant to Microfield.
///
/// Values can be obtained through [`Self::detect`] when `std` is enabled or
/// through [`Self::portable_only`] on every target. Individual feature bits
/// cannot be forged through the public API.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CpuCapabilities {
    architecture: Architecture,
    features: u16,
}

impl CpuCapabilities {
    /// Returns a snapshot that disables every optional ISA backend.
    ///
    /// The compilation-target architecture remains available for diagnostics.
    #[must_use]
    pub const fn portable_only() -> Self {
        Self {
            architecture: Architecture::current(),
            features: 0,
        }
    }

    /// Detects the relevant features of the current CPU exactly once.
    ///
    /// This operation is available only with `std`; `no_std` consumers must
    /// inject [`Self::portable_only`]. Detection never runs inside a batch
    /// operation.
    #[cfg(feature = "std")]
    #[must_use]
    pub fn detect() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            let mut features = 0;
            if std::arch::is_x86_feature_detected!("pclmulqdq") {
                features |= X86_PCLMULQDQ;
            }
            if std::arch::is_x86_feature_detected!("avx2") {
                features |= X86_AVX2;
            }
            if std::arch::is_x86_feature_detected!("vpclmulqdq") {
                features |= X86_VPCLMULQDQ;
            }
            if std::arch::is_x86_feature_detected!("bmi2") {
                features |= X86_BMI2;
            }
            if std::arch::is_x86_feature_detected!("adx") {
                features |= X86_ADX;
            }
            if std::arch::is_x86_feature_detected!("avx512f") {
                features |= X86_AVX512F;
            }
            if std::arch::is_x86_feature_detected!("avx512ifma") {
                features |= X86_AVX512IFMA;
            }
            Self {
                architecture: Architecture::X86_64,
                features,
            }
        }

        #[cfg(target_arch = "aarch64")]
        {
            let mut features = 0;
            if std::arch::is_aarch64_feature_detected!("neon") {
                features |= AARCH64_NEON;
            }
            if std::arch::is_aarch64_feature_detected!("pmull") {
                features |= AARCH64_PMULL;
            }
            Self {
                architecture: Architecture::Aarch64,
                features,
            }
        }

        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            Self::portable_only()
        }
    }

    /// Returns the architecture recorded by this snapshot.
    #[must_use]
    pub const fn architecture(self) -> Architecture {
        self.architecture
    }

    /// Reports scalar x86 carry-less multiplication support.
    #[must_use]
    pub const fn has_x86_pclmulqdq(self) -> bool {
        self.features & X86_PCLMULQDQ != 0
    }

    /// Reports x86 AVX2 support.
    #[must_use]
    pub const fn has_x86_avx2(self) -> bool {
        self.features & X86_AVX2 != 0
    }

    /// Reports x86 vector carry-less multiplication support.
    #[must_use]
    pub const fn has_x86_vpclmulqdq(self) -> bool {
        self.features & X86_VPCLMULQDQ != 0
    }

    /// Reports scalar x86 BMI2 multiplication support.
    #[must_use]
    pub const fn has_x86_bmi2(self) -> bool {
        self.features & X86_BMI2 != 0
    }

    /// Reports x86 dual carry-chain support.
    #[must_use]
    pub const fn has_x86_adx(self) -> bool {
        self.features & X86_ADX != 0
    }

    /// Reports x86 AVX-512 foundation support.
    #[must_use]
    pub const fn has_x86_avx512f(self) -> bool {
        self.features & X86_AVX512F != 0
    }

    /// Reports x86 AVX-512 integer fused multiply-add support.
    #[must_use]
    pub const fn has_x86_avx512ifma(self) -> bool {
        self.features & X86_AVX512IFMA != 0
    }

    /// Reports `AArch64` NEON support.
    #[must_use]
    pub const fn has_aarch64_neon(self) -> bool {
        self.features & AARCH64_NEON != 0
    }

    /// Reports `AArch64` polynomial multiplication support.
    #[must_use]
    pub const fn has_aarch64_pmull(self) -> bool {
        self.features & AARCH64_PMULL != 0
    }

    /// Reports availability of the scalar high-half multiply required by
    /// future `AArch64` prime-field adapters.
    ///
    /// `UMULH` is part of the maintained 64-bit Arm architecture baseline, so
    /// this is an architecture property rather than a runtime feature bit.
    #[must_use]
    pub const fn has_aarch64_mul_high(self) -> bool {
        matches!(self.architecture, Architecture::Aarch64)
    }

    pub(crate) const fn supports(self, backend: BackendId) -> bool {
        match backend {
            BackendId::Portable => true,
            BackendId::X86Pclmul => {
                matches!(self.architecture, Architecture::X86_64) && self.has_x86_pclmulqdq()
            }
            BackendId::X86Vpclmul => {
                matches!(self.architecture, Architecture::X86_64)
                    && self.has_x86_pclmulqdq()
                    && self.has_x86_avx2()
                    && self.has_x86_vpclmulqdq()
            }
            BackendId::Aarch64Pmull => {
                matches!(self.architecture, Architecture::Aarch64)
                    && self.has_aarch64_neon()
                    && self.has_aarch64_pmull()
            }
            BackendId::X86PrimeAvx2 => {
                matches!(self.architecture, Architecture::X86_64) && self.has_x86_avx2()
            }
            BackendId::X86PrimeBmi2 => {
                matches!(self.architecture, Architecture::X86_64) && self.has_x86_bmi2()
            }
        }
    }

    #[cfg(test)]
    pub(crate) const fn from_test_parts(architecture: Architecture, features: u16) -> Self {
        Self {
            architecture,
            features: features & 0x01ff,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_snapshot_disables_every_isa_backend() {
        let capabilities = CpuCapabilities::portable_only();
        assert!(capabilities.supports(BackendId::Portable));
        assert!(!capabilities.supports(BackendId::X86Pclmul));
        assert!(!capabilities.supports(BackendId::X86Vpclmul));
        assert!(!capabilities.supports(BackendId::Aarch64Pmull));
        assert!(!capabilities.supports(BackendId::X86PrimeAvx2));
        assert!(!capabilities.supports(BackendId::X86PrimeBmi2));
        assert_eq!(
            capabilities.has_aarch64_mul_high(),
            matches!(Architecture::current(), Architecture::Aarch64)
        );
    }

    #[test]
    fn backend_requirements_include_architecture_and_prerequisites() {
        let all_x86 = CpuCapabilities::from_test_parts(
            Architecture::X86_64,
            X86_PCLMULQDQ | X86_AVX2 | X86_VPCLMULQDQ | X86_BMI2 | X86_ADX,
        );
        assert!(all_x86.supports(BackendId::X86Pclmul));
        assert!(all_x86.supports(BackendId::X86Vpclmul));
        assert!(all_x86.supports(BackendId::X86PrimeAvx2));
        assert!(all_x86.supports(BackendId::X86PrimeBmi2));
        assert!(!all_x86.supports(BackendId::Aarch64Pmull));

        for missing in 0..3 {
            let capabilities = CpuCapabilities::from_test_parts(
                Architecture::X86_64,
                (X86_PCLMULQDQ | X86_AVX2 | X86_VPCLMULQDQ) & !(1 << missing),
            );
            assert!(!capabilities.supports(BackendId::X86Vpclmul));
        }

        let all_arm =
            CpuCapabilities::from_test_parts(Architecture::Aarch64, AARCH64_NEON | AARCH64_PMULL);
        assert!(all_arm.supports(BackendId::Aarch64Pmull));
        assert!(all_arm.has_aarch64_mul_high());
        assert!(!all_arm.supports(BackendId::X86Pclmul));

        for missing in 0..2 {
            let capabilities = CpuCapabilities::from_test_parts(
                Architecture::Aarch64,
                (AARCH64_NEON | AARCH64_PMULL) & !(1 << (missing + 3)),
            );
            assert!(!capabilities.supports(BackendId::Aarch64Pmull));
        }
    }
}
