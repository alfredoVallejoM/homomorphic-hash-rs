//! Budgeted exact 2-WL refinement localized to ambiguous cells and boundaries.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest as _, Sha256};

use super::super::{GraphError, Incidence, IncidenceGraph, VertexId};

const PAIR_MAGIC: &[u8; 4] = b"MF2W";
const PAIR_SCHEMA: u16 = 1;
const MAXIMUM_ROUNDS: u8 = 8;

/// Stable identity of the localized exact pair-refinement algorithm.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct LocalPairRefinementProfileId([u8; 32]);

impl LocalPairRefinementProfileId {
    /// Borrows the versioned identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for LocalPairRefinementProfileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for LocalPairRefinementProfileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "LocalPairRefinementProfileId({self})")
    }
}

/// Atomic execution state of the localized 2-WL tier.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum PairRefinementStatus {
    /// Every admitted pair and round was evaluated.
    Complete = 1,
    /// The invariant preflight exceeded the caller's work ceiling.
    SkippedBudget = 2,
}

/// Exact localized pair-color histogram and rooted descriptors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalPairRefinementProfile {
    id: LocalPairRefinementProfileId,
    status: PairRefinementStatus,
    rounds: u8,
    estimated_work: u64,
    ambiguous_vertex_count: u64,
    active_vertex_count: u64,
    color_histogram: Vec<(u64, u64)>,
    rooted_descriptors: Vec<Vec<u8>>,
}

impl LocalPairRefinementProfile {
    /// Runs exact pair refinement on ambiguous vertices plus their one-hop boundary.
    ///
    /// # Errors
    ///
    /// Rejects rounds outside `1..=8`, stable-size overflow and malformed graph
    /// metadata. A budget skip contains no partial colors or rooted values.
    pub fn analyze(
        graph: &IncidenceGraph,
        rounds: u8,
        maximum_work: u64,
    ) -> Result<Self, GraphError> {
        if !(1..=MAXIMUM_ROUNDS).contains(&rounds) {
            return Err(GraphError::InvalidPairRefinementProfile);
        }
        let id = derive_id(rounds);
        let base = vertex_descriptors(graph)?;
        let ambiguous = ambiguous_vertices(&base);
        let active = active_boundary(graph, &ambiguous);
        let active_count = u64::try_from(active.len()).map_err(|_| GraphError::GraphTooLarge)?;
        let estimated_work = active_count
            .checked_mul(active_count)
            .and_then(|value| value.checked_mul(active_count))
            .and_then(|value| value.checked_mul(u64::from(rounds)))
            .ok_or(GraphError::GraphTooLarge)?;
        if estimated_work > maximum_work {
            return Ok(Self {
                id,
                status: PairRefinementStatus::SkippedBudget,
                rounds,
                estimated_work,
                ambiguous_vertex_count: u64::try_from(ambiguous.len())
                    .map_err(|_| GraphError::GraphTooLarge)?,
                active_vertex_count: active_count,
                color_histogram: Vec::new(),
                rooted_descriptors: Vec::new(),
            });
        }
        if active.is_empty() {
            return Ok(Self {
                id,
                status: PairRefinementStatus::Complete,
                rounds,
                estimated_work,
                ambiguous_vertex_count: 0,
                active_vertex_count: 0,
                color_histogram: Vec::new(),
                rooted_descriptors: Vec::new(),
            });
        }

        let order = active.len();
        let mut keys = Vec::with_capacity(order * order);
        for &left in &active {
            for &right in &active {
                keys.push(initial_pair_key(graph, &base, left, right)?);
            }
        }
        let mut colors = intern(&keys);
        for _ in 0..rounds {
            keys.clear();
            for left in 0..order {
                for right in 0..order {
                    let mut key = Vec::new();
                    append_u64(&mut key, colors[left * order + right])?;
                    let mut transitions = (0..order)
                        .map(|middle| {
                            (
                                colors[left * order + middle],
                                colors[middle * order + right],
                            )
                        })
                        .collect::<Vec<_>>();
                    transitions.sort_unstable();
                    append_u64(&mut key, transitions.len())?;
                    for (first, second) in transitions {
                        append_u64(&mut key, first)?;
                        append_u64(&mut key, second)?;
                    }
                    keys.push(key);
                }
            }
            colors = intern(&keys);
        }

        let active_position = active
            .iter()
            .enumerate()
            .map(|(position, &vertex)| (vertex, position))
            .collect::<BTreeMap<_, _>>();
        let mut rooted_descriptors = Vec::with_capacity(ambiguous.len());
        for vertex in &ambiguous {
            let position = active_position[vertex];
            let mut pairs = (0..order)
                .map(|other| {
                    (
                        colors[position * order + other],
                        colors[other * order + position],
                    )
                })
                .collect::<Vec<_>>();
            pairs.sort_unstable();
            let mut descriptor = base[*vertex].clone();
            append_u64(&mut descriptor, pairs.len())?;
            for (outgoing, incoming) in pairs {
                append_u64(&mut descriptor, outgoing)?;
                append_u64(&mut descriptor, incoming)?;
            }
            rooted_descriptors.push(descriptor);
        }
        rooted_descriptors.sort_unstable();
        let mut histogram = BTreeMap::<usize, u64>::new();
        for color in colors {
            let count = histogram.entry(color).or_default();
            *count = count.checked_add(1).ok_or(GraphError::GraphTooLarge)?;
        }
        Ok(Self {
            id,
            status: PairRefinementStatus::Complete,
            rounds,
            estimated_work,
            ambiguous_vertex_count: u64::try_from(ambiguous.len())
                .map_err(|_| GraphError::GraphTooLarge)?,
            active_vertex_count: active_count,
            color_histogram: histogram
                .into_iter()
                .map(|(color, count)| {
                    Ok((
                        u64::try_from(color).map_err(|_| GraphError::GraphTooLarge)?,
                        count,
                    ))
                })
                .collect::<Result<_, GraphError>>()?,
            rooted_descriptors,
        })
    }

