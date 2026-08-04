//! Experimental relational matrix invariants over generated finite fields.

use core::fmt;

use microfield::{CanonicalEncoding, Field, Invert, StaticField};
use sha2::{Digest as _, Sha256};

use crate::structural::{SignatureAssurance, StructuralLaneEncoder};

use super::super::{GraphError, IncidenceGraph, VertexId};

const MATRIX_MAGIC: &[u8; 4] = b"MFRM";
const MATRIX_SCHEMA: u16 = 1;
const MAXIMUM_TRACE_POWER: u8 = 64;

/// Stable identity of one relational matrix operator and evaluation profile.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct RelationalMatrixProfileId([u8; 32]);

impl RelationalMatrixProfileId {
    /// Borrows the stable digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for RelationalMatrixProfileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for RelationalMatrixProfileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "RelationalMatrixProfileId({self})")
    }
}

/// Whether all requested matrix invariants were computed transactionally.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum MatrixAnalysisStatus {
    /// Every trace and characteristic evaluation is available.
    Complete = 1,
    /// The invariant work estimate exceeded the caller's ceiling.
    SkippedBudget = 2,
}

/// Finite-field traces and characteristic evaluations of a relational operator.
///
/// For every lane, the operator places an independently encoded vertex token
/// on the diagonal and an independently encoded relation/role token at each
/// directed incidence. Relabeling therefore conjugates the matrix by a
/// permutation. Traces and determinants are invariant under that conjugation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationalMatrixProfile<F, const K: usize>
where
    F: Field,
{
    id: RelationalMatrixProfileId,
    status: MatrixAnalysisStatus,
    maximum_trace_power: u8,
    estimated_scalar_products: u64,
    graph_count: u64,
    vertex_count: u64,
    traces: Vec<[F; K]>,
    characteristic_evaluations: Option<[F; K]>,
}

