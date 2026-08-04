//! Explicit byte-to-field encoders.

use microfield::{BinaryPolynomialField, CanonicalEncoding, Field, Gf2_256HhV1, PrimeField};
use sha2::{Digest as _, Sha256};

use super::{EncoderId, SignatureError};

const DEFAULT_MAXIMUM_INPUT_BYTES: usize = 16 * 1024 * 1024;
const INLINE_INPUT_BYTES: usize = 256;
const INLINE_FRAMED_BYTES: usize = INLINE_INPUT_BYTES + 9;
const DEFAULT_HASH_TO_FIELD_ATTEMPTS: u64 = 4_096;

/// A deterministic, identified mapping from byte strings to one field.
pub trait StructuralEncoder<F: Field>: Clone + Send + Sync + 'static {
    /// Encodes one byte string.
    ///
    /// # Errors
    ///
    /// Rejects resource-limit or canonicality violations.
    fn encode(&self, data: &[u8]) -> Result<F, SignatureError>;

    /// Stable identity of the encoding semantics, excluding resource ceilings.
    #[must_use]
    fn encoder_id(&self) -> EncoderId;
}

/// Encoder that produces independently domain-separated field coordinates.
pub trait StructuralLaneEncoder<F: Field, const K: usize>: Clone + Send + Sync + 'static {
    /// Encodes one source value independently in every lane.
    ///
    /// # Errors
    ///
    /// Rejects resource limits or exhausted canonical rejection sampling.
    fn encode_lanes(&self, data: &[u8]) -> Result<[F; K], SignatureError>;

    /// Stable identity of profile, channel, lane count and encoding algorithm.
    #[must_use]
    fn encoder_id(&self) -> EncoderId;
}

/// SHA-256 expansion followed by deterministic canonical rejection per lane.
///
/// SHA-256 is used as a stable mixer and domain separator. No cryptographic
/// security claim is made for signatures that consume these field elements.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DomainSeparatedHashToFieldEncoder<const K: usize> {
    profile_id: [u8; 32],
    channel: u64,
    maximum_input_bytes: usize,
    maximum_attempts: u64,
}

impl<const K: usize> DomainSeparatedHashToFieldEncoder<K> {
    /// Creates a lane encoder bound to one analysis profile and channel.
    #[must_use]
    pub const fn new(profile_id: [u8; 32], channel: u64) -> Self {
        Self {
            profile_id,
            channel,
            maximum_input_bytes: DEFAULT_MAXIMUM_INPUT_BYTES,
            maximum_attempts: DEFAULT_HASH_TO_FIELD_ATTEMPTS,
        }
    }

    /// Replaces the defensive source-size ceiling without changing identity.
    #[must_use]
    pub const fn with_maximum_input_bytes(mut self, maximum: usize) -> Self {
        self.maximum_input_bytes = maximum;
        self
    }

    /// Replaces the defensive rejection ceiling without changing identity.
    #[must_use]
    pub const fn with_maximum_attempts(mut self, maximum: u64) -> Self {
        self.maximum_attempts = maximum;
        self
    }

    /// Analysis profile bound into every lane.
    #[must_use]
    pub const fn profile_id(self) -> [u8; 32] {
        self.profile_id
    }

    /// Semantic channel bound into every lane.
    #[must_use]
    pub const fn channel(self) -> u64 {
        self.channel
    }

    /// Stable identity excluding defensive work ceilings.
    #[must_use]
    pub fn id(self) -> EncoderId {
        let mut descriptor = Vec::with_capacity(48);
        descriptor.extend_from_slice(b"sha256-canonical-rejection-lanes-v1\0");
        descriptor.extend_from_slice(&self.profile_id);
        descriptor.extend_from_slice(&self.channel.to_le_bytes());
        descriptor.extend_from_slice(&u64::try_from(K).unwrap_or(u64::MAX).to_le_bytes());
        EncoderId::derive(&descriptor)
    }
}

