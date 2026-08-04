//! Canonical binary envelope shared by structural states.

use microfield::FieldId;

use super::{EncoderId, SignatureContext, SignatureError, SignatureId, SignatureLaw};

const MAGIC: [u8; 4] = *b"MFSG";
const SCHEMA: u16 = 1;
pub(crate) const HEADER_BYTES: usize = 104;

pub(crate) fn encode_header(output: &mut Vec<u8>, context: SignatureContext) {
    output.extend_from_slice(&MAGIC);
    output.extend_from_slice(&SCHEMA.to_le_bytes());
    output.push(context.law() as u8);
    output.push(0);
    output.extend_from_slice(context.field_id().as_bytes());
    output.extend_from_slice(context.encoder_id().as_bytes());
    output.extend_from_slice(context.signature_id().as_bytes());
}

pub(crate) fn verify_header(
    bytes: &[u8],
    expected: SignatureContext,
) -> Result<(), SignatureError> {
    if bytes.len() < HEADER_BYTES {
        return Err(SignatureError::InvalidWireFormat("truncated header"));
    }
    if bytes[..4] != MAGIC || u16::from_le_bytes([bytes[4], bytes[5]]) != SCHEMA {
        return Err(SignatureError::InvalidWireFormat("magic or schema"));
    }
    if bytes[6] != expected.law() as u8 || bytes[7] != 0 {
        return Err(SignatureError::InvalidWireFormat("law or reserved byte"));
    }
    let field = FieldId::from_bytes(bytes[8..40].try_into().expect("header range"));
    let encoder = EncoderId::from_bytes(bytes[40..72].try_into().expect("header range"));
    let signature = SignatureId::from_bytes(bytes[72..104].try_into().expect("header range"));
    if field != expected.field_id()
        || encoder != expected.encoder_id()
        || signature != expected.signature_id()
    {
        return Err(SignatureError::IdentityMismatch);
    }
    Ok(())
}

#[allow(dead_code)]
const _: SignatureLaw = SignatureLaw::Additive;
