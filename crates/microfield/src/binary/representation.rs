//! Private limb and wide-product representations.

/// Canonical four-limb representation of a degree-256 binary field.
pub(crate) type Limbs256 = [u64; 4];

/// Unreduced eight-limb polynomial product.
pub(crate) type Wide512 = [u64; 8];