impl<F, const K: usize> RelationalMatrixProfile<F, K>
where
    F: Field + CanonicalEncoding + Invert + StaticField,
{
    /// Evaluates a bounded profile or returns an atomically skipped value.
    ///
    /// `maximum_scalar_products` controls the dense reference implementation.
    /// The estimate covers all matrix powers and Gaussian eliminations. It is
    /// independent of graph numbering and therefore cannot leak a partial
    /// invariant through data-dependent early termination.
    ///
    /// # Errors
    ///
    /// Rejects zero lanes, trace powers outside `1..=64`, stable-size overflow,
    /// allocation failure reported by the encoder, or hash-to-field exhaustion.
    pub fn analyze<E>(
        graph: &IncidenceGraph,
        maximum_trace_power: u8,
        encoder: &E,
        maximum_scalar_products: u64,
    ) -> Result<Self, GraphError>
    where
        E: StructuralLaneEncoder<F, K>,
    {
        if K == 0 || !(1..=MAXIMUM_TRACE_POWER).contains(&maximum_trace_power) {
            return Err(GraphError::InvalidMatrixProfile);
        }
        let id = derive_id::<F, E, K>(maximum_trace_power, encoder)?;
        let vertex_count =
            u64::try_from(graph.vertex_count()).map_err(|_| GraphError::GraphTooLarge)?;
        let estimated_scalar_products = estimate_work::<K>(vertex_count, maximum_trace_power)?;
        if estimated_scalar_products > maximum_scalar_products {
            return Ok(Self {
                id,
                status: MatrixAnalysisStatus::SkippedBudget,
                maximum_trace_power,
                estimated_scalar_products,
                graph_count: 1,
                vertex_count,
                traces: Vec::new(),
                characteristic_evaluations: None,
            });
        }

        let operator = build_operator::<F, E, K>(graph, encoder)?;
        let evaluation_points = encoder.encode_lanes(b"matrix/characteristic-evaluation/v1")?;
        let characteristic_evaluations = core::array::from_fn(|lane| {
            determinant_at::<F, K>(
                &operator,
                graph.vertex_count(),
                lane,
                evaluation_points[lane],
            )
        });
        let mut traces = Vec::new();
        traces
            .try_reserve_exact(usize::from(maximum_trace_power))
            .map_err(|_| GraphError::GraphTooLarge)?;
        let mut power = operator.clone();
        for exponent in 1..=maximum_trace_power {
            traces.push(matrix_trace::<F, K>(&power, graph.vertex_count()));
            if exponent != maximum_trace_power {
                power = multiply::<F, K>(&power, &operator, graph.vertex_count());
            }
        }
        Ok(Self {
            id,
            status: MatrixAnalysisStatus::Complete,
            maximum_trace_power,
            estimated_scalar_products,
            graph_count: 1,
            vertex_count,
            traces,
            characteristic_evaluations: Some(characteristic_evaluations),
        })
    }

    /// Combines block-diagonal operators representing a disjoint union.
    ///
    /// Traces add and characteristic-polynomial evaluations multiply.
    ///
    /// # Errors
    ///
    /// Rejects profile drift, skipped operands and stable counter overflow.
    pub fn combine_disjoint(&self, other: &Self) -> Result<Self, GraphError> {
        if self.id != other.id {
            return Err(GraphError::MatrixProfileMismatch);
        }
        if self.status != MatrixAnalysisStatus::Complete
            || other.status != MatrixAnalysisStatus::Complete
        {
            return Err(GraphError::MatrixAnalysisIncomplete);
        }
        let left_characteristic = self
            .characteristic_evaluations
            .ok_or(GraphError::MatrixAnalysisIncomplete)?;
        let right_characteristic = other
            .characteristic_evaluations
            .ok_or(GraphError::MatrixAnalysisIncomplete)?;
        let traces = self
            .traces
            .iter()
            .zip(&other.traces)
            .map(|(left, right)| core::array::from_fn(|lane| left[lane].add(right[lane])))
            .collect();
        Ok(Self {
            id: self.id,
            status: MatrixAnalysisStatus::Complete,
            maximum_trace_power: self.maximum_trace_power,
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
            traces,
            characteristic_evaluations: Some(core::array::from_fn(|lane| {
                left_characteristic[lane].mul(right_characteristic[lane])
            })),
        })
    }

    /// Complete operator/field/encoder/power identity.
    #[must_use]
    pub const fn id(&self) -> RelationalMatrixProfileId {
        self.id
    }

    /// Complete or atomically skipped status.
    #[must_use]
    pub const fn status(&self) -> MatrixAnalysisStatus {
        self.status
    }

    /// Largest computed positive power.
    #[must_use]
    pub const fn maximum_trace_power(&self) -> u8 {
        self.maximum_trace_power
    }

    /// Invariant preflight estimate used for admission.
    #[must_use]
    pub const fn estimated_scalar_products(&self) -> u64 {
        self.estimated_scalar_products
    }

    /// Number of disjoint source graphs represented.
    #[must_use]
    pub const fn graph_count(&self) -> u64 {
        self.graph_count
    }

    /// Total number of represented source vertices.
    #[must_use]
    pub const fn vertex_count(&self) -> u64 {
        self.vertex_count
    }

    /// `trace(A^k)` for `k = 1..=maximum_trace_power`, one value per lane.
    #[must_use]
    pub fn traces(&self) -> &[[F; K]] {
        &self.traces
    }

    /// `det(t_lane I - A_lane)` for every independent lane.
    #[must_use]
    pub const fn characteristic_evaluations(&self) -> Option<&[F; K]> {
        self.characteristic_evaluations.as_ref()
    }

    /// Finite-field matrix evaluations are collision-prone fingerprints.
    #[must_use]
    pub const fn assurance(&self) -> SignatureAssurance {
        SignatureAssurance::Fingerprint
    }

    /// Stable field-specific wire for persistence and differential fixtures.
    #[must_use]
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let repr_len = F::ZERO.to_canonical().as_ref().len();
        let value_count = self
            .traces
            .len()
            .saturating_mul(K)
            .saturating_add(self.characteristic_evaluations.map_or(0, |_| K));
        let mut bytes =
            Vec::with_capacity(105_usize.saturating_add(value_count.saturating_mul(repr_len)));
        bytes.extend_from_slice(MATRIX_MAGIC);
        bytes.extend_from_slice(&MATRIX_SCHEMA.to_be_bytes());
        bytes.extend_from_slice(self.id.as_bytes());
        bytes.extend_from_slice(F::spec().field_id().as_bytes());
        bytes.push(self.status as u8);
        bytes.push(self.maximum_trace_power);
        bytes.extend_from_slice(&[0, 0]);
        bytes.extend_from_slice(&self.estimated_scalar_products.to_be_bytes());
        bytes.extend_from_slice(&self.graph_count.to_be_bytes());
        bytes.extend_from_slice(&self.vertex_count.to_be_bytes());
        bytes.extend_from_slice(&u64::try_from(K).unwrap_or(u64::MAX).to_be_bytes());
        for trace in &self.traces {
            for value in trace {
                bytes.extend_from_slice(value.to_canonical().as_ref());
            }
        }
        if let Some(characteristic) = self.characteristic_evaluations {
            for value in characteristic {
                bytes.extend_from_slice(value.to_canonical().as_ref());
            }
        }
        bytes
    }
}

fn derive_id<F, E, const K: usize>(
    maximum_trace_power: u8,
    encoder: &E,
) -> Result<RelationalMatrixProfileId, GraphError>
where
    F: Field + StaticField,
    E: StructuralLaneEncoder<F, K>,
{
    let lanes = u64::try_from(K).map_err(|_| GraphError::GraphTooLarge)?;
    let mut hasher = Sha256::new();
    hasher.update(b"microfield/relational-matrix-profile/v1\0");
    hasher.update(F::spec().field_id().as_bytes());
    hasher.update(encoder.encoder_id().as_bytes());
    hasher.update(lanes.to_be_bytes());
    hasher.update([maximum_trace_power]);
    Ok(RelationalMatrixProfileId(hasher.finalize().into()))
}

