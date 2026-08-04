//! Common identified metadata for signature and field profiles.

use microfield::{CanonicalEncoding, Field, FieldId, Invert, Pow, StaticField};

use super::{
    AdditiveSignature, BidirectionalSequenceSignature, MultiEvaluationMultisetSignature,
    MultiEvaluationSequenceSignature, MultisetSignature, SequenceSignature, SignatureAssurance,
    SignatureContext, SignatureError, StructuralEncoder,
};

/// Whether a signature field uses static dispatch or a runtime context.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum SignatureFieldBinding {
    /// Nominal Rust type with compile-time dispatch.
    Static,
    /// Validated runtime context with explicit identity checks.
    Runtime,
}

/// Field metadata relevant when selecting a signature profile.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SignatureFieldProfile {
    field_id: FieldId,
    characteristic_is_two: bool,
    extension_degree: u32,
    canonical_bytes: usize,
    binding: SignatureFieldBinding,
}

impl SignatureFieldProfile {
    /// Describes a certified static field without hardcoding its concrete type.
    #[must_use]
    pub fn for_static<F: StaticField>() -> Self {
        let spec = F::spec();
        Self {
            field_id: spec.field_id(),
            characteristic_is_two: spec.characteristic() == 2,
            extension_degree: spec.degree(),
            canonical_bytes: usize::from(spec.canonical_bytes()),
            binding: SignatureFieldBinding::Static,
        }
    }

    /// Describes a validated runtime field context.
    #[cfg(any(feature = "dynamic-signatures", feature = "dynamic-fields"))]
    #[must_use]
    pub fn for_dynamic(field: &microfield::DynField) -> Self {
        Self {
            field_id: field.field_id(),
            characteristic_is_two: field.characteristic_is_two(),
            extension_degree: field.extension_degree(),
            canonical_bytes: field.canonical_bytes(),
            binding: SignatureFieldBinding::Runtime,
        }
    }

    /// Mathematical field identity.
    #[must_use]
    pub const fn field_id(self) -> FieldId {
        self.field_id
    }

    /// Reports whether addition has characteristic-two parity semantics.
    #[must_use]
    pub const fn characteristic_is_two(self) -> bool {
        self.characteristic_is_two
    }

    /// Extension degree over the prime subfield.
    #[must_use]
    pub const fn extension_degree(self) -> u32 {
        self.extension_degree
    }

    /// Canonical field-element width.
    #[must_use]
    pub const fn canonical_bytes(self) -> usize {
        self.canonical_bytes
    }

    /// Static or runtime binding mode.
    #[must_use]
    pub const fn binding(self) -> SignatureFieldBinding {
        self.binding
    }
}

/// Uniform read-only description of one concrete compact signature state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SignatureProfile {
    context: SignatureContext,
    assurance: SignatureAssurance,
    item_count: u64,
    evaluation_count: usize,
}

/// Evaluation counts maintained as RC profiles.
///
/// Other const-generic counts remain expressible but are experimental until
/// separately measured and admitted to the supported-surface inventory.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum SignatureEvaluationProfile {
    /// One coordinate: minimum state and arithmetic cost.
    K1,
    /// Two coordinates: maintained balanced profile.
    K2,
    /// Four coordinates: maintained high-discrimination profile.
    K4,
}

impl SignatureEvaluationProfile {
    /// Number of independent coordinates.
    #[must_use]
    pub const fn evaluation_count(self) -> usize {
        match self {
            Self::K1 => 1,
            Self::K2 => 2,
            Self::K4 => 4,
        }
    }

    /// Maps a coordinate count to a maintained profile.
    #[must_use]
    pub const fn from_evaluation_count(count: usize) -> Option<Self> {
        match count {
            1 => Some(Self::K1),
            2 => Some(Self::K2),
            4 => Some(Self::K4),
            _ => None,
        }
    }
}

impl SignatureProfile {
    /// Creates metadata for an already validated signature state.
    #[must_use]
    pub(crate) const fn new(
        context: SignatureContext,
        assurance: SignatureAssurance,
        item_count: u64,
        evaluation_count: usize,
    ) -> Self {
        Self {
            context,
            assurance,
            item_count,
            evaluation_count,
        }
    }

    /// Complete compatibility identity.
    #[must_use]
    pub const fn context(self) -> SignatureContext {
        self.context
    }

