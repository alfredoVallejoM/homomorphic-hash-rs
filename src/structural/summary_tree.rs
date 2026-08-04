//! Fixed-chunk file adapter and hierarchical ordered summaries.

use core::{fmt, ops::Range};

use microfield::{CanonicalEncoding, Field, Invert, Pow, StaticField};
use sha2::{Digest as _, Sha256};

use super::{SequenceSignature, SignatureContext, SignatureError, StructuralEncoder};

const CHECKPOINT_MAGIC: &[u8; 4] = b"MFST";
const CHECKPOINT_SCHEMA: u16 = 1;
const CHECKPOINT_HEADER_BYTES: usize = 64;
const CHUNK_FRAME_MAGIC: &[u8; 4] = b"MFFC";

/// Stable identity of file chunking and framing semantics.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct FileChunkProfileId([u8; 32]);

impl FileChunkProfileId {
    /// Borrows the canonical identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for FileChunkProfileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.as_bytes() {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for FileChunkProfileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "FileChunkProfileId({self})")
    }
}

/// Frozen fixed-size chunking and leaf-framing profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileChunkProfile {
    chunk_bytes: usize,
    profile_id: FileChunkProfileId,
}

impl FileChunkProfile {
    /// Creates the maintained fixed-chunk profile.
    ///
    /// The final chunk may be shorter. Every leaf is framed with `MFFC`, schema
    /// and byte length before it reaches the structural encoder.
    pub fn fixed(chunk_bytes: usize) -> Result<Self, SummaryTreeError> {
        if chunk_bytes == 0 {
            return Err(SummaryTreeError::InvalidChunkProfile);
        }
        let mut hasher = Sha256::new();
        hasher.update(b"microfield-file-chunk-profile-v1\0");
        hasher.update(b"fixed-size\0mffc-v1\0binary-pairwise-promote-odd-v1\0");
        hasher.update((chunk_bytes as u64).to_le_bytes());
        Ok(Self {
            chunk_bytes,
            profile_id: FileChunkProfileId(hasher.finalize().into()),
        })
    }

    /// Nominal bytes per complete chunk.
    #[must_use]
    pub const fn chunk_bytes(self) -> usize {
        self.chunk_bytes
    }

    /// Complete chunking/framing identity.
    #[must_use]
    pub const fn profile_id(self) -> FileChunkProfileId {
        self.profile_id
    }
}

/// Defensive construction and checkpoint ceilings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SummaryTreeLimits {
    /// Maximum exact file bytes retained by one tree.
    pub max_file_bytes: usize,
    /// Maximum number of leaves.
    pub max_chunks: usize,
    /// Maximum admitted configured chunk size.
    pub max_chunk_bytes: usize,
    /// Maximum serialized checkpoint size.
    pub max_checkpoint_bytes: usize,
}

impl Default for SummaryTreeLimits {
    fn default() -> Self {
        Self {
            max_file_bytes: 256 * 1024 * 1024,
            max_chunks: 1_000_000,
            max_chunk_bytes: 16 * 1024 * 1024,
            max_checkpoint_bytes: 512 * 1024 * 1024,
        }
    }
}

/// Failure produced before a summary tree publishes a partial state.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SummaryTreeError {
    /// The structural signature or encoder rejected an operation.
    Signature(SignatureError),
    /// Chunk size zero or outside the configured ceiling.
    InvalidChunkProfile,
    /// An edit range is reversed or outside the current file.
    InvalidRange,
    /// An edit or checkpoint exceeds a defensive ceiling.
    LimitExceeded(&'static str),
    /// The tree revision cannot advance.
    RevisionOverflow,
    /// A checkpoint is truncated, inconsistent or belongs to another profile.
    InvalidCheckpoint(&'static str),
    /// Allocation failed before publication.
    AllocationFailed,
}

impl fmt::Display for SummaryTreeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Signature(error) => error.fmt(formatter),
            Self::InvalidChunkProfile => formatter.write_str("invalid fixed chunk profile"),
            Self::InvalidRange => formatter.write_str("invalid file edit range"),
            Self::LimitExceeded(limit) => write!(formatter, "summary tree exceeds {limit} limit"),
            Self::RevisionOverflow => formatter.write_str("summary tree revision overflow"),
            Self::InvalidCheckpoint(reason) => {
                write!(formatter, "invalid summary tree checkpoint: {reason}")
            }
            Self::AllocationFailed => formatter.write_str("summary tree allocation failed"),
        }
    }
}

