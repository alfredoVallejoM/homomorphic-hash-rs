//! Exact degree histograms with a compact multiset correlation fingerprint.

use core::fmt;
use std::collections::BTreeMap;

use microfield::{CanonicalEncoding, Field, StaticField};
use sha2::{Digest as _, Sha256};

use crate::structural::{MultiEvaluationMultisetSignature, SignatureAssurance, StructuralEncoder};

use super::super::{GraphError, IncidenceGraph, VertexId};

const DEGREE_MAGIC: &[u8; 4] = b"MFDH";
const DEGREE_SCHEMA: u16 = 1;

/// Stable identity of one field/encoder/evaluation degree profile.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct DegreeHistogramProfileId([u8; 32]);

impl DegreeHistogramProfileId {
    /// Borrows the domain-separated identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for DegreeHistogramProfileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for DegreeHistogramProfileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "DegreeHistogramProfileId({self})")
    }
}

/// One exact `degree -> vertex count` entry.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DegreeHistogramBin {
    degree: u64,
    vertex_count: u64,
}

impl DegreeHistogramBin {
    /// Degree represented by this bin.
    #[must_use]
    pub const fn degree(self) -> u64 {
        self.degree
    }

    /// Exact number of vertices with this degree.
    #[must_use]
    pub const fn vertex_count(self) -> u64 {
        self.vertex_count
    }
}

/// Sparse exact degree histogram in ascending degree order.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DegreeHistogram {
    bins: Vec<DegreeHistogramBin>,
}

impl DegreeHistogram {
    /// Sorted non-empty bins.
    #[must_use]
    pub fn bins(&self) -> &[DegreeHistogramBin] {
        &self.bins
    }

    /// Exact count for `degree`, or zero when the bin is absent.
    #[must_use]
    pub fn vertex_count_at(&self, degree: u64) -> u64 {
        self.bins
            .binary_search_by_key(&degree, |bin| bin.degree)
            .map_or(0, |index| self.bins[index].vertex_count)
    }

    fn from_counts(counts: BTreeMap<u64, u64>) -> Self {
        Self {
            bins: counts
                .into_iter()
                .map(|(degree, vertex_count)| DegreeHistogramBin {
                    degree,
                    vertex_count,
                })
                .collect(),
        }
    }

    fn combine(&self, other: &Self) -> Result<Self, GraphError> {
        let mut counts = self
            .bins
            .iter()
            .map(|bin| (bin.degree, bin.vertex_count))
            .collect::<BTreeMap<_, _>>();
        for bin in &other.bins {
            let count = counts.entry(bin.degree).or_default();
            *count = count
                .checked_add(bin.vertex_count)
                .ok_or(GraphError::GraphTooLarge)?;
        }
        Ok(Self::from_counts(counts))
    }

    fn append_wire(&self, bytes: &mut Vec<u8>) {
        bytes.extend_from_slice(
            &u64::try_from(self.bins.len())
                .unwrap_or(u64::MAX)
                .to_be_bytes(),
        );
        for bin in &self.bins {
            bytes.extend_from_slice(&bin.degree.to_be_bytes());
            bytes.extend_from_slice(&bin.vertex_count.to_be_bytes());
        }
    }
}

/// Exact degree histograms plus a compact multiset fingerprint of joint degrees.
///
/// For simple undirected loopless graphs, `support`, `outgoing_records` and
/// `incoming_records` are the ordinary degree histogram. For the full
/// relational model they intentionally remain distinct:
///
/// - support counts distinct non-self weak neighbors;
/// - record histograms count normalized CSR records, with a self-loop once;
/// - multiplicity histograms sum each record's exact multiplicity.
///
/// The five histograms are exact necessary invariants. The joint field product
/// additionally correlates those values, vertex kind and loop statistics, but
/// remains a collision-prone fingerprint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DegreeHistogramProfile<F, E, const K: usize>
where
    F: Field,
    E: StructuralEncoder<F>,
{
    id: DegreeHistogramProfileId,
    graph_count: u64,
    vertex_count: u64,
    support: DegreeHistogram,
    outgoing_records: DegreeHistogram,
    incoming_records: DegreeHistogram,
    outgoing_multiplicity: DegreeHistogram,
    incoming_multiplicity: DegreeHistogram,
    joint_fingerprint: MultiEvaluationMultisetSignature<F, E, K>,
}

