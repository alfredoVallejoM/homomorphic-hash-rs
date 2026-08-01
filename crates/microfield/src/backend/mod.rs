//! Execution-strategy adapters.

pub(crate) mod portable;
pub(crate) mod profile;

#[cfg(all(feature = "portable", target_arch = "x86_64"))]
#[allow(unsafe_code)]
pub(crate) mod x86_pclmul;

#[cfg(all(feature = "portable", target_arch = "aarch64"))]
#[allow(unsafe_code)]
pub(crate) mod aarch64_pmull;

#[cfg(all(feature = "portable", feature = "builtin-fields"))]
use crate::{Gf2_128V1, Gf2_256AltV1, Gf2_256HhV1, kernel::KernelCatalog};

#[cfg(all(feature = "portable", feature = "builtin-fields"))]
pub(crate) const fn gf2_128_v1_catalog(
    portable: &'static crate::kernel::KernelSet<Gf2_128V1>,
) -> KernelCatalog<Gf2_128V1> {
    let catalog = KernelCatalog::portable(portable);
    #[cfg(target_arch = "x86_64")]
    {
        catalog.with_x86_pclmul(&x86_pclmul::GF2_128_V1_KERNELS)
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
        catalog.with_x86_pclmul(&x86_pclmul::GF2_256_HH_V1_KERNELS)
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
        catalog.with_x86_pclmul(&x86_pclmul::GF2_256_ALT_V1_KERNELS)
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
