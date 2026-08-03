//! Profile-independent, injective encoding of one ordered incidence graph.

use core::fmt;

use sha2::{Digest as _, Sha256};

use super::super::{
    GraphError, GraphSchemaId, IncidenceGraph, IncidenceGraphBuilder, RelationId, VertexId,
    VertexKind,
};

const MAGIC: &[u8; 4] = b"MFC2";
const MODEL_VERSION: u16 = 1;
const MIN_VERTEX_BYTES: usize = 1 + 8;
const MIN_ARC_BYTES: usize = 8 + 8 + 8 + 8 + 8;

/// Version of the exact canonical graph envelope.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct CanonicalGraphEncodingId(u16);

impl CanonicalGraphEncodingId {
    /// First profile-independent encoding of the normalized incidence model.
    pub const V1: Self = Self(1);

    /// Stable integer written into the wire envelope.
    #[must_use]
    pub const fn as_u16(self) -> u16 {
        self.0
    }

    fn parse(version: u16) -> Result<Self, GraphError> {
        if version == Self::V1.0 {
            Ok(Self::V1)
        } else {
            Err(GraphError::UnsupportedCanonicalEncoding { version })
        }
    }
}

/// SHA-256 index key of complete exact canonical bytes.
///
/// This is a convenience key. Authoritative equality compares the complete
/// bytes exposed by [`CanonicalGraphForm::bytes`].
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct CanonicalGraphKey([u8; 32]);

impl CanonicalGraphKey {
    /// Borrows the digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for CanonicalGraphKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for CanonicalGraphKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "CanonicalGraphKey({self})")
    }
}

/// Verified output of a complete exact canonization search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalGraphForm {
    encoding_id: CanonicalGraphEncodingId,
    schema_id: GraphSchemaId,
    key: CanonicalGraphKey,
    bytes: Vec<u8>,
    original_to_canonical: Vec<VertexId>,
    canonical_to_original: Vec<VertexId>,
}

impl CanonicalGraphForm {
    /// Exact versioned graph bytes, independent of analysis accelerators.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Stable schema whose semantics are represented by the bytes.
    #[must_use]
    pub const fn schema_id(&self) -> GraphSchemaId {
        self.schema_id
    }

    /// Version of the exact wire envelope.
    #[must_use]
    pub const fn encoding_id(&self) -> CanonicalGraphEncodingId {
        self.encoding_id
    }

    /// Convenience digest of the complete canonical bytes.
    #[must_use]
    pub const fn key(&self) -> CanonicalGraphKey {
        self.key
    }

    /// Maps each original index to its canonical position.
    #[must_use]
    pub fn original_to_canonical(&self) -> &[VertexId] {
        &self.original_to_canonical
    }

    /// Maps each canonical position back to the supplied graph.
    #[must_use]
    pub fn canonical_to_original(&self) -> &[VertexId] {
        &self.canonical_to_original
    }

    /// Strictly parses and reconstructs the normalized graph document.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::InvalidCanonicalEncoding`] if the retained bytes
    /// are not a normalized, injective v1 document.
    pub fn decode(&self) -> Result<CanonicalGraphDocument, GraphError> {
        CanonicalGraphDocument::from_bytes(&self.bytes)
    }
}

/// Strictly parsed ordered graph document.
///
/// Parsing proves framing and normalization, not that arbitrary external bytes
/// are the lexicographic minimum of an isomorphism class. Only
/// [`CanonicalGraphForm`] is published by the exact search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalGraphDocument {
    encoding_id: CanonicalGraphEncodingId,
    schema_id: GraphSchemaId,
    graph: IncidenceGraph,
}

impl CanonicalGraphDocument {
    /// Parses a complete graph envelope and rejects truncation/trailing bytes.
    ///
    /// # Errors
    ///
    /// Rejects unsupported versions, invalid endpoints, duplicate records,
    /// non-canonical record order and inconsistent counters.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, GraphError> {
        parse_document(bytes)
    }

    /// Encoding version found in the envelope.
    #[must_use]
    pub const fn encoding_id(&self) -> CanonicalGraphEncodingId {
        self.encoding_id
    }

    /// Application schema found in the envelope.
    #[must_use]
    pub const fn schema_id(&self) -> GraphSchemaId {
        self.schema_id
    }

    /// Reconstructed normalized graph in the encoded vertex order.
    #[must_use]
    pub const fn graph(&self) -> &IncidenceGraph {
        &self.graph
    }

    /// Consumes the document and returns its normalized graph.
    #[must_use]
    pub fn into_graph(self) -> IncidenceGraph {
        self.graph
    }
}