    /// Complete algorithm/round identity.
    #[must_use]
    pub const fn id(&self) -> LocalPairRefinementProfileId {
        self.id
    }
    /// Complete or atomically skipped status.
    #[must_use]
    pub const fn status(&self) -> PairRefinementStatus {
        self.status
    }
    /// Exact refinement rounds requested.
    #[must_use]
    pub const fn rounds(&self) -> u8 {
        self.rounds
    }
    /// Invariant preflight work estimate.
    #[must_use]
    pub const fn estimated_work(&self) -> u64 {
        self.estimated_work
    }
    /// Vertices in non-singleton exact local cells.
    #[must_use]
    pub const fn ambiguous_vertex_count(&self) -> u64 {
        self.ambiguous_vertex_count
    }
    /// Ambiguous vertices plus their exact one-hop boundary.
    #[must_use]
    pub const fn active_vertex_count(&self) -> u64 {
        self.active_vertex_count
    }
    /// Exact final pair-color multiplicities.
    #[must_use]
    pub fn color_histogram(&self) -> &[(u64, u64)] {
        &self.color_histogram
    }
    /// Sorted exact per-root pair descriptors for reinjection or comparison.
    #[must_use]
    pub fn rooted_descriptors(&self) -> &[Vec<u8>] {
        &self.rooted_descriptors
    }

    /// Stable field-independent envelope.
    #[must_use]
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(PAIR_MAGIC);
        bytes.extend_from_slice(&PAIR_SCHEMA.to_be_bytes());
        bytes.extend_from_slice(self.id.as_bytes());
        bytes.push(self.status as u8);
        bytes.push(self.rounds);
        bytes.extend_from_slice(&self.estimated_work.to_be_bytes());
        bytes.extend_from_slice(&self.ambiguous_vertex_count.to_be_bytes());
        bytes.extend_from_slice(&self.active_vertex_count.to_be_bytes());
        bytes.extend_from_slice(
            &u64::try_from(self.color_histogram.len())
                .unwrap_or(u64::MAX)
                .to_be_bytes(),
        );
        for (color, count) in &self.color_histogram {
            bytes.extend_from_slice(&color.to_be_bytes());
            bytes.extend_from_slice(&count.to_be_bytes());
        }
        bytes.extend_from_slice(
            &u64::try_from(self.rooted_descriptors.len())
                .unwrap_or(u64::MAX)
                .to_be_bytes(),
        );
        for descriptor in &self.rooted_descriptors {
            bytes.extend_from_slice(
                &u64::try_from(descriptor.len())
                    .unwrap_or(u64::MAX)
                    .to_be_bytes(),
            );
            bytes.extend_from_slice(descriptor);
        }
        bytes
    }
}

fn derive_id(rounds: u8) -> LocalPairRefinementProfileId {
    let mut hasher = Sha256::new();
    hasher.update(b"microfield/local-pair-refinement/v1\0");
    hasher.update([rounds]);
    LocalPairRefinementProfileId(hasher.finalize().into())
}

