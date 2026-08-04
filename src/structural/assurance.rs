//! Explicit interpretation limits for finite-field structural signatures.

/// What equality of one structural state is allowed to establish.
///
/// This classification concerns the represented field elements. An encoder
/// collision can still map different source byte strings to the same element.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum SignatureAssurance {
    /// Equality is only equality of a finite fingerprint.
    Fingerprint,
    /// Enough distinct evaluations determine equal encoded coefficients up to
    /// the stated common cardinality.
    BoundedExactOverEncodedElements {
        /// Largest equal-length sequence or multiset covered by the theorem.
        maximum_cardinality: usize,
    },
    /// The application retains and compares every original item exactly.
    ExactTracked,
}

impl SignatureAssurance {
    /// Reports whether this assurance proves equality over already encoded
    /// elements at `cardinality`.
    #[must_use]
    pub fn covers_encoded_cardinality(self, cardinality: u64) -> bool {
        match self {
            Self::Fingerprint => false,
            Self::BoundedExactOverEncodedElements {
                maximum_cardinality,
            } => usize::try_from(cardinality)
                .is_ok_and(|cardinality| cardinality <= maximum_cardinality),
            Self::ExactTracked => true,
        }
    }

    /// Reports whether original source values, rather than only their field
    /// encodings, are retained exactly.
    #[must_use]
    pub const fn tracks_source_values(self) -> bool {
        matches!(self, Self::ExactTracked)
    }
}
