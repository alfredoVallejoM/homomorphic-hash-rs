//! Stable identities separating application semantics from analysis profiles.

use core::fmt;

use sha2::{Digest as _, Sha256};

/// Identity of the application schema used to interpret graph labels and roles.
///
/// Names, field choices and execution profiles are deliberately excluded. Two
/// applications should share this identity only when their exact graph bytes
/// have the same semantics.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct GraphSchemaId([u8; 32]);

impl GraphSchemaId {
    /// Derives a stable identity from an owned, versioned schema descriptor.
    #[must_use]
    pub fn derive(descriptor: &[u8]) -> Self {
        Self(derive_id(b"microfield/graph-schema-id/v1\0", descriptor))
    }

    /// Creates an identity from already validated stable bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Generic semantics of the built-in directed relational incidence model.
    #[must_use]
    pub fn generic_incidence_v1() -> Self {
        Self::derive(b"microfield/incidence-graph/generic-semantics/v1")
    }

    /// Borrows the stable identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Default for GraphSchemaId {
    fn default() -> Self {
        Self::generic_incidence_v1()
    }
}

impl fmt::Display for GraphSchemaId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(formatter, &self.0)
    }
}

impl fmt::Debug for GraphSchemaId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "GraphSchemaId({self})")
    }
}

/// Identity of a non-authoritative fingerprint/refinement configuration.
///
/// This identity may include fields, lanes, evaluation points and policies. It
/// must never be embedded into exact canonical graph bytes.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct GraphAnalysisProfileId([u8; 32]);

impl GraphAnalysisProfileId {
    /// Derives a stable identity from a complete canonical profile descriptor.
    #[must_use]
    pub fn derive(descriptor: &[u8]) -> Self {
        Self(derive_id(
            b"microfield/graph-analysis-profile-id/v1\0",
            descriptor,
        ))
    }

    /// Creates an identity from already validated stable bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Borrows the stable identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for GraphAnalysisProfileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(formatter, &self.0)
    }
}

impl fmt::Debug for GraphAnalysisProfileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "GraphAnalysisProfileId({self})")
    }
}

fn derive_id(domain: &[u8], descriptor: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update((descriptor.len() as u64).to_le_bytes());
    hasher.update(descriptor);
    hasher.finalize().into()
}

fn write_hex(formatter: &mut fmt::Formatter<'_>, bytes: &[u8]) -> fmt::Result {
    for byte in bytes {
        write!(formatter, "{byte:02x}")?;
    }
    Ok(())
}
