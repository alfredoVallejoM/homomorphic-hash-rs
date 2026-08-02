//! Immutable metadata associated with maintained field presentations.

use crate::{ArtifactId, FieldId};

/// Exact characteristic of a field without imposing a machine-word limit.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Characteristic {
    decimal: &'static str,
    small: Option<u64>,
}

impl Characteristic {
    /// Creates metadata emitted by a certified generator.
    #[doc(hidden)]
    #[must_use]
    pub const fn __from_generated(decimal: &'static str, small: Option<u64>) -> Self {
        Self { decimal, small }
    }

    /// Returns the canonical unsigned decimal representation.
    #[must_use]
    pub const fn decimal(self) -> &'static str {
        self.decimal
    }

    /// Returns the characteristic as `u64` when representable.
    #[must_use]
    pub const fn as_u64(self) -> Option<u64> {
        self.small
    }
}

impl PartialEq<u64> for Characteristic {
    fn eq(&self, other: &u64) -> bool {
        self.small == Some(*other)
    }
}

/// Generated metadata for a statically maintained field.
#[derive(Debug)]
pub struct StaticFieldSpec {
    pub(crate) field_id: FieldId,
    pub(crate) artifact_id: ArtifactId,
    pub(crate) name: &'static str,
    pub(crate) characteristic: Characteristic,
    pub(crate) degree: u32,
    pub(crate) canonical_bytes: u16,
    pub(crate) descriptor_json: &'static [u8],
    pub(crate) certificate_json: &'static [u8],
}

impl StaticFieldSpec {
    /// Constructs metadata embedded by the certified source generator.
    ///
    /// This entry point is hidden because handwritten callers should not claim
    /// certification. Generated modules use it to keep metadata immutable and
    /// allocation-free.
    #[doc(hidden)]
    #[must_use]
    pub const fn __from_generated(
        field_id: FieldId,
        artifact_id: ArtifactId,
        name: &'static str,
        degree: u32,
        canonical_bytes: u16,
        descriptor_json: &'static [u8],
        certificate_json: &'static [u8],
    ) -> Self {
        Self {
            field_id,
            artifact_id,
            name,
            characteristic: Characteristic::__from_generated("2", Some(2)),
            degree,
            canonical_bytes,
            descriptor_json,
            certificate_json,
        }
    }

    /// Constructs prime-field metadata emitted by a certified generator.
    ///
    /// This compatibility boundary exists so generated source in downstream
    /// crates can retain truthful field identity while using persistent batch
    /// plans. Handwritten callers must not use it to claim certification.
    #[doc(hidden)]
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn __from_generated_prime(
        field_id: FieldId,
        artifact_id: ArtifactId,
        name: &'static str,
        characteristic_decimal: &'static str,
        characteristic_small: Option<u64>,
        degree: u32,
        canonical_bytes: u16,
        descriptor_json: &'static [u8],
        certificate_json: &'static [u8],
    ) -> Self {
        Self {
            field_id,
            artifact_id,
            name,
            characteristic: Characteristic::__from_generated(
                characteristic_decimal,
                characteristic_small,
            ),
            degree,
            canonical_bytes,
            descriptor_json,
            certificate_json,
        }
    }

    /// Returns the stable field identity.
    #[must_use]
    pub const fn field_id(&self) -> FieldId {
        self.field_id
    }

    /// Returns the identity of the generated portable representation.
    #[must_use]
    pub const fn artifact_id(&self) -> ArtifactId {
        self.artifact_id
    }

    /// Returns the maintained presentation name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Returns the field characteristic.
    #[must_use]
    pub const fn characteristic(&self) -> Characteristic {
        self.characteristic
    }

    /// Returns the extension degree.
    #[must_use]
    pub const fn degree(&self) -> u32 {
        self.degree
    }

    /// Returns the canonical representation size in bytes.
    #[must_use]
    pub const fn canonical_bytes(&self) -> u16 {
        self.canonical_bytes
    }

    /// Returns the canonical descriptor JSON.
    #[must_use]
    pub const fn descriptor_json(&self) -> &'static [u8] {
        self.descriptor_json
    }

    /// Returns the validation certificate JSON.
    #[must_use]
    pub const fn certificate_json(&self) -> &'static [u8] {
        self.certificate_json
    }
}
