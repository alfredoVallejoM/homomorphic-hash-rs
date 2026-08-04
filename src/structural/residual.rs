//! Algebraic remainders without membership or security claims.

use microfield::Field;

use super::{SignatureId, SignatureLaw};

/// A value whose recomposition can be checked against an original signature.
///
/// A valid residual proves only the displayed field equation. It never proves
/// that the candidate belonged to an untracked collection: division by every
/// non-zero field element is always possible.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AlgebraicResidual<F: Field> {
    pub(crate) signature_id: SignatureId,
    pub(crate) law: SignatureLaw,
    pub(crate) state: F,
    pub(crate) item_count: u64,
    pub(crate) zero_factor_count: u64,
}

impl<F: Field> AlgebraicResidual<F> {
    /// Identity of the exact equation being recomposed.
    #[must_use]
    pub const fn signature_id(&self) -> SignatureId {
        self.signature_id
    }

    /// Structural law of the residual.
    #[must_use]
    pub const fn law(&self) -> SignatureLaw {
        self.law
    }

    /// Remaining field state.
    #[must_use]
    pub const fn state(&self) -> F {
        self.state
    }

    /// Remaining logical item count.
    #[must_use]
    pub const fn item_count(&self) -> u64 {
        self.item_count
    }

    /// Remaining zero-factor count; meaningful only for multiset laws.
    #[must_use]
    pub const fn zero_factor_count(&self) -> u64 {
        self.zero_factor_count
    }
}