fn ambiguous_vertices(descriptors: &[Vec<u8>]) -> Vec<usize> {
    let mut groups = BTreeMap::<&[u8], Vec<usize>>::new();
    for (index, descriptor) in descriptors.iter().enumerate() {
        groups.entry(descriptor).or_default().push(index);
    }
    groups
        .into_values()
        .filter(|group| group.len() > 1)
        .flatten()
        .collect()
}

fn active_boundary(graph: &IncidenceGraph, ambiguous: &[usize]) -> Vec<usize> {
    let mut active = ambiguous.iter().copied().collect::<BTreeSet<_>>();
    for &vertex in ambiguous {
        let id = VertexId::new(vertex);
        for incidence in graph.outgoing(id).iter().chain(graph.incoming(id)) {
            active.insert(incidence.neighbor().index());
        }
    }
    active.into_iter().collect()
}

fn vertex_descriptors(graph: &IncidenceGraph) -> Result<Vec<Vec<u8>>, GraphError> {
    let mut descriptors = Vec::with_capacity(graph.vertex_count());
    for index in 0..graph.vertex_count() {
        let vertex = VertexId::new(index);
        let mut descriptor = vec![graph.vertex_kind(vertex) as u8];
        append_framed(&mut descriptor, graph.vertex_label(vertex))?;
        append_incidence_summary(&mut descriptor, graph, graph.outgoing(vertex))?;
        append_incidence_summary(&mut descriptor, graph, graph.incoming(vertex))?;
        descriptors.push(descriptor);
    }
    Ok(descriptors)
}

fn append_incidence_summary(
    output: &mut Vec<u8>,
    graph: &IncidenceGraph,
    incidences: &[Incidence],
) -> Result<(), GraphError> {
    let mut records = Vec::with_capacity(incidences.len());
    for incidence in incidences {
        let relation = graph.relation(incidence.relation());
        let neighbor = incidence.neighbor();
        let mut record = vec![graph.vertex_kind(neighbor) as u8];
        append_framed(&mut record, graph.vertex_label(neighbor))?;
        append_framed(&mut record, relation.relation())?;
        append_framed(&mut record, relation.role())?;
        record.extend_from_slice(&incidence.multiplicity().to_be_bytes());
        records.push(record);
    }
    records.sort_unstable();
    append_u64(output, records.len())?;
    for record in records {
        append_framed(output, &record)?;
    }
    Ok(())
}

fn initial_pair_key(
    graph: &IncidenceGraph,
    base: &[Vec<u8>],
    left: usize,
    right: usize,
) -> Result<Vec<u8>, GraphError> {
    let mut key = vec![u8::from(left == right)];
    append_framed(&mut key, &base[left])?;
    append_framed(&mut key, &base[right])?;
    append_framed(&mut key, &arc_bundle(graph, left, right)?)?;
    append_framed(&mut key, &arc_bundle(graph, right, left)?)?;
    Ok(key)
}

fn arc_bundle(graph: &IncidenceGraph, source: usize, target: usize) -> Result<Vec<u8>, GraphError> {
    let mut records = Vec::new();
    for incidence in graph.outgoing(VertexId::new(source)) {
        if incidence.neighbor().index() == target {
            let relation = graph.relation(incidence.relation());
            let mut record = Vec::new();
            append_framed(&mut record, relation.relation())?;
            append_framed(&mut record, relation.role())?;
            record.extend_from_slice(&incidence.multiplicity().to_be_bytes());
            records.push(record);
        }
    }
    records.sort_unstable();
    let mut bundle = Vec::new();
    append_u64(&mut bundle, records.len())?;
    for record in records {
        append_framed(&mut bundle, &record)?;
    }
    Ok(bundle)
}

fn intern(keys: &[Vec<u8>]) -> Vec<usize> {
    let mut unique = keys.to_vec();
    unique.sort_unstable();
    unique.dedup();
    keys.iter()
        .map(|key| unique.binary_search(key).expect("interned key"))
        .collect()
}

fn append_u64(output: &mut Vec<u8>, value: usize) -> Result<(), GraphError> {
    output.extend_from_slice(
        &u64::try_from(value)
            .map_err(|_| GraphError::GraphTooLarge)?
            .to_be_bytes(),
    );
    Ok(())
}

fn append_framed(output: &mut Vec<u8>, bytes: &[u8]) -> Result<(), GraphError> {
    append_u64(output, bytes.len())?;
    output.extend_from_slice(bytes);
    Ok(())
}
