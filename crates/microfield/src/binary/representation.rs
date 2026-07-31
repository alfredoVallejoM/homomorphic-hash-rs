//! Private limb and wide-product representations.

/// Canonical two-limb representation of a degree-128 binary field.
pub(crate) type Limbs128 = [u64; 2];

/// Canonical four-limb representation of a degree-256 binary field.
pub(crate) type Limbs256 = [u64; 4];

/// Unreduced four-limb polynomial product.
pub(crate) type Wide256 = [u64; 4];

/// Unreduced eight-limb polynomial product.
pub(crate) type Wide512 = [u64; 8];