impl std::error::Error for SummaryTreeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Signature(error) => Some(error),
            _ => None,
        }
    }
}

impl From<SignatureError> for SummaryTreeError {
    fn from(error: SignatureError) -> Self {
        Self::Signature(error)
    }
}

/// Profile-bound root exposed to callers instead of a bare field evaluation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HomomorphicSummaryRoot<F: Field> {
    profile_id: FileChunkProfileId,
    context: SignatureContext,
    evaluation: F,
    byte_len: u64,
    chunk_count: u64,
}

impl<F: Field> HomomorphicSummaryRoot<F> {
    /// Chunk/framing identity.
    #[must_use]
    pub const fn profile_id(self) -> FileChunkProfileId {
        self.profile_id
    }

    /// Field, encoder, law and base identity.
    #[must_use]
    pub const fn context(self) -> SignatureContext {
        self.context
    }

    /// Ordered finite-field evaluation.
    #[must_use]
    pub const fn evaluation(self) -> F {
        self.evaluation
    }

    /// Exact file length represented by this root.
    #[must_use]
    pub const fn byte_len(self) -> u64 {
        self.byte_len
    }

    /// Exact number of framed leaves.
    #[must_use]
    pub const fn chunk_count(self) -> u64 {
        self.chunk_count
    }
}

impl<F> HomomorphicSummaryRoot<F>
where
    F: Field + CanonicalEncoding,
{
    /// Serializes a profile-bound root descriptor as `MFSR` schema 1.
    #[must_use]
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let representation = self.evaluation.to_canonical();
        let mut bytes = Vec::with_capacity(160 + representation.as_ref().len());
        bytes.extend_from_slice(b"MFSR");
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&[0, 0]);
        bytes.extend_from_slice(self.profile_id.as_bytes());
        bytes.extend_from_slice(self.context.field_id().as_bytes());
        bytes.extend_from_slice(self.context.encoder_id().as_bytes());
        bytes.extend_from_slice(self.context.signature_id().as_bytes());
        bytes.push(self.context.law() as u8);
        bytes.extend_from_slice(&[0; 7]);
        bytes.extend_from_slice(&self.byte_len.to_le_bytes());
        bytes.extend_from_slice(&self.chunk_count.to_le_bytes());
        bytes.extend_from_slice(representation.as_ref());
        bytes
    }
}

/// Update route selected by the fixed-chunk adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SummaryEditPath {
    /// Replacement was byte-for-byte identical.
    NoChange,
    /// Chunk boundaries stayed fixed and only affected ancestor paths changed.
    LocalTree,
    /// File length changed, so fixed boundaries required a complete rebuild.
    BoundaryRebuild,
}

/// Observable work and revision of one committed edit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SummaryEditReport {
    path: SummaryEditPath,
    touched_leaves: usize,
    recomputed_nodes: usize,
    revision: u64,
}

impl SummaryEditReport {
    /// Selected update route.
    #[must_use]
    pub const fn path(self) -> SummaryEditPath {
        self.path
    }

    /// Leaves whose exact bytes changed.
    #[must_use]
    pub const fn touched_leaves(self) -> usize {
        self.touched_leaves
    }

    /// Leaf and ancestor summaries recomputed before commit.
    #[must_use]
    pub const fn recomputed_nodes(self) -> usize {
        self.recomputed_nodes
    }

    /// Published tree revision.
    #[must_use]
    pub const fn revision(self) -> u64 {
        self.revision
    }
}

/// Exact fixed-chunk file storage plus an ordered hierarchical summary.
///
/// Internal nodes use the sequence concatenation law. For a single-leaf edit,
/// the maintained shape recomputes one node per level. The exact chunks remain
/// the source of truth; the root is a non-cryptographic algebraic fingerprint.
#[derive(Clone, Debug)]
pub struct HomomorphicSummaryTree<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    profile: FileChunkProfile,
    encoder: E,
    base: F,
    chunks: Vec<Vec<u8>>,
    levels: Vec<Vec<SequenceSignature<F, E>>>,
    byte_len: usize,
    revision: u64,
    limits: SummaryTreeLimits,
}