pub(crate) fn canonical_form_from_order(
    graph: &IncidenceGraph,
    canonical_to_original: Vec<VertexId>,
    schema_id: GraphSchemaId,
) -> Result<CanonicalGraphForm, GraphError> {
    if canonical_to_original.len() != graph.vertex_count() {
        return Err(GraphError::InvalidCanonicalOrder);
    }
    let mut original_to_canonical = vec![VertexId::new(0); graph.vertex_count()];
    let mut seen = vec![false; graph.vertex_count()];
    for (canonical, original) in canonical_to_original.iter().copied().enumerate() {
        if original.index() >= graph.vertex_count() || seen[original.index()] {
            return Err(GraphError::InvalidCanonicalOrder);
        }
        seen[original.index()] = true;
        original_to_canonical[original.index()] = VertexId::new(canonical);
    }

    let mut bytes = Vec::new();
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&CanonicalGraphEncodingId::V1.as_u16().to_be_bytes());
    bytes.extend_from_slice(&MODEL_VERSION.to_be_bytes());
    bytes.extend_from_slice(schema_id.as_bytes());
    append_usize(&mut bytes, graph.vertex_count())?;
    append_usize(&mut bytes, graph.incidence_count())?;
    bytes.extend_from_slice(&graph.total_multiplicity().to_be_bytes());

    for original in &canonical_to_original {
        bytes.push(graph.vertex_kind(*original) as u8);
        append_framed(&mut bytes, graph.vertex_label(*original))?;
    }

    let mut arcs = Vec::with_capacity(graph.incidence_count());
    for source in 0..graph.vertex_count() {
        for incidence in graph.outgoing(VertexId::new(source)) {
            arcs.push(OrderedArc {
                source: original_to_canonical[source].index(),
                target: original_to_canonical[incidence.neighbor().index()].index(),
                relation: incidence.relation(),
                multiplicity: incidence.multiplicity(),
            });
        }
    }
    arcs.sort_unstable_by(|left, right| compare_arcs(graph, left, right));
    for arc in arcs {
        append_usize(&mut bytes, arc.source)?;
        append_usize(&mut bytes, arc.target)?;
        let descriptor = graph.relation(arc.relation);
        append_framed(&mut bytes, descriptor.relation())?;
        append_framed(&mut bytes, descriptor.role())?;
        bytes.extend_from_slice(&arc.multiplicity.to_be_bytes());
    }

    let mut hasher = Sha256::new();
    hasher.update(b"microfield/canonical-graph-key/v1\0");
    hasher.update(&bytes);
    let key = CanonicalGraphKey(hasher.finalize().into());
    Ok(CanonicalGraphForm {
        encoding_id: CanonicalGraphEncodingId::V1,
        schema_id,
        key,
        bytes,
        original_to_canonical,
        canonical_to_original,
    })
}

#[derive(Clone, Copy, Debug)]
struct OrderedArc {
    source: usize,
    target: usize,
    relation: RelationId,
    multiplicity: u64,
}

fn compare_arcs(
    graph: &IncidenceGraph,
    left: &OrderedArc,
    right: &OrderedArc,
) -> core::cmp::Ordering {
    let left_descriptor = graph.relation(left.relation);
    let right_descriptor = graph.relation(right.relation);
    (left.source, left.target)
        .cmp(&(right.source, right.target))
        .then_with(|| left_descriptor.cmp(right_descriptor))
        .then_with(|| left.multiplicity.cmp(&right.multiplicity))
}