impl<F, E, const K: usize> DegreeHistogramProfile<F, E, K>
where
    F: Field + CanonicalEncoding + StaticField,
    E: StructuralEncoder<F>,
{
    /// Computes all histograms and the joint multiset fingerprint in one pass.
    ///
    /// # Errors
    ///
    /// Rejects invalid/repeated evaluation offsets, graph-size overflow,
    /// multiplicity overflow or encoder failure without publishing a profile.
    pub fn analyze(
        graph: &IncidenceGraph,
        encoder: E,
        offsets: [F; K],
    ) -> Result<Self, GraphError> {
        let mut joint_fingerprint = MultiEvaluationMultisetSignature::new(encoder, offsets)?;
        let id = derive_id(joint_fingerprint.context().signature_id());
        let support_degrees = support_degrees(graph)?;
        let mut support = BTreeMap::new();
        let mut outgoing_records = BTreeMap::new();
        let mut incoming_records = BTreeMap::new();
        let mut outgoing_multiplicity = BTreeMap::new();
        let mut incoming_multiplicity = BTreeMap::new();

        for (index, &support_degree) in support_degrees.iter().enumerate() {
            let vertex = VertexId::new(index);
            let outgoing = graph.outgoing(vertex);
            let incoming = graph.incoming(vertex);
            let outgoing_count =
                u64::try_from(outgoing.len()).map_err(|_| GraphError::GraphTooLarge)?;
            let incoming_count =
                u64::try_from(incoming.len()).map_err(|_| GraphError::GraphTooLarge)?;
            let outgoing_weight = sum_multiplicity(outgoing)?;
            let incoming_weight = sum_multiplicity(incoming)?;
            increment(&mut support, support_degree)?;
            increment(&mut outgoing_records, outgoing_count)?;
            increment(&mut incoming_records, incoming_count)?;
            increment(&mut outgoing_multiplicity, outgoing_weight)?;
            increment(&mut incoming_multiplicity, incoming_weight)?;

            let mut loop_records = 0_u64;
            let mut loop_multiplicity = 0_u64;
            for incidence in outgoing {
                if incidence.neighbor() == vertex {
                    loop_records = loop_records
                        .checked_add(1)
                        .ok_or(GraphError::GraphTooLarge)?;
                    loop_multiplicity = loop_multiplicity
                        .checked_add(incidence.multiplicity())
                        .ok_or(GraphError::MultiplicityOverflow)?;
                }
            }
            let token = degree_token(
                graph.vertex_kind(vertex) as u8,
                support_degree,
                outgoing_count,
                incoming_count,
                outgoing_weight,
                incoming_weight,
                loop_records,
                loop_multiplicity,
            );
            joint_fingerprint.insert(&token)?;
        }

        Ok(Self {
            id,
            graph_count: 1,
            vertex_count: u64::try_from(graph.vertex_count())
                .map_err(|_| GraphError::GraphTooLarge)?,
            support: DegreeHistogram::from_counts(support),
            outgoing_records: DegreeHistogram::from_counts(outgoing_records),
            incoming_records: DegreeHistogram::from_counts(incoming_records),
            outgoing_multiplicity: DegreeHistogram::from_counts(outgoing_multiplicity),
            incoming_multiplicity: DegreeHistogram::from_counts(incoming_multiplicity),
            joint_fingerprint,
        })
    }

    /// Combines profiles exactly for a disjoint union.
    ///
    /// # Errors
    ///
    /// Rejects different field/encoder/offset identities and counter overflow.
    pub fn combine_disjoint(&self, other: &Self) -> Result<Self, GraphError> {
        if self.id != other.id {
            return Err(GraphError::DegreeHistogramProfileMismatch);
        }
        Ok(Self {
            id: self.id,
            graph_count: self
                .graph_count
                .checked_add(other.graph_count)
                .ok_or(GraphError::GraphTooLarge)?,
            vertex_count: self
                .vertex_count
                .checked_add(other.vertex_count)
                .ok_or(GraphError::GraphTooLarge)?,
            support: self.support.combine(&other.support)?,
            outgoing_records: self.outgoing_records.combine(&other.outgoing_records)?,
            incoming_records: self.incoming_records.combine(&other.incoming_records)?,
            outgoing_multiplicity: self
                .outgoing_multiplicity
                .combine(&other.outgoing_multiplicity)?,
            incoming_multiplicity: self
                .incoming_multiplicity
                .combine(&other.incoming_multiplicity)?,
            joint_fingerprint: self.joint_fingerprint.combine(&other.joint_fingerprint)?,
        })
    }

    /// Complete field, encoder and evaluation-point identity.
    #[must_use]
    pub const fn id(&self) -> DegreeHistogramProfileId {
        self.id
    }

    /// Number of disjoint graphs represented.
    #[must_use]
    pub const fn graph_count(&self) -> u64 {
        self.graph_count
    }

    /// Exact number of represented vertices.
    #[must_use]
    pub const fn vertex_count(&self) -> u64 {
        self.vertex_count
    }

    /// Ordinary degree histogram for simple undirected loopless graphs.
    #[must_use]
    pub const fn support(&self) -> &DegreeHistogram {
        &self.support
    }

    /// Histogram of normalized outgoing CSR records.
    #[must_use]
    pub const fn outgoing_records(&self) -> &DegreeHistogram {
        &self.outgoing_records
    }

    /// Histogram of normalized incoming CSR records.
    #[must_use]
    pub const fn incoming_records(&self) -> &DegreeHistogram {
        &self.incoming_records
    }

    /// Histogram of summed outgoing multiplicities.
    #[must_use]
    pub const fn outgoing_multiplicity(&self) -> &DegreeHistogram {
        &self.outgoing_multiplicity
    }

    /// Histogram of summed incoming multiplicities.
    #[must_use]
    pub const fn incoming_multiplicity(&self) -> &DegreeHistogram {
        &self.incoming_multiplicity
    }

    /// Compact multievaluation product over correlated per-vertex degrees.
    #[must_use]
    pub const fn joint_fingerprint(&self) -> &MultiEvaluationMultisetSignature<F, E, K> {
        &self.joint_fingerprint
    }

    /// Overall equality still contains one finite-field correlation channel.
    #[must_use]
    pub const fn assurance(&self) -> SignatureAssurance {
        SignatureAssurance::Fingerprint
    }

    /// Stable field-specific persistence envelope.
    #[must_use]
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let fingerprint = self.joint_fingerprint.to_canonical_bytes();
        let mut bytes = Vec::new();
        bytes.extend_from_slice(DEGREE_MAGIC);
        bytes.extend_from_slice(&DEGREE_SCHEMA.to_be_bytes());
        bytes.extend_from_slice(self.id.as_bytes());
        bytes.extend_from_slice(&self.graph_count.to_be_bytes());
        bytes.extend_from_slice(&self.vertex_count.to_be_bytes());
        self.support.append_wire(&mut bytes);
        self.outgoing_records.append_wire(&mut bytes);
        self.incoming_records.append_wire(&mut bytes);
        self.outgoing_multiplicity.append_wire(&mut bytes);
        self.incoming_multiplicity.append_wire(&mut bytes);
        bytes.extend_from_slice(
            &u64::try_from(fingerprint.len())
                .unwrap_or(u64::MAX)
                .to_be_bytes(),
        );
        bytes.extend_from_slice(&fingerprint);
        bytes
    }
}

