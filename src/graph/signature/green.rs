//! Experimental RG2 theta contractions over relational adjacency operators.

use core::fmt;

use microfield::{CanonicalEncoding, Field, StaticField};
use sha2::{Digest as _, Sha256};

use crate::structural::{SignatureAssurance, StructuralLaneEncoder};

use super::super::{GraphError, IncidenceGraph, VertexId};

const THETA_MAGIC: &[u8; 4] = b"MFTH";
const THETA_SCHEMA: u16 = 1;
const THETA_TRIPLES: [(u8, u8, u8); 6] = [
    (1, 2, 2),
    (1, 2, 3),
    (1, 3, 3),
    (2, 2, 2),
    (2, 2, 3),
    (2, 3, 3),
];

/// Stable identity of the RG2 theta-contraction catalog and field profile.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct RelationalThetaProfileId([u8; 32]);

impl RelationalThetaProfileId {
    /// Borrows the stable digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for RelationalThetaProfileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for RelationalThetaProfileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "RelationalThetaProfileId({self})")
    }
}

/// Whether every frozen RG2 contraction was computed.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum ThetaAnalysisStatus {
    /// All six contractions are available.
    Complete = 1,
    /// The invariant preflight exceeded the caller's work ceiling.
    SkippedBudget = 2,
}

/// Fixed RG2 family `sum_(u,v) A^a_uv A^b_uv A^c_uv`.
///
/// The six path triples are frozen in schema v1 and include theta patterns with
/// two independent loops. The operator contains typed directed relation/role
/// weights and exact multiplicity, but no diagonal vertex term. This is an
/// experimental invariant and not an isomorphism proof.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationalThetaProfile<F, const K: usize>
where
    F: Field,
{
    id: RelationalThetaProfileId,
    status: ThetaAnalysisStatus,
    estimated_scalar_products: u64,
    graph_count: u64,
    vertex_count: u64,
    contractions: [[F; K]; 6],
}

impl<F, const K: usize> RelationalThetaProfile<F, K>
where
    F: Field + CanonicalEncoding + StaticField,
{
    /// Evaluates the frozen RG2 catalog or skips atomically.
    ///
    /// # Errors
    ///
    /// Rejects zero lanes, stable-size overflow and lane-encoding failures.
    pub fn analyze<E>(
        graph: &IncidenceGraph,
        encoder: &E,
        maximum_scalar_products: u64,
    ) -> Result<Self, GraphError>
    where
        E: StructuralLaneEncoder<F, K>,
    {
        if K == 0 {
            return Err(GraphError::InvalidThetaProfile);
        }
        let id = derive_id::<F, E, K>(encoder)?;
        let order = u64::try_from(graph.vertex_count()).map_err(|_| GraphError::GraphTooLarge)?;
        let estimated_scalar_products = estimate_work::<K>(order)?;
        if estimated_scalar_products > maximum_scalar_products {
            return Ok(Self {
                id,
                status: ThetaAnalysisStatus::SkippedBudget,
                estimated_scalar_products,
                graph_count: 1,
                vertex_count: order,
                contractions: [[F::ZERO; K]; 6],
            });
        }
        let adjacency = build_adjacency::<F, E, K>(graph, encoder)?;
        let squared = multiply::<F, K>(&adjacency, &adjacency, graph.vertex_count());
        let cubed = multiply::<F, K>(&squared, &adjacency, graph.vertex_count());
        let powers = [&adjacency, &squared, &cubed];
        let contractions = core::array::from_fn(|index| {
            let (first, second, third) = THETA_TRIPLES[index];
            contract::<F, K>(
                powers[usize::from(first - 1)],
                powers[usize::from(second - 1)],
                powers[usize::from(third - 1)],
            )
        });
        Ok(Self {
            id,
            status: ThetaAnalysisStatus::Complete,
            estimated_scalar_products,
            graph_count: 1,
            vertex_count: order,
            contractions,
        })
    }

    /// Adds contractions for a block-diagonal disjoint union.
    ///
    /// # Errors
    ///
    /// Rejects profile drift, skipped operands and stable counter overflow.
    pub fn combine_disjoint(&self, other: &Self) -> Result<Self, GraphError> {
        if self.id != other.id {
            return Err(GraphError::ThetaProfileMismatch);
        }
        if self.status != ThetaAnalysisStatus::Complete
            || other.status != ThetaAnalysisStatus::Complete
        {
            return Err(GraphError::ThetaAnalysisIncomplete);
        }
        Ok(Self {
            id: self.id,
            status: ThetaAnalysisStatus::Complete,
            estimated_scalar_products: self
                .estimated_scalar_products
                .checked_add(other.estimated_scalar_products)
                .ok_or(GraphError::GraphTooLarge)?,
            graph_count: self
                .graph_count
                .checked_add(other.graph_count)
                .ok_or(GraphError::GraphTooLarge)?,
            vertex_count: self
                .vertex_count
                .checked_add(other.vertex_count)
                .ok_or(GraphError::GraphTooLarge)?,
            contractions: core::array::from_fn(|index| {
                core::array::from_fn(|lane| {
                    self.contractions[index][lane].add(other.contractions[index][lane])
                })
            }),
        })
    }

    /// Frozen ordered path triples `(a,b,c)`.
    #[must_use]
    pub const fn path_triples() -> &'static [(u8, u8, u8); 6] {
        &THETA_TRIPLES
    }

    /// Complete catalog/field/encoder identity.
    #[must_use]
    pub const fn id(&self) -> RelationalThetaProfileId {
        self.id
    }

    /// Complete or atomically skipped status.
    #[must_use]
    pub const fn status(&self) -> ThetaAnalysisStatus {
        self.status
    }

    /// Invariant dense-work preflight.
    #[must_use]
    pub const fn estimated_scalar_products(&self) -> u64 {
        self.estimated_scalar_products
    }

    /// Six RG2 contractions, one value per independent field lane.
    #[must_use]
    pub const fn contractions(&self) -> &[[F; K]; 6] {
        &self.contractions
    }

    /// Finite-field contractions are collision-prone fingerprints.
    #[must_use]
    pub const fn assurance(&self) -> SignatureAssurance {
        SignatureAssurance::Fingerprint
    }

    /// Stable field-specific wire.
    #[must_use]
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let repr_len = F::ZERO.to_canonical().as_ref().len();
        let mut bytes = Vec::with_capacity(101 + 6 * K.saturating_mul(repr_len));
        bytes.extend_from_slice(THETA_MAGIC);
        bytes.extend_from_slice(&THETA_SCHEMA.to_be_bytes());
        bytes.extend_from_slice(self.id.as_bytes());
        bytes.extend_from_slice(F::spec().field_id().as_bytes());
        bytes.push(self.status as u8);
        bytes.push(0);
        bytes.extend_from_slice(&self.estimated_scalar_products.to_be_bytes());
        bytes.extend_from_slice(&self.graph_count.to_be_bytes());
        bytes.extend_from_slice(&self.vertex_count.to_be_bytes());
        bytes.extend_from_slice(&u64::try_from(K).unwrap_or(u64::MAX).to_be_bytes());
        for contraction in self.contractions {
            for value in contraction {
                bytes.extend_from_slice(value.to_canonical().as_ref());
            }
        }
        bytes
    }
}