fn parse_document(bytes: &[u8]) -> Result<CanonicalGraphDocument, GraphError> {
    let mut cursor = Cursor::new(bytes);
    if cursor.take(MAGIC.len())? != MAGIC {
        return Err(GraphError::InvalidCanonicalEncoding);
    }
    let encoding_id = CanonicalGraphEncodingId::parse(cursor.read_u16()?)?;
    if cursor.read_u16()? != MODEL_VERSION {
        return Err(GraphError::InvalidCanonicalEncoding);
    }
    let mut schema = [0_u8; 32];
    schema.copy_from_slice(cursor.take(32)?);
    let schema_id = GraphSchemaId::from_bytes(schema);
    let vertex_count = cursor.read_usize()?;
    let incidence_count = cursor.read_usize()?;
    let expected_multiplicity = cursor.read_u64()?;
    if vertex_count > cursor.remaining() / MIN_VERTEX_BYTES {
        return Err(GraphError::InvalidCanonicalEncoding);
    }

    let mut builder = IncidenceGraphBuilder::new();
    for _ in 0..vertex_count {
        let kind = match cursor.read_u8()? {
            1 => VertexKind::Entity,
            2 => VertexKind::Hyperedge,
            _ => return Err(GraphError::InvalidCanonicalEncoding),
        };
        let label = cursor.read_framed()?.to_vec();
        builder.add_typed_vertex(kind, label);
    }
    if incidence_count > cursor.remaining() / MIN_ARC_BYTES {
        return Err(GraphError::InvalidCanonicalEncoding);
    }

    let mut previous: Option<(usize, usize, Vec<u8>, Vec<u8>)> = None;
    let mut total_multiplicity = 0_u64;
    for _ in 0..incidence_count {
        let source = cursor.read_usize()?;
        let target = cursor.read_usize()?;
        if source >= vertex_count || target >= vertex_count {
            return Err(GraphError::InvalidCanonicalEncoding);
        }
        let relation = cursor.read_framed()?.to_vec();
        let role = cursor.read_framed()?.to_vec();
        let multiplicity = cursor.read_u64()?;
        if multiplicity == 0 {
            return Err(GraphError::InvalidCanonicalEncoding);
        }
        total_multiplicity = total_multiplicity
            .checked_add(multiplicity)
            .ok_or(GraphError::InvalidCanonicalEncoding)?;
        let key = (source, target, relation.clone(), role.clone());
        if previous.as_ref().is_some_and(|prior| prior >= &key) {
            return Err(GraphError::InvalidCanonicalEncoding);
        }
        previous = Some(key);
        builder.add_directed_relation(
            VertexId::new(source),
            VertexId::new(target),
            relation,
            role,
            multiplicity,
        )?;
    }
    if cursor.remaining() != 0 || total_multiplicity != expected_multiplicity {
        return Err(GraphError::InvalidCanonicalEncoding);
    }
    let graph = builder.build()?;
    if graph.vertex_count() != vertex_count
        || graph.incidence_count() != incidence_count
        || graph.total_multiplicity() != expected_multiplicity
    {
        return Err(GraphError::InvalidCanonicalEncoding);
    }

    let identity = (0..vertex_count).map(VertexId::new).collect();
    let reencoded = canonical_form_from_order(&graph, identity, schema_id)?;
    if reencoded.bytes != bytes {
        return Err(GraphError::InvalidCanonicalEncoding);
    }
    Ok(CanonicalGraphDocument {
        encoding_id,
        schema_id,
        graph,
    })
}

fn append_usize(output: &mut Vec<u8>, value: usize) -> Result<(), GraphError> {
    let value = u64::try_from(value).map_err(|_| GraphError::GraphTooLarge)?;
    output.extend_from_slice(&value.to_be_bytes());
    Ok(())
}

fn append_framed(output: &mut Vec<u8>, bytes: &[u8]) -> Result<(), GraphError> {
    append_usize(output, bytes.len())?;
    output.extend_from_slice(bytes);
    Ok(())
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    const fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], GraphError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(GraphError::InvalidCanonicalEncoding)?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or(GraphError::InvalidCanonicalEncoding)?;
        self.offset = end;
        Ok(bytes)
    }

    fn read_u8(&mut self) -> Result<u8, GraphError> {
        Ok(self.take(1)?[0])
    }

    fn read_u16(&mut self) -> Result<u16, GraphError> {
        let mut bytes = [0_u8; 2];
        bytes.copy_from_slice(self.take(2)?);
        Ok(u16::from_be_bytes(bytes))
    }

    fn read_u64(&mut self) -> Result<u64, GraphError> {
        let mut bytes = [0_u8; 8];
        bytes.copy_from_slice(self.take(8)?);
        Ok(u64::from_be_bytes(bytes))
    }

    fn read_usize(&mut self) -> Result<usize, GraphError> {
        usize::try_from(self.read_u64()?).map_err(|_| GraphError::InvalidCanonicalEncoding)
    }

    fn read_framed(&mut self) -> Result<&'a [u8], GraphError> {
        let length = self.read_usize()?;
        self.take(length)
    }
}
