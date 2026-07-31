//! Domain-separated identities for fields and generated artifacts.

use sha2::{Digest, Sha256};

use crate::{ArtifactBundleDigest, ArtifactId, FieldId};

const FIELD_DOMAIN: &[u8] = b"microfield:field-id:v1\0";
const ARTIFACT_DOMAIN: &[u8] = b"microfield:artifact-id:v1\0";
const ARTIFACT_BUNDLE_DOMAIN: &[u8] = b"microfield:artifact-bundle:v1\0";

pub(crate) fn field_id(identity_bytes: &[u8]) -> FieldId {
    FieldId::from_bytes(digest(FIELD_DOMAIN, identity_bytes))
}

pub(crate) fn artifact_id(descriptor_bytes: &[u8]) -> ArtifactId {
    ArtifactId::from_bytes(digest(ARTIFACT_DOMAIN, descriptor_bytes))
}

pub(crate) fn artifact_bundle_digest(descriptor_bytes: &[u8]) -> ArtifactBundleDigest {
    ArtifactBundleDigest::from_bytes(digest(ARTIFACT_BUNDLE_DOMAIN, descriptor_bytes))
}

pub(crate) fn content_digest(bytes: &[u8]) -> String {
    hex(&digest(&[], bytes))
}

pub(crate) fn proof_digest(plan_bytes: &[u8]) -> String {
    hex(&digest(b"microfield:reduction-plan:v1\0", plan_bytes))
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(char::from(HEX[usize::from(byte >> 4)]));
        result.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    result
}

fn digest(domain: &[u8], bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(bytes);
    hasher.finalize().into()
}