    /// Meaning allowed for equality of the represented state.
    #[must_use]
    pub const fn assurance(self) -> SignatureAssurance {
        self.assurance
    }

    /// Exact number of logical input items.
    #[must_use]
    pub const fn item_count(self) -> u64 {
        self.item_count
    }

    /// Number of independent evaluation coordinates.
    #[must_use]
    pub const fn evaluation_count(self) -> usize {
        self.evaluation_count
    }

    /// Returns the maintained K profile, or `None` for an experimental count.
    #[must_use]
    pub const fn maintained_evaluation_profile(self) -> Option<SignatureEvaluationProfile> {
        SignatureEvaluationProfile::from_evaluation_count(self.evaluation_count)
    }
}

mod sealed {
    pub trait Sealed {}
}

/// Common static-dispatch contract for compact homomorphic signature states.
///
/// The trait is sealed and requires `Clone`, so it cannot be used as a trait
/// object. It adds uniform metadata and persistence outside the algebraic hot
/// path without erasing the concrete law or field type.
pub trait CompactSignature: Clone + sealed::Sealed {
    /// Returns the identified law, assurance, item count and lane count.
    #[must_use]
    fn signature_profile(&self) -> SignatureProfile;

    /// Serializes the compact `MFSG` state.
    ///
    /// This snapshot never contains source values retained by a `Tracked*`
    /// adapter.
    ///
    /// # Errors
    ///
    /// Runtime fields may reject an internally inconsistent element.
    fn to_compact_snapshot(&self) -> Result<Vec<u8>, SignatureError>;
}

macro_rules! impl_static_compact {
    ($signature:ty, $count:expr, $evaluations:expr) => {
        impl<F, E> sealed::Sealed for $signature
        where
            F: Field + CanonicalEncoding + StaticField,
            E: StructuralEncoder<F>,
        {
        }

        impl<F, E> CompactSignature for $signature
        where
            F: Field + CanonicalEncoding + StaticField,
            E: StructuralEncoder<F>,
        {
            fn signature_profile(&self) -> SignatureProfile {
                SignatureProfile::new(self.context(), self.assurance(), $count(self), $evaluations)
            }

            fn to_compact_snapshot(&self) -> Result<Vec<u8>, SignatureError> {
                Ok(self.to_canonical_bytes())
            }
        }
    };
}

impl_static_compact!(
    AdditiveSignature<F, E>,
    |value: &AdditiveSignature<F, E>| value.term_count(),
    1
);

impl<F, E> sealed::Sealed for SequenceSignature<F, E>
where
    F: Field + CanonicalEncoding + StaticField + Pow + Invert,
    E: StructuralEncoder<F>,
{
}

impl<F, E> CompactSignature for SequenceSignature<F, E>
where
    F: Field + CanonicalEncoding + StaticField + Pow + Invert,
    E: StructuralEncoder<F>,
{
    fn signature_profile(&self) -> SignatureProfile {
        SignatureProfile::new(self.context(), self.assurance(), self.len(), 1)
    }

    fn to_compact_snapshot(&self) -> Result<Vec<u8>, SignatureError> {
        Ok(self.to_canonical_bytes())
    }
}

impl<F, E> sealed::Sealed for BidirectionalSequenceSignature<F, E>
where
    F: Field + CanonicalEncoding + StaticField + Pow + Invert,
    E: StructuralEncoder<F>,
{
}

impl<F, E> CompactSignature for BidirectionalSequenceSignature<F, E>
where
    F: Field + CanonicalEncoding + StaticField + Pow + Invert,
    E: StructuralEncoder<F>,
{
    fn signature_profile(&self) -> SignatureProfile {
        SignatureProfile::new(self.context(), self.assurance(), self.len(), 2)
    }

    fn to_compact_snapshot(&self) -> Result<Vec<u8>, SignatureError> {
        Ok(self.to_canonical_bytes())
    }
}

impl<F, E> sealed::Sealed for MultisetSignature<F, E>
where
    F: Field + CanonicalEncoding + StaticField + Invert,
    E: StructuralEncoder<F>,
{
}