fn estimate_work<const K: usize>(
    vertex_count: u64,
    maximum_trace_power: u8,
) -> Result<u64, GraphError> {
    u64::try_from(K)
        .map_err(|_| GraphError::GraphTooLarge)?
        .checked_mul(u64::from(maximum_trace_power))
        .and_then(|value| value.checked_mul(vertex_count))
        .and_then(|value| value.checked_mul(vertex_count))
        .and_then(|value| value.checked_mul(vertex_count))
        .ok_or(GraphError::GraphTooLarge)
}

fn build_operator<F, E, const K: usize>(
    graph: &IncidenceGraph,
    encoder: &E,
) -> Result<Vec<[F; K]>, GraphError>
where
    F: Field,
    E: StructuralLaneEncoder<F, K>,
{
    let order = graph.vertex_count();
    let matrix_len = order.checked_mul(order).ok_or(GraphError::GraphTooLarge)?;
    let mut matrix = vec![[F::ZERO; K]; matrix_len];
    for source_index in 0..order {
        let source = VertexId::new(source_index);
        let mut vertex_token = Vec::new();
        vertex_token.push(1);
        vertex_token.push(graph.vertex_kind(source) as u8);
        append_framed(&mut vertex_token, graph.vertex_label(source))?;
        let diagonal = encoder.encode_lanes(&vertex_token)?;
        add_lanes(&mut matrix[source_index * order + source_index], diagonal);

        for incidence in graph.outgoing(source) {
            let descriptor = graph.relation(incidence.relation());
            let mut relation_token = Vec::new();
            relation_token.push(2);
            append_framed(&mut relation_token, descriptor.relation())?;
            append_framed(&mut relation_token, descriptor.role())?;
            let relation = encoder.encode_lanes(&relation_token)?;
            let target = incidence.neighbor().index();
            let cell = &mut matrix[source_index * order + target];
            for lane in 0..K {
                cell[lane] = cell[lane].add(scale_by_u64(relation[lane], incidence.multiplicity()));
            }
        }
    }
    Ok(matrix)
}

fn add_lanes<F: Field, const K: usize>(target: &mut [F; K], value: [F; K]) {
    for lane in 0..K {
        target[lane] = target[lane].add(value[lane]);
    }
}

fn matrix_trace<F: Field, const K: usize>(matrix: &[[F; K]], order: usize) -> [F; K] {
    let mut trace = [F::ZERO; K];
    for diagonal in 0..order {
        add_lanes(&mut trace, matrix[diagonal * order + diagonal]);
    }
    trace
}

fn multiply<F: Field, const K: usize>(
    left: &[[F; K]],
    right: &[[F; K]],
    order: usize,
) -> Vec<[F; K]> {
    let mut output = vec![[F::ZERO; K]; order.saturating_mul(order)];
    for row in 0..order {
        for inner in 0..order {
            let left_value = left[row * order + inner];
            for column in 0..order {
                let right_value = right[inner * order + column];
                let target = &mut output[row * order + column];
                for lane in 0..K {
                    target[lane] = target[lane].add(left_value[lane].mul(right_value[lane]));
                }
            }
        }
    }
    output
}

fn determinant_at<F: Field + Invert, const K: usize>(
    operator: &[[F; K]],
    order: usize,
    lane: usize,
    point: F,
) -> F {
    let mut matrix = Vec::with_capacity(order.saturating_mul(order));
    for row in 0..order {
        for column in 0..order {
            let value = operator[row * order + column][lane];
            matrix.push(if row == column {
                point.sub(value)
            } else {
                F::ZERO.sub(value)
            });
        }
    }
    let mut determinant = F::ONE;
    for column in 0..order {
        let Some(pivot_row) = (column..order).find(|&row| !matrix[row * order + column].is_zero())
        else {
            return F::ZERO;
        };
        if pivot_row != column {
            for index in column..order {
                matrix.swap(column * order + index, pivot_row * order + index);
            }
            determinant = determinant.neg();
        }
        let pivot = matrix[column * order + column];
        determinant = determinant.mul(pivot);
        let Some(inverse) = pivot.invert() else {
            return F::ZERO;
        };
        for row in column + 1..order {
            let factor = matrix[row * order + column].mul(inverse);
            matrix[row * order + column] = F::ZERO;
            for index in column + 1..order {
                let correction = factor.mul(matrix[column * order + index]);
                matrix[row * order + index] = matrix[row * order + index].sub(correction);
            }
        }
    }
    determinant
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
