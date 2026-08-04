//! Execution-strategy adapters.

pub(crate) mod portable;
#[cfg(feature = "prime-fields")]
pub(crate) mod prime_profile;
pub(crate) mod profile;

#[cfg(all(feature = "portable", target_arch = "x86_64"))]
#[allow(unsafe_code)]
pub(crate) mod x86_pclmul;

#[cfg(all(feature = "portable", target_arch = "x86_64"))]
#[allow(unsafe_code)]
pub(crate) mod x86_vpclmul;

#[cfg(all(feature = "portable", feature = "prime-fields", target_arch = "x86_64"))]
#[allow(unsafe_code)]
pub(crate) mod x86_prime;

#[cfg(all(feature = "portable", target_arch = "aarch64"))]
#[allow(unsafe_code)]
pub(crate) mod aarch64_pmull;

#[cfg(all(feature = "portable", feature = "builtin-fields"))]
use crate::{Gf2_128V1, Gf2_256AltV1, Gf2_256HhV1, kernel::KernelCatalog};

#[cfg(all(feature = "portable", feature = "prime-fields"))]
use crate::{Fp251V1, Fp256GenericV1, FpGoldilocks64V1};

#[cfg(all(
    feature = "portable",
    feature = "prime-fields",
    not(feature = "builtin-fields")
))]
use crate::kernel::KernelCatalog;

#[cfg(all(feature = "portable", feature = "builtin-fields"))]
pub(crate) const fn gf2_128_v1_catalog(
    portable: &'static crate::kernel::KernelSet<Gf2_128V1>,
) -> KernelCatalog<Gf2_128V1> {
    let catalog = KernelCatalog::portable(portable);
    #[cfg(target_arch = "x86_64")]
    {
        catalog
            .with_x86_pclmul(&x86_pclmul::GF2_128_V1_KERNELS)
            .with_x86_vpclmul(&x86_vpclmul::GF2_128_V1_KERNELS)
    }
    #[cfg(target_arch = "aarch64")]
    {
        catalog.with_aarch64_pmull(&aarch64_pmull::GF2_128_V1_KERNELS)
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        catalog
    }
}

#[cfg(all(feature = "portable", feature = "builtin-fields"))]
pub(crate) const fn gf2_256_hh_v1_catalog(
    portable: &'static crate::kernel::KernelSet<Gf2_256HhV1>,
) -> KernelCatalog<Gf2_256HhV1> {
    let catalog = KernelCatalog::portable(portable);
    #[cfg(target_arch = "x86_64")]
    {
        catalog
            .with_x86_pclmul(&x86_pclmul::GF2_256_HH_V1_KERNELS)
            .with_x86_vpclmul(&x86_vpclmul::GF2_256_HH_V1_KERNELS)
    }
    #[cfg(target_arch = "aarch64")]
    {
        catalog.with_aarch64_pmull(&aarch64_pmull::GF2_256_HH_V1_KERNELS)
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        catalog
    }
}

#[cfg(all(feature = "portable", feature = "builtin-fields"))]
pub(crate) const fn gf2_256_alt_v1_catalog(
    portable: &'static crate::kernel::KernelSet<Gf2_256AltV1>,
) -> KernelCatalog<Gf2_256AltV1> {
    let catalog = KernelCatalog::portable(portable);
    #[cfg(target_arch = "x86_64")]
    {
        catalog
            .with_x86_pclmul(&x86_pclmul::GF2_256_ALT_V1_KERNELS)
            .with_x86_vpclmul(&x86_vpclmul::GF2_256_ALT_V1_KERNELS)
    }
    #[cfg(target_arch = "aarch64")]
    {
        catalog.with_aarch64_pmull(&aarch64_pmull::GF2_256_ALT_V1_KERNELS)
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        catalog
    }
}

#[cfg(all(feature = "portable", feature = "prime-fields"))]
pub(crate) const fn fp251_v1_catalog(
    portable: &'static crate::kernel::KernelSet<Fp251V1>,
) -> KernelCatalog<Fp251V1> {
    let catalog = KernelCatalog::portable(portable);
    #[cfg(target_arch = "x86_64")]
    {
        catalog.with_x86_prime_avx2(&x86_prime::FP251_AVX2_KERNELS)
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        catalog
    }
}

#[cfg(all(feature = "portable", feature = "prime-fields"))]
pub(crate) const fn fp_goldilocks64_v1_catalog(
    portable: &'static crate::kernel::KernelSet<FpGoldilocks64V1>,
) -> KernelCatalog<FpGoldilocks64V1> {
    let catalog = KernelCatalog::portable(portable);
    #[cfg(target_arch = "x86_64")]
    {
        catalog.with_x86_prime_avx2(&x86_prime::FP_GOLDILOCKS_AVX2_KERNELS)
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        catalog
    }
}

#[cfg(all(feature = "portable", feature = "prime-fields"))]
pub(crate) const fn fp256_generic_v1_catalog(
    portable: &'static crate::kernel::KernelSet<Fp256GenericV1>,
) -> KernelCatalog<Fp256GenericV1> {
    let catalog = KernelCatalog::portable(portable);
    #[cfg(target_arch = "x86_64")]
    {
        catalog.with_x86_prime_bmi2(&x86_prime::FP256_BMI2_KERNELS)
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        catalog
    }
}
