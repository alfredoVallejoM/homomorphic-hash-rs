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

    /// Builds an identifier from generated lowercase hexadecimal text.
    #[doc(hidden)]
    #[must_use]
    pub const fn __from_generated_hex(hex: &str) -> Self {
        Self(decode_digest(hex))
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

    /// Builds an identifier from generated lowercase hexadecimal text.
    #[doc(hidden)]
    #[must_use]
    pub const fn __from_generated_hex(hex: &str) -> Self {
        Self(decode_digest(hex))
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

const fn decode_digest(hex: &str) -> [u8; 32] {
    let bytes = hex.as_bytes();
    assert!(bytes.len() == 64);
    let mut out = [0_u8; 32];
    let mut index = 0;
    while index < out.len() {
        out[index] = (decode_nibble(bytes[index * 2]) << 4) | decode_nibble(bytes[index * 2 + 1]);
        index += 1;
    }
    out
}

const fn decode_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => panic!("generated digest must use lowercase hexadecimal"),
    }
}