impl<F, E> HomomorphicSummaryTree<F, E>
where
    F: Field + CanonicalEncoding + StaticField + Pow + Invert,
    E: StructuralEncoder<F>,
{
    /// Builds an exact tree under default defensive limits.
    pub fn from_bytes(
        profile: FileChunkProfile,
        encoder: E,
        base: F,
        bytes: &[u8],
    ) -> Result<Self, SummaryTreeError> {
        Self::from_bytes_with_limits(profile, encoder, base, bytes, SummaryTreeLimits::default())
    }

    /// Builds an exact tree under explicit defensive limits.
    pub fn from_bytes_with_limits(
        profile: FileChunkProfile,
        encoder: E,
        base: F,
        bytes: &[u8],
        limits: SummaryTreeLimits,
    ) -> Result<Self, SummaryTreeError> {
        Self::build(profile, encoder, base, bytes, limits, 0)
    }

    fn build(
        profile: FileChunkProfile,
        encoder: E,
        base: F,
        bytes: &[u8],
        limits: SummaryTreeLimits,
        revision: u64,
    ) -> Result<Self, SummaryTreeError> {
        validate_admission(profile, bytes.len(), limits)?;
        let chunk_count = chunk_count(bytes.len(), profile.chunk_bytes)?;
        if chunk_count > limits.max_chunks {
            return Err(SummaryTreeError::LimitExceeded("chunks"));
        }
        let mut chunks = Vec::new();
        chunks
            .try_reserve_exact(chunk_count)
            .map_err(|_| SummaryTreeError::AllocationFailed)?;
        for chunk in bytes.chunks(profile.chunk_bytes) {
            let mut owned = Vec::new();
            owned
                .try_reserve_exact(chunk.len())
                .map_err(|_| SummaryTreeError::AllocationFailed)?;
            owned.extend_from_slice(chunk);
            chunks.push(owned);
        }
        let levels = build_levels(&chunks, encoder.clone(), base)?;
        Ok(Self {
            profile,
            encoder,
            base,
            chunks,
            levels,
            byte_len: bytes.len(),
            revision,
            limits,
        })
    }

    /// Profile-bound algebraic root and exact size metadata.
    #[must_use]
    pub fn root(&self) -> HomomorphicSummaryRoot<F> {
        let root = self.root_signature();
        HomomorphicSummaryRoot {
            profile_id: self.profile.profile_id,
            context: root.context(),
            evaluation: root.state(),
            byte_len: self.byte_len as u64,
            chunk_count: self.chunks.len() as u64,
        }
    }

    /// Compact sequence signature stored at the root.
    #[must_use]
    pub fn root_signature(&self) -> &SequenceSignature<F, E> {
        &self.levels.last().expect("a tree always has one level")[0]
    }

    /// Current exact file length.
    #[must_use]
    pub const fn byte_len(&self) -> usize {
        self.byte_len
    }

    /// Current number of fixed-size leaves.
    #[must_use]
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Current committed edit revision.
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Frozen chunk profile.
    #[must_use]
    pub const fn profile(&self) -> FileChunkProfile {
        self.profile
    }

    /// Materializes the exact retained file bytes.
    pub fn to_file_bytes(&self) -> Result<Vec<u8>, SummaryTreeError> {
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(self.byte_len)
            .map_err(|_| SummaryTreeError::AllocationFailed)?;
        for chunk in &self.chunks {
            bytes.extend_from_slice(chunk);
        }
        debug_assert_eq!(bytes.len(), self.byte_len);
        Ok(bytes)
    }

    /// Replaces one byte range transactionally.
    ///
    /// Equal-length replacement preserves boundaries and updates only affected
    /// paths. A length change rebuilds the fixed-chunk shape before publication.
    pub fn replace_range(
        &mut self,
        range: Range<usize>,
        replacement: &[u8],
    ) -> Result<SummaryEditReport, SummaryTreeError> {
        validate_range(&range, self.byte_len)?;
        if range.start == range.end && replacement.is_empty() {
            return Ok(self.no_change_report());
        }
        if range.len() == replacement.len() {
            self.replace_fixed(range, replacement)
        } else {
            self.replace_with_rebuild(range, replacement)
        }
    }

    /// Inserts bytes at one exact offset using the boundary-rebuild route.
    pub fn insert_range(
        &mut self,
        offset: usize,
        bytes: &[u8],
    ) -> Result<SummaryEditReport, SummaryTreeError> {
        self.replace_range(offset..offset, bytes)
    }

    /// Removes one exact byte range using the boundary-rebuild route.
    pub fn remove_range(
        &mut self,
        range: Range<usize>,
    ) -> Result<SummaryEditReport, SummaryTreeError> {
        self.replace_range(range, &[])
    }

    /// Appends bytes to the exact file.
    pub fn append(&mut self, bytes: &[u8]) -> Result<SummaryEditReport, SummaryTreeError> {
        self.insert_range(self.byte_len, bytes)
    }

    /// Truncates the exact file to `new_len` bytes.
    pub fn truncate(&mut self, new_len: usize) -> Result<SummaryEditReport, SummaryTreeError> {
        if new_len > self.byte_len {
            return Err(SummaryTreeError::InvalidRange);
        }
        self.remove_range(new_len..self.byte_len)
    }

    fn replace_fixed(
        &mut self,
        range: Range<usize>,
        replacement: &[u8],
    ) -> Result<SummaryEditReport, SummaryTreeError> {
        if range.is_empty() {
            return Ok(self.no_change_report());
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(SummaryTreeError::RevisionOverflow)?;
        let first = range.start / self.profile.chunk_bytes;
        let last = (range.end - 1) / self.profile.chunk_bytes;
        let mut changed = Vec::new();
        changed
            .try_reserve_exact(last - first + 1)
            .map_err(|_| SummaryTreeError::AllocationFailed)?;
        for index in first..=last {
            let chunk_start = index * self.profile.chunk_bytes;
            let overlap_start = range.start.max(chunk_start);
            let overlap_end = range.end.min(chunk_start + self.chunks[index].len());
            let mut chunk = Vec::new();
            chunk
                .try_reserve_exact(self.chunks[index].len())
                .map_err(|_| SummaryTreeError::AllocationFailed)?;
            chunk.extend_from_slice(&self.chunks[index]);
            let local_start = overlap_start - chunk_start;
            let local_end = overlap_end - chunk_start;
            let source_start = overlap_start - range.start;
            let source_end = overlap_end - range.start;
            chunk[local_start..local_end].copy_from_slice(&replacement[source_start..source_end]);
            if chunk != self.chunks[index] {
                changed.push((index, chunk));
            }
        }
        if changed.is_empty() {
            return Ok(self.no_change_report());
        }

        let mut candidates: Vec<Vec<(usize, SequenceSignature<F, E>)>> = Vec::new();
        candidates
            .try_reserve_exact(self.levels.len())
            .map_err(|_| SummaryTreeError::AllocationFailed)?;
        let mut leaves = Vec::new();
        leaves
            .try_reserve_exact(changed.len())
            .map_err(|_| SummaryTreeError::AllocationFailed)?;
        for (index, chunk) in &changed {
            leaves.push((
                *index,
                leaf_signature(self.encoder.clone(), self.base, chunk)?,
            ));
        }
        candidates.push(leaves);

        for level in 1..self.levels.len() {
            let previous_changed = &candidates[level - 1];
            let mut parent_indexes = Vec::new();
            parent_indexes
                .try_reserve_exact(previous_changed.len())
                .map_err(|_| SummaryTreeError::AllocationFailed)?;
            for (index, _) in previous_changed {
                let parent = index / 2;
                if parent_indexes.last().copied() != Some(parent) {
                    parent_indexes.push(parent);
                }
            }
            let mut parents = Vec::new();
            parents
                .try_reserve_exact(parent_indexes.len())
                .map_err(|_| SummaryTreeError::AllocationFailed)?;
            for parent in parent_indexes {
                let left_index = parent * 2;
                let left =
                    candidate_or_existing(previous_changed, &self.levels[level - 1], left_index);
                let right_index = left_index + 1;
                let value = if right_index < self.levels[level - 1].len() {
                    let right = candidate_or_existing(
                        previous_changed,
                        &self.levels[level - 1],
                        right_index,
                    );
                    left.concatenate(right)?
                } else {
                    left.clone()
                };
                parents.push((parent, value));
            }
            candidates.push(parents);
        }

        let recomputed_nodes = candidates.iter().map(Vec::len).sum();
        let touched_leaves = changed.len();
        for (index, chunk) in changed {
            self.chunks[index] = chunk;
        }
        for (level, values) in candidates.into_iter().enumerate() {
            for (index, value) in values {
                self.levels[level][index] = value;
            }
        }
        self.revision = revision;
        Ok(SummaryEditReport {
            path: SummaryEditPath::LocalTree,
            touched_leaves,
            recomputed_nodes,
            revision,
        })
    }

    fn replace_with_rebuild(
        &mut self,
        range: Range<usize>,
        replacement: &[u8],
    ) -> Result<SummaryEditReport, SummaryTreeError> {
        if range.is_empty() && replacement.is_empty() {
            return Ok(self.no_change_report());
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(SummaryTreeError::RevisionOverflow)?;
        let removed = range.len();
        let new_len = self
            .byte_len
            .checked_sub(removed)
            .and_then(|length| length.checked_add(replacement.len()))
            .ok_or(SummaryTreeError::LimitExceeded("file bytes"))?;
        validate_admission(self.profile, new_len, self.limits)?;
        let current = self.to_file_bytes()?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(new_len)
            .map_err(|_| SummaryTreeError::AllocationFailed)?;
        bytes.extend_from_slice(&current[..range.start]);
        bytes.extend_from_slice(replacement);
        bytes.extend_from_slice(&current[range.end..]);
        let candidate = Self::build(
            self.profile,
            self.encoder.clone(),
            self.base,
            &bytes,
            self.limits,
            revision,
        )?;
        let touched_leaves = candidate.chunks.len();
        let recomputed_nodes = candidate.levels.iter().map(Vec::len).sum();
        *self = candidate;
        Ok(SummaryEditReport {
            path: SummaryEditPath::BoundaryRebuild,
            touched_leaves,
            recomputed_nodes,
            revision,
        })
    }

    fn no_change_report(&self) -> SummaryEditReport {
        SummaryEditReport {
            path: SummaryEditPath::NoChange,
            touched_leaves: 0,
            recomputed_nodes: 0,
            revision: self.revision,
        }
    }

    /// Serializes revision, compact root and exact file bytes as `MFST` v1.
    pub fn to_checkpoint_bytes(&self) -> Result<Vec<u8>, SummaryTreeError> {
        let root = self.root_signature().to_canonical_bytes();
        let total = CHECKPOINT_HEADER_BYTES
            .checked_add(root.len())
            .and_then(|size| size.checked_add(self.byte_len))
            .ok_or(SummaryTreeError::LimitExceeded("checkpoint bytes"))?;
        if total > self.limits.max_checkpoint_bytes {
            return Err(SummaryTreeError::LimitExceeded("checkpoint bytes"));
        }
        let file = self.to_file_bytes()?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(total)
            .map_err(|_| SummaryTreeError::AllocationFailed)?;
        bytes.extend_from_slice(CHECKPOINT_MAGIC);
        bytes.extend_from_slice(&CHECKPOINT_SCHEMA.to_le_bytes());
        bytes.extend_from_slice(&[0, 0]);
        bytes.extend_from_slice(self.profile.profile_id.as_bytes());
        bytes.extend_from_slice(&self.revision.to_le_bytes());
        bytes.extend_from_slice(&(self.byte_len as u64).to_le_bytes());
        bytes.extend_from_slice(&(root.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&root);
        bytes.extend_from_slice(&file);
        Ok(bytes)
    }

    /// Restores an `MFST` checkpoint and rebuilds every internal node.
    pub fn from_checkpoint_bytes(
        profile: FileChunkProfile,
        encoder: E,
        base: F,
        bytes: &[u8],
        limits: SummaryTreeLimits,
    ) -> Result<Self, SummaryTreeError> {
        if bytes.len() > limits.max_checkpoint_bytes {
            return Err(SummaryTreeError::LimitExceeded("checkpoint bytes"));
        }
        if bytes.len() < CHECKPOINT_HEADER_BYTES
            || &bytes[..4] != CHECKPOINT_MAGIC
            || u16::from_le_bytes([bytes[4], bytes[5]]) != CHECKPOINT_SCHEMA
            || bytes[6..8] != [0, 0]
        {
            return Err(SummaryTreeError::InvalidCheckpoint("header"));
        }
        if &bytes[8..40] != profile.profile_id.as_bytes() {
            return Err(SummaryTreeError::InvalidCheckpoint("chunk profile"));
        }
        let revision = u64::from_le_bytes(bytes[40..48].try_into().expect("revision range"));
        let file_len = read_usize(&bytes[48..56], "file length")?;
        let root_len = read_usize(&bytes[56..64], "root length")?;
        let root_end = CHECKPOINT_HEADER_BYTES
            .checked_add(root_len)
            .filter(|end| *end <= bytes.len())
            .ok_or(SummaryTreeError::InvalidCheckpoint("root boundary"))?;
        let file_end = root_end
            .checked_add(file_len)
            .filter(|end| *end == bytes.len())
            .ok_or(SummaryTreeError::InvalidCheckpoint("file boundary"))?;
        let expected = SequenceSignature::from_canonical_bytes(
            encoder.clone(),
            base,
            &bytes[CHECKPOINT_HEADER_BYTES..root_end],
        )?;
        let candidate = Self::build(
            profile,
            encoder,
            base,
            &bytes[root_end..file_end],
            limits,
            revision,
        )?;
        if candidate.root_signature() != &expected {
            return Err(SummaryTreeError::InvalidCheckpoint("root mismatch"));
        }
        Ok(candidate)
    }
}

fn validate_admission(
    profile: FileChunkProfile,
    file_len: usize,
    limits: SummaryTreeLimits,
) -> Result<(), SummaryTreeError> {
    if profile.chunk_bytes == 0 || profile.chunk_bytes > limits.max_chunk_bytes {
        return Err(SummaryTreeError::InvalidChunkProfile);
    }
    if file_len > limits.max_file_bytes {
        return Err(SummaryTreeError::LimitExceeded("file bytes"));
    }
    Ok(())
}

fn validate_range(range: &Range<usize>, file_len: usize) -> Result<(), SummaryTreeError> {
    if range.start > range.end || range.end > file_len {
        Err(SummaryTreeError::InvalidRange)
    } else {
        Ok(())
    }
}

fn chunk_count(file_len: usize, chunk_bytes: usize) -> Result<usize, SummaryTreeError> {
    if file_len == 0 {
        return Ok(0);
    }
    file_len
        .checked_add(chunk_bytes - 1)
        .map(|length| length / chunk_bytes)
        .ok_or(SummaryTreeError::LimitExceeded("chunks"))
}

fn frame_chunk(chunk: &[u8]) -> Result<Vec<u8>, SummaryTreeError> {
    let capacity = 14_usize
        .checked_add(chunk.len())
        .ok_or(SummaryTreeError::LimitExceeded("chunk frame"))?;
    let mut framed = Vec::new();
    framed
        .try_reserve_exact(capacity)
        .map_err(|_| SummaryTreeError::AllocationFailed)?;
    framed.extend_from_slice(CHUNK_FRAME_MAGIC);
    framed.extend_from_slice(&1_u16.to_le_bytes());
    framed.extend_from_slice(&(chunk.len() as u64).to_le_bytes());
    framed.extend_from_slice(chunk);
    Ok(framed)
}

fn leaf_signature<F, E>(
    encoder: E,
    base: F,
    chunk: &[u8],
) -> Result<SequenceSignature<F, E>, SummaryTreeError>
where
    F: Field + CanonicalEncoding + StaticField + Pow + Invert,
    E: StructuralEncoder<F>,
{
    let mut leaf = SequenceSignature::new(encoder, base)?;
    leaf.push(&frame_chunk(chunk)?)?;
    Ok(leaf)
}

fn build_levels<F, E>(
    chunks: &[Vec<u8>],
    encoder: E,
    base: F,
) -> Result<Vec<Vec<SequenceSignature<F, E>>>, SummaryTreeError>
where
    F: Field + CanonicalEncoding + StaticField + Pow + Invert,
    E: StructuralEncoder<F>,
{
    if chunks.is_empty() {
        return Ok(vec![vec![SequenceSignature::new(encoder, base)?]]);
    }
    let mut leaves = Vec::new();
    leaves
        .try_reserve_exact(chunks.len())
        .map_err(|_| SummaryTreeError::AllocationFailed)?;
    for chunk in chunks {
        leaves.push(leaf_signature(encoder.clone(), base, chunk)?);
    }
    let mut levels = vec![leaves];
    while levels.last().expect("leaf level").len() > 1 {
        let previous = levels.last().expect("previous level");
        let mut next = Vec::new();
        next.try_reserve_exact(previous.len().div_ceil(2))
            .map_err(|_| SummaryTreeError::AllocationFailed)?;
        for pair in previous.chunks(2) {
            next.push(if pair.len() == 2 {
                pair[0].concatenate(&pair[1])?
            } else {
                pair[0].clone()
            });
        }
        levels.push(next);
    }
    Ok(levels)
}

fn candidate_or_existing<'a, F, E>(
    candidates: &'a [(usize, SequenceSignature<F, E>)],
    existing: &'a [SequenceSignature<F, E>],
    index: usize,
) -> &'a SequenceSignature<F, E>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    candidates
        .binary_search_by_key(&index, |(candidate, _)| *candidate)
        .map(|position| &candidates[position].1)
        .unwrap_or(&existing[index])
}

fn read_usize(bytes: &[u8], label: &'static str) -> Result<usize, SummaryTreeError> {
    let value = u64::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| SummaryTreeError::InvalidCheckpoint(label))?,
    );
    usize::try_from(value).map_err(|_| SummaryTreeError::LimitExceeded(label))
}
