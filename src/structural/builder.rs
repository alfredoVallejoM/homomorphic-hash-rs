//! Typed construction facade for maintained signature families.

use core::marker::PhantomData;

use microfield::{CanonicalEncoding, Field, Invert, Pow, StaticField};

use super::{
    AdditiveSignature, BidirectionalSequenceSignature, MultiEvaluationMultisetSignature,
    MultiEvaluationSequenceSignature, MultisetSignature, SequenceSignature, SignatureError,
    SignatureFieldProfile, StructuralEncoder, TrackedMultiset, TrackedSequence,
};

/// Reusable, statically dispatched factory for signatures over one field and encoder.
///
/// Each method returns its concrete signature type. The builder never stores a
/// backend, uses virtual dispatch or erases the structural law.
#[derive(Clone, Debug)]
pub struct SignatureBuilder<F, E> {
    encoder: E,
    marker: PhantomData<fn() -> F>,
}

impl<F, E> SignatureBuilder<F, E>
where
    F: Field + CanonicalEncoding + StaticField,
    E: StructuralEncoder<F>,
{
    /// Binds subsequent signatures to one statically identified field and encoder.
    #[must_use]
    pub const fn new(encoder: E) -> Self {
        Self {
            encoder,
            marker: PhantomData,
        }
    }

    /// Describes the finite-field presentation selected by this builder.
    #[must_use]
    pub fn field_profile(&self) -> SignatureFieldProfile {
        SignatureFieldProfile::for_static::<F>()
    }

    /// Borrows the encoder shared by all products of this builder.
    #[must_use]
    pub const fn encoder(&self) -> &E {
        &self.encoder
    }

    /// Builds the additive identity.
    #[must_use]
    pub fn additive(&self) -> AdditiveSignature<F, E> {
        AdditiveSignature::new(self.encoder.clone())
    }

    /// Builds a sequence with an explicit positional base.
    ///
    /// # Errors
    ///
    /// Rejects zero, one or a non-invertible base.
    pub fn sequence(&self, base: F) -> Result<SequenceSignature<F, E>, SignatureError>
    where
        F: Pow + Invert,
    {
        SequenceSignature::new(self.encoder.clone(), base)
    }

    /// Builds a sequence that evaluates both orientations.
    ///
    /// # Errors
    ///
    /// Rejects a degenerate base.
    pub fn bidirectional_sequence(
        &self,
        base: F,
    ) -> Result<BidirectionalSequenceSignature<F, E>, SignatureError>
    where
        F: Pow + Invert,
    {
        BidirectionalSequenceSignature::new(self.encoder.clone(), base)
    }

    /// Builds a commutative product signature at one affine offset.
    #[must_use]
    pub fn multiset(&self, offset: F) -> MultisetSignature<F, E>
    where
        F: Invert,
    {
        MultisetSignature::new(self.encoder.clone(), offset)
    }

    /// Builds a multiset evaluated at `K` distinct offsets.
    ///
    /// # Errors
    ///
    /// Rejects an empty or repeated set of offsets.
    pub fn multi_evaluation_multiset<const K: usize>(
        &self,
        offsets: [F; K],
    ) -> Result<MultiEvaluationMultisetSignature<F, E, K>, SignatureError> {
        MultiEvaluationMultisetSignature::new(self.encoder.clone(), offsets)
    }

    /// Builds a sequence evaluated at `K` distinct positional bases.
    ///
    /// # Errors
    ///
    /// Rejects empty, repeated, zero or one bases.
    pub fn multi_evaluation_sequence<const K: usize>(
        &self,
        bases: [F; K],
    ) -> Result<MultiEvaluationSequenceSignature<F, E, K>, SignatureError>
    where
        F: Pow,
    {
        MultiEvaluationSequenceSignature::new(self.encoder.clone(), bases)
    }

    /// Builds an exact source-retaining sequence.
    ///
    /// # Errors
    ///
    /// Rejects a degenerate base.
    pub fn tracked_sequence(&self, base: F) -> Result<TrackedSequence<F, E>, SignatureError>
    where
        F: Pow + Invert,
    {
        TrackedSequence::new(self.encoder.clone(), base)
    }

    /// Builds an exact source-retaining multiset.
    #[must_use]
    pub fn tracked_multiset(&self, offset: F) -> TrackedMultiset<F, E>
    where
        F: Invert,
    {
        TrackedMultiset::new(self.encoder.clone(), offset)
    }
}