impl<F, const K: usize> StructuralLaneEncoder<F, K> for DomainSeparatedHashToFieldEncoder<K>
where
    F: Field + CanonicalEncoding,
{
    fn encode_lanes(&self, data: &[u8]) -> Result<[F; K], SignatureError> {
        if data.len() > self.maximum_input_bytes {
            return Err(SignatureError::InputTooLarge {
                maximum: self.maximum_input_bytes,
                actual: data.len(),
            });
        }
        if K == 0 {
            return Err(SignatureError::InvalidEvaluationPoints);
        }
        let mut lanes = [F::ZERO; K];
        for (lane, output) in lanes.iter_mut().enumerate() {
            *output = hash_lane::<F>(
                self.profile_id,
                self.channel,
                u64::try_from(lane).map_err(|_| SignatureError::HashToFieldExhausted)?,
                data,
                self.maximum_attempts,
            )?;
        }
        Ok(lanes)
    }

    fn encoder_id(&self) -> EncoderId {
        self.id()
    }
}

fn hash_lane<F: Field + CanonicalEncoding>(
    profile_id: [u8; 32],
    channel: u64,
    lane: u64,
    data: &[u8],
    maximum_attempts: u64,
) -> Result<F, SignatureError> {
    let repr_len = F::ZERO.to_canonical().as_ref().len();
    let data_len = u64::try_from(data.len()).map_err(|_| SignatureError::HashToFieldExhausted)?;
    let mut candidate = Vec::new();
    candidate
        .try_reserve_exact(repr_len)
        .map_err(|_| SignatureError::AllocationFailed)?;
    for attempt in 0..maximum_attempts {
        candidate.clear();
        let mut block = 0_u64;
        while candidate.len() < repr_len {
            let mut hasher = Sha256::new();
            hasher.update(b"microfield/hash-to-field-lane/v1\0");
            hasher.update(profile_id);
            hasher.update(channel.to_le_bytes());
            hasher.update(lane.to_le_bytes());
            hasher.update(data_len.to_le_bytes());
            hasher.update(data);
            hasher.update(attempt.to_le_bytes());
            hasher.update(block.to_le_bytes());
            let digest = hasher.finalize();
            let remaining = repr_len - candidate.len();
            candidate.extend_from_slice(&digest[..remaining.min(digest.len())]);
            block = block
                .checked_add(1)
                .ok_or(SignatureError::HashToFieldExhausted)?;
        }
        if let Ok(element) = F::from_canonical_slice(&candidate) {
            return Ok(element);
        }
    }
    Err(SignatureError::HashToFieldExhausted)
}

/// Runtime-context equivalent of [`StructuralEncoder`].
///
/// This adapter is intentionally separate: static field signatures remain
/// monomorphized and do not carry an `Arc` or runtime field checks.
#[cfg(any(feature = "dynamic-signatures", feature = "dynamic-fields"))]
pub trait DynamicStructuralEncoder: Clone + Send + Sync + 'static {
    /// Encodes bytes under one validated runtime field context.
    ///
    /// # Errors
    ///
    /// Rejects resource, canonicality or field-family mismatches.
    fn encode_dynamic(
        &self,
        field: &microfield::DynField,
        data: &[u8],
    ) -> Result<microfield::DynElement, SignatureError>;

    /// Stable identity shared with the corresponding static encoder.
    #[must_use]
    fn encoder_id(&self) -> EncoderId;
}

/// Strictly decodes exactly one canonical element.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CanonicalElementEncoder;

impl CanonicalElementEncoder {
    /// Returns the stable identity of strict canonical decoding.
    #[must_use]
    pub fn id(self) -> EncoderId {
        EncoderId::derive(b"canonical-field-element-v1")
    }
}

impl<F> StructuralEncoder<F> for CanonicalElementEncoder
where
    F: Field + CanonicalEncoding,
{
    fn encode(&self, data: &[u8]) -> Result<F, SignatureError> {
        F::from_canonical_slice(data).map_err(|_| SignatureError::NonCanonicalElement)
    }

    fn encoder_id(&self) -> EncoderId {
        self.id()
    }
}

#[cfg(any(feature = "dynamic-signatures", feature = "dynamic-fields"))]
impl DynamicStructuralEncoder for CanonicalElementEncoder {
    fn encode_dynamic(
        &self,
        field: &microfield::DynField,
        data: &[u8],
    ) -> Result<microfield::DynElement, SignatureError> {
        field
            .decode(data)
            .map_err(|_| SignatureError::NonCanonicalElement)
    }