fn derive_id<F, E, const K: usize>(encoder: &E) -> Result<RelationalThetaProfileId, GraphError>
where
    F: Field + StaticField,
    E: StructuralLaneEncoder<F, K>,
{
    let mut hasher = Sha256::new();
    hasher.update(b"microfield/relational-theta-profile/v1\0");
    hasher.update(F::spec().field_id().as_bytes());
    hasher.update(encoder.encoder_id().as_bytes());
    hasher.update(
        u64::try_from(K)
            .map_err(|_| GraphError::GraphTooLarge)?
            .to_be_bytes(),
    );
    for (first, second, third) in THETA_TRIPLES {
        hasher.update([first, second, third]);
    }
    Ok(RelationalThetaProfileId(hasher.finalize().into()))
}

fn estimate_work<const K: usize>(order: u64) -> Result<u64, GraphError> {
    let lanes = u64::try_from(K).map_err(|_| GraphError::GraphTooLarge)?;
    let matrix_products = lanes
        .checked_mul(2)
        .and_then(|value| value.checked_mul(order))
        .and_then(|value| value.checked_mul(order))
        .and_then(|value| value.checked_mul(order))
        .ok_or(GraphError::GraphTooLarge)?;
    let contractions = lanes
        .checked_mul(12)
        .and_then(|value| value.checked_mul(order))
        .and_then(|value| value.checked_mul(order))
        .ok_or(GraphError::GraphTooLarge)?;
    matrix_products
        .checked_add(contractions)
        .ok_or(GraphError::GraphTooLarge)
}

fn build_adjacency<F, E, const K: usize>(
    graph: &IncidenceGraph,
    encoder: &E,
) -> Result<Vec<[F; K]>, GraphError>
where
    F: Field,
    E: StructuralLaneEncoder<F, K>,
{
    let order = graph.vertex_count();
    let mut matrix = vec![[F::ZERO; K]; order.checked_mul(order).ok_or(GraphError::GraphTooLarge)?];
    for source_index in 0..order {
        for incidence in graph.outgoing(VertexId::new(source_index)) {
            let descriptor = graph.relation(incidence.relation());
            let mut token = vec![3];
            append_framed(&mut token, descriptor.relation())?;
            append_framed(&mut token, descriptor.role())?;
            let relation = encoder.encode_lanes(&token)?;
            let cell = &mut matrix[source_index * order + incidence.neighbor().index()];
            for lane in 0..K {
                cell[lane] = cell[lane].add(scale_by_u64(relation[lane], incidence.multiplicity()));
            }
        }
    }
    Ok(matrix)
}

fn multiply<F: Field, const K: usize>(
    left: &[[F; K]],
    right: &[[F; K]],
    order: usize,
) -> Vec<[F; K]> {
    let mut output = vec![[F::ZERO; K]; order.saturating_mul(order)];
    for row in 0..order {
        for inner in 0..order {
            for column in 0..order {
                for lane in 0..K {
                    output[row * order + column][lane] = output[row * order + column][lane].add(
                        left[row * order + inner][lane].mul(right[inner * order + column][lane]),
                    );
                }
            }
        }
    }
    output
}

fn contract<F: Field, const K: usize>(
    first: &[[F; K]],
    second: &[[F; K]],
    third: &[[F; K]],
) -> [F; K] {
    let mut output = [F::ZERO; K];
    for index in 0..first.len() {
        for lane in 0..K {
            output[lane] = output[lane].add(
                first[index][lane]
                    .mul(second[index][lane])
                    .mul(third[index][lane]),
            );
        }
    }
    output
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

fn scale_by_u64<F: Field>(value: F, mut scalar: u64) -> F {
    let mut result = F::ZERO;
    let mut addend = value;
    while scalar != 0 {
        if scalar & 1 == 1 {
            result = result.add(addend);
        }
        scalar >>= 1;
        if scalar != 0 {
            addend = addend.add(addend);
        }
    }
    result
}