#[cfg(any(feature = "dynamic-signatures", feature = "dynamic-fields"))]
mod dynamic {
    use microfield::{DynElement, DynField};

    use super::super::{
        DynamicAdditiveSignature, DynamicBidirectionalSequenceSignature,
        DynamicMultiEvaluationMultisetSignature, DynamicMultiEvaluationSequenceSignature,
        DynamicMultisetSignature, DynamicSequenceSignature, DynamicStructuralEncoder,
        SignatureError, SignatureFieldProfile,
    };

    /// Reusable factory for signatures over one validated runtime field context.
    #[derive(Clone, Debug)]
    pub struct DynamicSignatureBuilder<E>
    where
        E: DynamicStructuralEncoder,
    {
        field: DynField,
        encoder: E,
    }

    impl<E> DynamicSignatureBuilder<E>
    where
        E: DynamicStructuralEncoder,
    {
        /// Binds subsequent signatures to one validated runtime field and encoder.
        #[must_use]
        pub const fn new(field: DynField, encoder: E) -> Self {
            Self { field, encoder }
        }

        /// Borrows the immutable runtime field context.
        #[must_use]
        pub const fn field(&self) -> &DynField {
            &self.field
        }

        /// Describes the runtime field selected by this builder.
        #[must_use]
        pub fn field_profile(&self) -> SignatureFieldProfile {
            SignatureFieldProfile::for_dynamic(&self.field)
        }

        /// Builds the additive identity.
        #[must_use]
        pub fn additive(&self) -> DynamicAdditiveSignature<E> {
            DynamicAdditiveSignature::new(self.field.clone(), self.encoder.clone())
        }

        /// Builds an ordered runtime sequence.
        ///
        /// # Errors
        ///
        /// Rejects a foreign or degenerate base.
        pub fn sequence(
            &self,
            base: DynElement,
        ) -> Result<DynamicSequenceSignature<E>, SignatureError> {
            DynamicSequenceSignature::new(self.field.clone(), self.encoder.clone(), base)
        }

        /// Builds a bidirectional runtime sequence.
        ///
        /// # Errors
        ///
        /// Rejects a foreign or degenerate base.
        pub fn bidirectional_sequence(
            &self,
            base: DynElement,
        ) -> Result<DynamicBidirectionalSequenceSignature<E>, SignatureError> {
            DynamicBidirectionalSequenceSignature::new(
                self.field.clone(),
                self.encoder.clone(),
                base,
            )
        }

        /// Builds a runtime multiset at one affine offset.
        ///
        /// # Errors
        ///
        /// Rejects an offset from another field.
        pub fn multiset(
            &self,
            offset: DynElement,
        ) -> Result<DynamicMultisetSignature<E>, SignatureError> {
            DynamicMultisetSignature::new(self.field.clone(), self.encoder.clone(), offset)
        }

        /// Builds a runtime multiset at distinct evaluation offsets.
        ///
        /// # Errors
        ///
        /// Rejects empty, duplicate or foreign offsets.
        pub fn multi_evaluation_multiset(
            &self,
            offsets: impl Into<Vec<DynElement>>,
        ) -> Result<DynamicMultiEvaluationMultisetSignature<E>, SignatureError> {
            DynamicMultiEvaluationMultisetSignature::new(
                self.field.clone(),
                self.encoder.clone(),
                offsets,
            )
        }

        /// Builds a runtime sequence at distinct evaluation bases.
        ///
        /// # Errors
        ///
        /// Rejects empty, duplicate, foreign or degenerate bases.
        pub fn multi_evaluation_sequence(
            &self,
            bases: impl Into<Vec<DynElement>>,
        ) -> Result<DynamicMultiEvaluationSequenceSignature<E>, SignatureError> {
            DynamicMultiEvaluationSequenceSignature::new(
                self.field.clone(),
                self.encoder.clone(),
                bases,
            )
        }
    }
}

#[cfg(any(feature = "dynamic-signatures", feature = "dynamic-fields"))]
pub use dynamic::DynamicSignatureBuilder;
