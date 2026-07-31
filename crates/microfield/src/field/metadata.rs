//! Immutable metadata associated with maintained field presentations.

use crate::FieldId;

/// Generated metadata for a statically maintained field.
#[derive(Debug)]
pub struct StaticFieldSpec {
    pub(crate) field_id: FieldId,
    pub(crate) name: &'static str,
    pub(crate) characteristic: u64,
    pub(crate) degree: u32,
    pub(crate) canonical_bytes: u16,
    pub(crate) descriptor_json: &'static [u8],
    pub(crate) certificate_json: &'static [u8],
}

impl StaticFieldSpec {
    /// Returns the stable field identity.
    #[must_use]
    pub const fn field_id(&self) -> FieldId {
        self.field_id
    }

    /// Returns the maintained presentation name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Returns the field characteristic.
    #[must_use]
    pub const fn characteristic(&self) -> u64 {
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