    fn encoder_id(&self) -> EncoderId {
        self.id()
    }
}

/// Length-framed polynomial reduction for binary extension fields.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BinaryPolynomialEncoder {
    domain_tag: u64,
    maximum_input_bytes: usize,
}

impl BinaryPolynomialEncoder {
    /// Creates a domain-separated encoder with a 16 MiB defensive ceiling.
    #[must_use]
    pub const fn new(domain_tag: u64) -> Self {
        Self {
            domain_tag,
            maximum_input_bytes: DEFAULT_MAXIMUM_INPUT_BYTES,
        }
    }

    /// Replaces the non-semantic resource ceiling.
    #[must_use]
    pub const fn with_maximum_input_bytes(mut self, maximum: usize) -> Self {
        self.maximum_input_bytes = maximum;
        self
    }

    /// Returns the domain-separation tag.
    #[must_use]
    pub const fn domain_tag(self) -> u64 {
        self.domain_tag
    }

    /// Returns the stable identity, independent of the resource ceiling.
    #[must_use]
    pub fn id(self) -> EncoderId {
        EncoderId::derive_tagged(
            b"binary-polynomial-little-endian-framed-v1\0",
            self.domain_tag,
        )
    }
}

impl<F> StructuralEncoder<F> for BinaryPolynomialEncoder
where
    F: Field + BinaryPolynomialField,
{
    fn encode(&self, data: &[u8]) -> Result<F, SignatureError> {
        let framed = frame(data, self.domain_tag, self.maximum_input_bytes)?;
        Ok(F::from_polynomial_bytes_mod(framed.as_slice()))
    }

    fn encoder_id(&self) -> EncoderId {
        self.id()
    }
}

#[cfg(any(feature = "dynamic-signatures", feature = "dynamic-fields"))]
impl DynamicStructuralEncoder for BinaryPolynomialEncoder {
    fn encode_dynamic(
        &self,
        field: &microfield::DynField,
        data: &[u8],
    ) -> Result<microfield::DynElement, SignatureError> {
        if field.family() != microfield::DynFamilyKind::BinaryPolynomial {
            return Err(SignatureError::EncoderFamilyMismatch);
        }
        let framed = frame(data, self.domain_tag, self.maximum_input_bytes)?;
        Ok(field.reduce_bytes_mod_order(framed.as_slice()))
    }

    fn encoder_id(&self) -> EncoderId {
        self.id()
    }
}

/// Length-framed integer reduction for prime fields.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrimeIntegerEncoder {
    domain_tag: u64,
    maximum_input_bytes: usize,
}

/// Byte-compatible adapter for the legacy symmetric-difference and sequence
/// embedding over `Gf2_256HhV1`.
///
/// It is preserved for migration only. The historical chunk scheme has known
/// collisions and does not frame total byte length.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LegacyLinearEncoderV1;

impl StructuralEncoder<Gf2_256HhV1> for LegacyLinearEncoderV1 {
    fn encode(&self, data: &[u8]) -> Result<Gf2_256HhV1, SignatureError> {
        let mut result = Gf2_256HhV1::ZERO;
        for chunk in data.chunks(32).rev() {
            let mut buffer = [0_u8; 32];
            buffer[..chunk.len()].copy_from_slice(chunk);
            let block = Gf2_256HhV1::from_canonical(&buffer)
                .expect("every 256-bit binary encoding is canonical");
            result = result.mul_by_x().add(block);
        }
        Ok(result)
    }

    fn encoder_id(&self) -> EncoderId {
        EncoderId::derive(b"legacy-gf2-256-linear-chunks-v1")
    }
}

/// Byte-compatible adapter for the legacy multiset affine embedding.
///
/// Clearing bit 255 did not globally prove that the later affine factor was
/// non-zero. New [`super::MultisetSignature`] therefore counts zero factors
/// instead of relying on that assumption.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LegacyAffineEncoderV1;