fn derive_id(signature_id: crate::structural::SignatureId) -> DegreeHistogramProfileId {
    let mut hasher = Sha256::new();
    hasher.update(b"microfield/degree-histogram-profile/v1\0");
    hasher.update(signature_id.as_bytes());
    DegreeHistogramProfileId(hasher.finalize().into())
}

pub(crate) fn exact_degree_histograms_equal(
    left: &IncidenceGraph,
    right: &IncidenceGraph,
) -> Result<bool, GraphError> {
    Ok(exact_degree_histograms(left)? == exact_degree_histograms(right)?)
}

fn exact_degree_histograms(graph: &IncidenceGraph) -> Result<[BTreeMap<u64, u64>; 5], GraphError> {
    let support_degrees = support_degrees(graph)?;
    let mut summaries: [BTreeMap<u64, u64>; 5] = core::array::from_fn(|_| BTreeMap::new());
    for (index, support) in support_degrees.into_iter().enumerate() {
        let vertex = VertexId::new(index);
        let outgoing = graph.outgoing(vertex);
        let incoming = graph.incoming(vertex);
        increment(&mut summaries[0], support)?;
        increment(
            &mut summaries[1],
            u64::try_from(outgoing.len()).map_err(|_| GraphError::GraphTooLarge)?,
        )?;
        increment(
            &mut summaries[2],
            u64::try_from(incoming.len()).map_err(|_| GraphError::GraphTooLarge)?,
        )?;
        increment(&mut summaries[3], sum_multiplicity(outgoing)?)?;
        increment(&mut summaries[4], sum_multiplicity(incoming)?)?;
    }
    Ok(summaries)
}

fn support_degrees(graph: &IncidenceGraph) -> Result<Vec<u64>, GraphError> {
    let mut neighborhoods = vec![Vec::new(); graph.vertex_count()];
    for source in 0..graph.vertex_count() {
        for incidence in graph.outgoing(VertexId::new(source)) {
            let target = incidence.neighbor().index();
            if source != target {
                neighborhoods[source].push(target);
                neighborhoods[target].push(source);
            }
        }
    }
    neighborhoods
        .into_iter()
        .map(|mut neighbors| {
            neighbors.sort_unstable();
            neighbors.dedup();
            u64::try_from(neighbors.len()).map_err(|_| GraphError::GraphTooLarge)
        })
        .collect()
}

fn sum_multiplicity(incidences: &[super::super::Incidence]) -> Result<u64, GraphError> {
    incidences.iter().try_fold(0_u64, |total, incidence| {
        total
            .checked_add(incidence.multiplicity())
            .ok_or(GraphError::MultiplicityOverflow)
    })
}

fn increment(histogram: &mut BTreeMap<u64, u64>, degree: u64) -> Result<(), GraphError> {
    let count = histogram.entry(degree).or_default();
    *count = count.checked_add(1).ok_or(GraphError::GraphTooLarge)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn degree_token(
    kind: u8,
    support: u64,
    outgoing_records: u64,
    incoming_records: u64,
    outgoing_multiplicity: u64,
    incoming_multiplicity: u64,
    loop_records: u64,
    loop_multiplicity: u64,
) -> [u8; 58] {
    let mut token = [0_u8; 58];
    token[0] = 1;
    token[1] = kind;
    for (index, value) in [
        support,
        outgoing_records,
        incoming_records,
        outgoing_multiplicity,
        incoming_multiplicity,
        loop_records,
        loop_multiplicity,
    ]
    .into_iter()
    .enumerate()
    {
        let start = 2 + index * 8;
        token[start..start + 8].copy_from_slice(&value.to_be_bytes());
    }
    token
}
