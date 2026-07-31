//! Stable value objects identifying fields and generated artifacts.

use core::fmt;

/// Stable identity of field semantics and canonical encoding.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct FieldId([u8; 32]);

impl FieldId {
    /// Builds an identifier from its serialized 32-byte digest.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Borrows the serialized digest.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Returns the serialized digest.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for FieldId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(formatter, &self.0)
    }
}

impl fmt::Debug for FieldId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("FieldId(")?;
        write_hex(formatter, &self.0)?;
        formatter.write_str(")")
    }
}

#[cfg(feature = "generator")]
impl serde::Serialize for FieldId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

/// Identity of a concrete generated representation of a field.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct ArtifactId([u8; 32]);

impl ArtifactId {
    /// Builds an identifier from its serialized 32-byte digest.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Borrows the serialized digest.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Returns the serialized digest.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for ArtifactId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(formatter, &self.0)
    }
}

impl fmt::Debug for ArtifactId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ArtifactId(")?;
        write_hex(formatter, &self.0)?;
        formatter.write_str(")")
    }
}

#[cfg(feature = "generator")]
impl serde::Serialize for ArtifactId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

/// Integrity digest of the exact files in a generated artifact bundle.
///
/// Unlike [`ArtifactId`], this value changes when presentation-only bytes such
/// as the field name change.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct ArtifactBundleDigest([u8; 32]);

impl ArtifactBundleDigest {
    /// Builds a digest from its serialized 32-byte representation.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Borrows the serialized digest.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Returns the serialized digest.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for ArtifactBundleDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(formatter, &self.0)
    }
}

impl fmt::Debug for ArtifactBundleDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ArtifactBundleDigest(")?;
        write_hex(formatter, &self.0)?;
        formatter.write_str(")")
    }
}

#[cfg(feature = "generator")]
impl serde::Serialize for ArtifactBundleDigest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

fn write_hex(formatter: &mut fmt::Formatter<'_>, bytes: &[u8]) -> fmt::Result {
    for byte in bytes {
        write!(formatter, "{byte:02x}")?;
    }
    Ok(())
}