impl StructuralEncoder<Gf2_256HhV1> for LegacyAffineEncoderV1 {
    fn encode(&self, data: &[u8]) -> Result<Gf2_256HhV1, SignatureError> {
        let mut result = Gf2_256HhV1::ZERO;
        for chunk in data.chunks(32).rev() {
            let mut buffer = [0_u8; 32];
            buffer[..chunk.len()].copy_from_slice(chunk);
            buffer[31] &= 0x7f;
            let block = Gf2_256HhV1::from_canonical(&buffer)
                .expect("every 256-bit binary encoding is canonical");
            result = result.mul_by_x().add(block);
        }
        Ok(result)
    }

    fn encoder_id(&self) -> EncoderId {
        EncoderId::derive(b"legacy-gf2-256-affine-chunks-v1")
    }
}

impl PrimeIntegerEncoder {
    /// Creates a domain-separated encoder with a 16 MiB defensive ceiling.
    #[must_use]
    pub const fn new(domain_tag: u64) -> Self {
        Self {
            domain_tag,
            maximum_input_bytes: DEFAULT_MAXIMUM_INPUT_BYTES,
        }
    }

    /// Replaces the non-semantic resource ceiling.
    #[must_use]
    pub const fn with_maximum_input_bytes(mut self, maximum: usize) -> Self {
        self.maximum_input_bytes = maximum;
        self
    }

    /// Returns the domain-separation tag.
    #[must_use]
    pub const fn domain_tag(self) -> u64 {
        self.domain_tag
    }

    /// Returns the stable identity, independent of the resource ceiling.
    #[must_use]
    pub fn id(self) -> EncoderId {
        EncoderId::derive_tagged(b"prime-integer-little-endian-framed-v1\0", self.domain_tag)
    }
}

impl<F> StructuralEncoder<F> for PrimeIntegerEncoder
where
    F: Field + PrimeField,
{
    fn encode(&self, data: &[u8]) -> Result<F, SignatureError> {
        let framed = frame(data, self.domain_tag, self.maximum_input_bytes)?;
        Ok(F::from_bytes_mod_order(framed.as_slice()))
    }

    fn encoder_id(&self) -> EncoderId {
        self.id()
    }
}

#[cfg(any(feature = "dynamic-signatures", feature = "dynamic-fields"))]
impl DynamicStructuralEncoder for PrimeIntegerEncoder {
    fn encode_dynamic(
        &self,
        field: &microfield::DynField,
        data: &[u8],
    ) -> Result<microfield::DynElement, SignatureError> {
        if field.family() != microfield::DynFamilyKind::Prime {
            return Err(SignatureError::EncoderFamilyMismatch);
        }
        let framed = frame(data, self.domain_tag, self.maximum_input_bytes)?;
        Ok(field.reduce_bytes_mod_order(framed.as_slice()))
    }

    fn encoder_id(&self) -> EncoderId {
        self.id()
    }
}

// Boxing the inline variant would add a heap allocation to the common path.
#[allow(clippy::large_enum_variant)]
enum FramedBytes {
    Inline {
        bytes: [u8; INLINE_FRAMED_BYTES],
        length: usize,
    },
    Heap(Vec<u8>),
}

impl FramedBytes {
    fn as_slice(&self) -> &[u8] {
        match self {
            Self::Inline { bytes, length } => &bytes[..*length],
            Self::Heap(bytes) => bytes,
        }
    }
}

fn frame(data: &[u8], domain_tag: u64, maximum: usize) -> Result<FramedBytes, SignatureError> {
    if data.len() > maximum {
        return Err(SignatureError::InputTooLarge {
            maximum,
            actual: data.len(),
        });
    }
    let capacity = data
        .len()
        .checked_add(9)
        .ok_or(SignatureError::CounterOverflow)?;
    if data.len() <= INLINE_INPUT_BYTES {
        let mut bytes = [0_u8; INLINE_FRAMED_BYTES];
        bytes[..data.len()].copy_from_slice(data);
        bytes[data.len()] = 1;
        bytes[data.len() + 1..capacity].copy_from_slice(&domain_tag.to_le_bytes());
        return Ok(FramedBytes::Inline {
            bytes,
            length: capacity,
        });
    }
    let mut framed = Vec::new();
    framed
        .try_reserve_exact(capacity)
        .map_err(|_| SignatureError::AllocationFailed)?;
    framed.extend_from_slice(data);
    framed.push(1);
    framed.extend_from_slice(&domain_tag.to_le_bytes());
    Ok(FramedBytes::Heap(framed))
}