impl<F, E> CompactSignature for MultisetSignature<F, E>
where
    F: Field + CanonicalEncoding + StaticField + Invert,
    E: StructuralEncoder<F>,
{
    fn signature_profile(&self) -> SignatureProfile {
        SignatureProfile::new(self.context(), self.assurance(), self.cardinality(), 1)
    }

    fn to_compact_snapshot(&self) -> Result<Vec<u8>, SignatureError> {
        Ok(self.to_canonical_bytes())
    }
}

impl<F, E, const K: usize> sealed::Sealed for MultiEvaluationMultisetSignature<F, E, K>
where
    F: Field + CanonicalEncoding + StaticField,
    E: StructuralEncoder<F>,
{
}

impl<F, E, const K: usize> CompactSignature for MultiEvaluationMultisetSignature<F, E, K>
where
    F: Field + CanonicalEncoding + StaticField,
    E: StructuralEncoder<F>,
{
    fn signature_profile(&self) -> SignatureProfile {
        SignatureProfile::new(self.context(), self.assurance(), self.cardinality(), K)
    }

    fn to_compact_snapshot(&self) -> Result<Vec<u8>, SignatureError> {
        Ok(self.to_canonical_bytes())
    }
}

impl<F, E, const K: usize> sealed::Sealed for MultiEvaluationSequenceSignature<F, E, K>
where
    F: Field + CanonicalEncoding + StaticField + Pow,
    E: StructuralEncoder<F>,
{
}

impl<F, E, const K: usize> CompactSignature for MultiEvaluationSequenceSignature<F, E, K>
where
    F: Field + CanonicalEncoding + StaticField + Pow,
    E: StructuralEncoder<F>,
{
    fn signature_profile(&self) -> SignatureProfile {
        SignatureProfile::new(self.context(), self.assurance(), self.len(), K)
    }

    fn to_compact_snapshot(&self) -> Result<Vec<u8>, SignatureError> {
        Ok(self.to_canonical_bytes())
    }
}

#[cfg(any(feature = "dynamic-signatures", feature = "dynamic-fields"))]
mod dynamic_profiles {
    use super::{sealed, CompactSignature, SignatureAssurance, SignatureProfile};
    use crate::structural::{
        DynamicAdditiveSignature, DynamicBidirectionalSequenceSignature,
        DynamicMultiEvaluationMultisetSignature, DynamicMultiEvaluationSequenceSignature,
        DynamicMultisetSignature, DynamicSequenceSignature, DynamicStructuralEncoder,
        SignatureError,
    };

    macro_rules! impl_dynamic_compact {
        ($signature:ident, $count:ident, $evaluations:expr, $assurance:expr) => {
            impl<E> sealed::Sealed for $signature<E> where E: DynamicStructuralEncoder {}

            impl<E> CompactSignature for $signature<E>
            where
                E: DynamicStructuralEncoder,
            {
                fn signature_profile(&self) -> SignatureProfile {
                    SignatureProfile::new(
                        self.context(),
                        $assurance(self),
                        self.$count(),
                        $evaluations(self),
                    )
                }

                fn to_compact_snapshot(&self) -> Result<Vec<u8>, SignatureError> {
                    self.to_canonical_bytes()
                }
            }
        };
    }

    impl_dynamic_compact!(DynamicAdditiveSignature, term_count, |_| 1, |_| {
        SignatureAssurance::Fingerprint
    });
    impl_dynamic_compact!(DynamicSequenceSignature, len, |_| 1, |_| {
        SignatureAssurance::Fingerprint
    });
    impl_dynamic_compact!(DynamicBidirectionalSequenceSignature, len, |_| 2, |_| {
        SignatureAssurance::Fingerprint
    });
    impl_dynamic_compact!(DynamicMultisetSignature, cardinality, |_| 1, |_| {
        SignatureAssurance::Fingerprint
    });
    impl_dynamic_compact!(
        DynamicMultiEvaluationMultisetSignature,
        cardinality,
        |value: &DynamicMultiEvaluationMultisetSignature<E>| value.offsets().len(),
        |value: &DynamicMultiEvaluationMultisetSignature<E>| value.assurance()
    );
    impl_dynamic_compact!(
        DynamicMultiEvaluationSequenceSignature,
        len,
        |value: &DynamicMultiEvaluationSequenceSignature<E>| value.bases().len(),
        |value: &DynamicMultiEvaluationSequenceSignature<E>| value.assurance()
    );
}
