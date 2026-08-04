//! Per-cell power moments with independent finite-field lane encoding.

use core::fmt;
use std::collections::BTreeMap;

use microfield::{CanonicalEncoding, Field, StaticField};
use sha2::{Digest as _, Sha256};

use crate::structural::{SignatureAssurance, StructuralLaneEncoder};

use super::super::{GraphError, IncidenceGraph, VertexId};

const MOMENT_MAGIC: &[u8; 4] = b"MFCM";
const MOMENT_SCHEMA: u16 = 1;

/// Stable identity of one round/cell moment family.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct CellMomentProfileId([u8; 32]);

impl CellMomentProfileId {
    /// Borrows the stable digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for CellMomentProfileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for CellMomentProfileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "CellMomentProfileId({self})")
    }
}

/// Power sums retained for one exact, caller-defined cell descriptor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellMomentCell<F, const K: usize, const D: usize>
where
    F: Field,
{
    descriptor: Vec<u8>,
    cardinality: u64,
    power_sums: [[F; K]; D],
}

impl<F, const K: usize, const D: usize> CellMomentCell<F, K, D>
where
    F: Field,
{
    /// Exact cell descriptor; profile ordering is lexicographic over these bytes.
    #[must_use]
    pub fn descriptor(&self) -> &[u8] {
        &self.descriptor
    }

    /// Number of values accumulated in this cell.
    #[must_use]
    pub const fn cardinality(&self) -> u64 {
        self.cardinality
    }

    /// Power sums `sum encode(value)^d` for `d = 1..=D` in each lane.
    #[must_use]
    pub const fn power_sums(&self) -> &[[F; K]; D] {
        &self.power_sums
    }
}

/// Composable moments grouped by exact invariant cell descriptors.
///
/// Callers feeding arbitrary rounds must ensure both `cell_descriptor` and
/// `value` are equivariant under graph relabeling. [`Self::analyze_initial`]
/// provides a safe built-in profile from exact one-hop relational records.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellMomentProfile<F, const K: usize, const D: usize>
where
    F: Field,
{
    id: CellMomentProfileId,
    round: u64,
    value_count: u64,
    cells: BTreeMap<Vec<u8>, CellMomentCell<F, K, D>>,
}

