//! Prime-field representations, reductions and generated plan contracts.

mod certificate;
mod exponentiation;
mod goldilocks;
mod implementation;
mod montgomery;
mod plan;
mod range;
mod small;

pub use certificate::{PrimeCertificateError, verify_builtin_prime_certificates};
pub use exponentiation::{PrimeExponentiationCost, PrimeExponentiationPlan};
pub use plan::{
    BarrettPlan, MontgomeryAlgorithm, MontgomeryPlan, PrimeReductionKind, PrimeReductionPlan,
    PrimeRepresentationKind, RangeContract, RangeProofError, SignedPowerOfTwo, SolinasPlan,
};

pub(crate) use goldilocks::{barrett_reduce_goldilocks, reduce_goldilocks};
pub(crate) use implementation::{PrimeFieldSpec, PrimeWideProduct};
#[cfg(all(feature = "portable", target_arch = "x86_64"))]
pub(crate) use montgomery::{add_mod, montgomery_reduce_wide};
pub(crate) use montgomery::{
    add_mod_256, cmp_limbs, from_montgomery_256, montgomery_reduce_wide_256, neg_mod_256,
    sub_mod_256, to_montgomery_256, wide_product,
};
pub(crate) use small::reduce_bytes_mod_u16;