impl<F, const K: usize, const D: usize> CellMomentProfile<F, K, D>
where
    F: Field + CanonicalEncoding + StaticField,
{
    /// Creates an empty identified moment profile for one refinement round.
    ///
    /// # Errors
    ///
    /// Zero lanes or zero retained powers are rejected.
    pub fn new<E>(round: u64, encoder: &E) -> Result<Self, GraphError>
    where
        E: StructuralLaneEncoder<F, K>,
    {
        if K == 0 || D == 0 {
            return Err(GraphError::InvalidCellMomentProfile);
        }
        let lanes = u64::try_from(K).map_err(|_| GraphError::GraphTooLarge)?;
        let degrees = u64::try_from(D).map_err(|_| GraphError::GraphTooLarge)?;
        let mut hasher = Sha256::new();
        hasher.update(b"microfield/cell-moment-profile/v1\0");
        hasher.update(F::spec().field_id().as_bytes());
        hasher.update(encoder.encoder_id().as_bytes());
        hasher.update(lanes.to_be_bytes());
        hasher.update(degrees.to_be_bytes());
        hasher.update(round.to_be_bytes());
        Ok(Self {
            id: CellMomentProfileId(hasher.finalize().into()),
            round,
            value_count: 0,
            cells: BTreeMap::new(),
        })
    }

    /// Builds the invariant round-zero/one-hop profile of a normalized graph.
    ///
    /// Cells use exact vertex kind and label. Values additionally encode sorted
    /// incoming and outgoing relation, role, multiplicity, neighbor kind and
    /// neighbor label records.
    ///
    /// # Errors
    ///
    /// Propagates stable-size and lane-encoding failures without publishing a
    /// partially built profile.
    pub fn analyze_initial<E>(graph: &IncidenceGraph, encoder: &E) -> Result<Self, GraphError>
    where
        E: StructuralLaneEncoder<F, K>,
    {
        let mut profile = Self::new(0, encoder)?;
        for index in 0..graph.vertex_count() {
            let vertex = VertexId::new(index);
            let cell = vertex_token(graph, vertex)?;
            let value = local_value_token(graph, vertex)?;
            profile.absorb(&cell, &value, encoder)?;
        }
        Ok(profile)
    }

    /// Adds one equivariant value to an exact invariant cell.
    ///
    /// # Errors
    ///
    /// Propagates encoder failures and rejects stable counter overflow. Mutation
    /// occurs only after all fallible encoding and arithmetic preconditions.
    pub fn absorb<E>(
        &mut self,
        cell_descriptor: &[u8],
        value: &[u8],
        encoder: &E,
    ) -> Result<(), GraphError>
    where
        E: StructuralLaneEncoder<F, K>,
    {
        let encoded = encoder.encode_lanes(value)?;
        let next_total = self
            .value_count
            .checked_add(1)
            .ok_or(GraphError::GraphTooLarge)?;
        let current_cardinality = self
            .cells
            .get(cell_descriptor)
            .map_or(0, |cell| cell.cardinality);
        let next_cardinality = current_cardinality
            .checked_add(1)
            .ok_or(GraphError::GraphTooLarge)?;
        let cell = self
            .cells
            .entry(cell_descriptor.to_vec())
            .or_insert_with(|| CellMomentCell {
                descriptor: cell_descriptor.to_vec(),
                cardinality: 0,
                power_sums: [[F::ZERO; K]; D],
            });
        for (lane, encoded_lane) in encoded.into_iter().enumerate() {
            let mut power = F::ONE;
            for degree in 0..D {
                power = power.mul(encoded_lane);
                cell.power_sums[degree][lane] = cell.power_sums[degree][lane].add(power);
            }
        }
        cell.cardinality = next_cardinality;
        self.value_count = next_total;
        Ok(())
    }

    /// Adds profiles from a disjoint union cell by cell.
    ///
    /// # Errors
    ///
    /// Rejects identity drift and stable counter overflow.
    pub fn combine_disjoint(&self, other: &Self) -> Result<Self, GraphError> {
        if self.id != other.id {
            return Err(GraphError::CellMomentProfileMismatch);
        }
        let mut combined = self.clone();
        combined.value_count = combined
            .value_count
            .checked_add(other.value_count)
            .ok_or(GraphError::GraphTooLarge)?;
        for (descriptor, right) in &other.cells {
            let left = combined
                .cells
                .entry(descriptor.clone())
                .or_insert_with(|| CellMomentCell {
                    descriptor: descriptor.clone(),
                    cardinality: 0,
                    power_sums: [[F::ZERO; K]; D],
                });
            left.cardinality = left
                .cardinality
                .checked_add(right.cardinality)
                .ok_or(GraphError::GraphTooLarge)?;
            for degree in 0..D {
                for lane in 0..K {
                    left.power_sums[degree][lane] =
                        left.power_sums[degree][lane].add(right.power_sums[degree][lane]);
                }
            }
        }
        Ok(combined)
    }

    /// Complete field/encoder/lane/degree/round identity.
    #[must_use]
    pub const fn id(&self) -> CellMomentProfileId {
        self.id
    }

    /// Caller-defined refinement round.
    #[must_use]
    pub const fn round(&self) -> u64 {
        self.round
    }

    /// Total values represented by all cells.
    #[must_use]
    pub const fn value_count(&self) -> u64 {
        self.value_count
    }

    /// Cells in exact descriptor order.
    pub fn cells(&self) -> impl ExactSizeIterator<Item = &CellMomentCell<F, K, D>> {
        self.cells.values()
    }

    /// Power moments over finite fields remain collision-prone fingerprints.
    #[must_use]
    pub const fn assurance(&self) -> SignatureAssurance {
        SignatureAssurance::Fingerprint
    }

    /// Stable field-specific wire.
    #[must_use]
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MOMENT_MAGIC);
        bytes.extend_from_slice(&MOMENT_SCHEMA.to_be_bytes());
        bytes.extend_from_slice(self.id.as_bytes());
        bytes.extend_from_slice(F::spec().field_id().as_bytes());
        bytes.extend_from_slice(&self.round.to_be_bytes());
        bytes.extend_from_slice(&self.value_count.to_be_bytes());
        bytes.extend_from_slice(&u64::try_from(K).unwrap_or(u64::MAX).to_be_bytes());
        bytes.extend_from_slice(&u64::try_from(D).unwrap_or(u64::MAX).to_be_bytes());
        bytes.extend_from_slice(
            &u64::try_from(self.cells.len())
                .unwrap_or(u64::MAX)
                .to_be_bytes(),
        );
        for cell in self.cells.values() {
            append_framed(&mut bytes, &cell.descriptor).expect("validated descriptor length");
            bytes.extend_from_slice(&cell.cardinality.to_be_bytes());
            for degree in &cell.power_sums {
                for value in degree {
                    bytes.extend_from_slice(value.to_canonical().as_ref());
                }
            }
        }
        bytes
    }
}

fn vertex_token(graph: &IncidenceGraph, vertex: VertexId) -> Result<Vec<u8>, GraphError> {
    let mut token = Vec::new();
    token.push(graph.vertex_kind(vertex) as u8);
    append_framed(&mut token, graph.vertex_label(vertex))?;
    Ok(token)
}

fn local_value_token(graph: &IncidenceGraph, vertex: VertexId) -> Result<Vec<u8>, GraphError> {
    let mut token = vertex_token(graph, vertex)?;
    let mut records = Vec::new();
    for (direction, incidences) in [
        (1_u8, graph.outgoing(vertex)),
        (2_u8, graph.incoming(vertex)),
    ] {
        for incidence in incidences {
            let mut record = Vec::new();
            record.push(direction);
            let descriptor = graph.relation(incidence.relation());
            append_framed(&mut record, descriptor.relation())?;
            append_framed(&mut record, descriptor.role())?;
            record.extend_from_slice(&incidence.multiplicity().to_be_bytes());
            let neighbor = incidence.neighbor();
            record.push(graph.vertex_kind(neighbor) as u8);
            append_framed(&mut record, graph.vertex_label(neighbor))?;
            records.push(record);
        }
    }
    records.sort_unstable();
    token.extend_from_slice(
        &u64::try_from(records.len())
            .map_err(|_| GraphError::GraphTooLarge)?
            .to_be_bytes(),
    );
    for record in records {
        append_framed(&mut token, &record)?;
    }
    Ok(token)
}

fn append_framed(output: &mut Vec<u8>, bytes: &[u8]) -> Result<(), GraphError> {
    output.extend_from_slice(
        &u64::try_from(bytes.len())
            .map_err(|_| GraphError::GraphTooLarge)?
            .to_be_bytes(),
    );
    output.extend_from_slice(bytes);
    Ok(())
}
