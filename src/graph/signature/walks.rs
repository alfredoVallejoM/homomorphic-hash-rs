//! Long relational closed-walk traces through certified linear recurrences.

use core::fmt;

use microfield::{CanonicalEncoding, Field, Invert, StaticField};
use sha2::{Digest as _, Sha256};

use crate::structural::{SignatureAssurance, StructuralLaneEncoder};

use super::super::{GraphError, IncidenceGraph, VertexId};

const WALK_MAGIC: &[u8; 4] = b"MFCW";
const WALK_SCHEMA: u16 = 1;
const MAXIMUM_QUERY_COUNT: usize = 1_024;

/// Relabeling-equivariant operator whose closed walks are queried.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum ClosedWalkOperator {
    /// Relational vertex adjacency including typed diagonal labels.
    Adjacency = 1,
    /// Hashimoto-style transitions that forbid immediate edge reversal.
    NonBacktracking = 2,
}

/// Stable identity of a sorted set of positive closed-walk lengths.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ClosedWalkQueryPlanId([u8; 32]);

impl ClosedWalkQueryPlanId {
    /// Borrows the stable digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for ClosedWalkQueryPlanId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for ClosedWalkQueryPlanId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "ClosedWalkQueryPlanId({self})")
    }
}

/// Immutable, identity-bound set of lengths queried from `trace(A^k)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClosedWalkQueryPlan {
    id: ClosedWalkQueryPlanId,
    lengths: Vec<u64>,
}

impl ClosedWalkQueryPlan {
    /// Validates, sorts and deduplicates positive lengths.
    ///
    /// # Errors
    ///
    /// Rejects empty plans, length zero and more than 1,024 distinct queries.
    pub fn new(lengths: impl Into<Vec<u64>>) -> Result<Self, GraphError> {
        let mut lengths = lengths.into();
        lengths.sort_unstable();
        lengths.dedup();
        if lengths.is_empty() || lengths.len() > MAXIMUM_QUERY_COUNT || lengths.first() == Some(&0)
        {
            return Err(GraphError::InvalidClosedWalkPlan);
        }
        let mut hasher = Sha256::new();
        hasher.update(b"microfield/closed-walk-query-plan/v1\0");
        hasher.update(
            u64::try_from(lengths.len())
                .map_err(|_| GraphError::GraphTooLarge)?
                .to_be_bytes(),
        );
        for length in &lengths {
            hasher.update(length.to_be_bytes());
        }
        Ok(Self {
            id: ClosedWalkQueryPlanId(hasher.finalize().into()),
            lengths,
        })
    }

    /// Stable ordered-query identity.
    #[must_use]
    pub const fn id(&self) -> ClosedWalkQueryPlanId {
        self.id
    }

    /// Sorted distinct positive lengths.
    #[must_use]
    pub fn lengths(&self) -> &[u64] {
        &self.lengths
    }
}

/// Stable identity of field, encoder, lanes and closed-walk plan.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct RelationalClosedWalkProfileId([u8; 32]);

impl RelationalClosedWalkProfileId {
    /// Borrows the stable digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for RelationalClosedWalkProfileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for RelationalClosedWalkProfileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "RelationalClosedWalkProfileId({self})")
    }
}

/// Whether all requested trace lengths were evaluated transactionally.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum ClosedWalkAnalysisStatus {
    /// Seed traces, recurrences and every requested term are available.
    Complete = 1,
    /// The invariant preflight exceeded the caller's work ceiling.
    SkippedBudget = 2,
}

/// Exact finite-field traces `trace(A^k)` at potentially huge `u64` lengths.
///
/// At most `2n + 1` consecutive traces are computed. Berlekamp--Massey then
/// recovers a recurrence of order at most `n`, guaranteed to exist by
/// Cayley--Hamilton. Binary exponentiation in the quotient recurrence ring
/// evaluates distant indices without walking through every intermediate
/// length. Values remain finite-field fingerprints, not simple-cycle counts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationalClosedWalkProfile<F, const K: usize>
where
    F: Field,
{
    id: RelationalClosedWalkProfileId,
    plan: ClosedWalkQueryPlan,
    operator: ClosedWalkOperator,
    status: ClosedWalkAnalysisStatus,
    estimated_field_operations: u64,
    graph_count: u64,
    vertex_count: u64,
    recurrence_orders: [u32; K],
    traces: Vec<[F; K]>,
}

impl<F, const K: usize> RelationalClosedWalkProfile<F, K>
where
    F: Field + CanonicalEncoding + Invert + StaticField,
{
    /// Evaluates long trace queries or skips before allocating large buffers.
    ///
    /// # Errors
    ///
    /// Rejects zero lanes, overflows and encoder failures.
    pub fn analyze<E>(
        graph: &IncidenceGraph,
        plan: ClosedWalkQueryPlan,
        encoder: &E,
        maximum_field_operations: u64,
    ) -> Result<Self, GraphError>
    where
        E: StructuralLaneEncoder<F, K>,
    {
        Self::analyze_operator(
            graph,
            plan,
            ClosedWalkOperator::Adjacency,
            encoder,
            maximum_field_operations,
        )
    }

    /// Evaluates long non-backtracking trace queries over incidence states.
    ///
    /// For an undirected graph represented by paired directed incidences, a
    /// transition cannot return immediately through the reverse endpoint.
    /// Self-loops remain admissible as one-state cycles. Multiplicity and the
    /// complete relation/role descriptor are retained in transition weights.
    ///
    /// # Errors
    ///
    /// Rejects zero lanes, stable-size overflow and encoder failures.
    pub fn analyze_non_backtracking<E>(
        graph: &IncidenceGraph,
        plan: ClosedWalkQueryPlan,
        encoder: &E,
        maximum_field_operations: u64,
    ) -> Result<Self, GraphError>
    where
        E: StructuralLaneEncoder<F, K>,
    {
        Self::analyze_operator(
            graph,
            plan,
            ClosedWalkOperator::NonBacktracking,
            encoder,
            maximum_field_operations,
        )
    }

    fn analyze_operator<E>(
        graph: &IncidenceGraph,
        plan: ClosedWalkQueryPlan,
        operator_kind: ClosedWalkOperator,
        encoder: &E,
        maximum_field_operations: u64,
    ) -> Result<Self, GraphError>
    where
        E: StructuralLaneEncoder<F, K>,
    {
        if K == 0 {
            return Err(GraphError::InvalidClosedWalkProfile);
        }
        let id = derive_id::<F, E, K>(&plan, operator_kind, encoder)?;
        let (operator_order, transition_count) = match operator_kind {
            ClosedWalkOperator::Adjacency => (
                graph.vertex_count(),
                graph
                    .vertex_count()
                    .checked_add(graph.incidence_count())
                    .ok_or(GraphError::GraphTooLarge)?,
            ),
            ClosedWalkOperator::NonBacktracking => (
                graph.incidence_count(),
                non_backtracking_transition_count(graph)?,
            ),
        };
        let estimated_field_operations =
            estimate_work::<K>(operator_order, transition_count, plan.lengths.len())?;
        let vertex_count =
            u64::try_from(graph.vertex_count()).map_err(|_| GraphError::GraphTooLarge)?;
        if estimated_field_operations > maximum_field_operations {
            return Ok(Self {
                id,
                plan,
                operator: operator_kind,
                status: ClosedWalkAnalysisStatus::SkippedBudget,
                estimated_field_operations,
                graph_count: 1,
                vertex_count,
                recurrence_orders: [0; K],
                traces: Vec::new(),
            });
        }

        let operator = match operator_kind {
            ClosedWalkOperator::Adjacency => build_adjacency_operator::<F, E, K>(graph, encoder)?,
            ClosedWalkOperator::NonBacktracking => {
                build_non_backtracking_operator::<F, E, K>(graph, encoder)?
            }
        };
        let seed = seed_traces::<F, K>(operator_order, &operator)?;
        let mut recurrence_orders = [0_u32; K];
        let mut recurrences = Vec::with_capacity(K);
        for lane in 0..K {
            let lane_seed = seed.iter().map(|values| values[lane]).collect::<Vec<_>>();
            let recurrence = berlekamp_massey(&lane_seed)?;
            recurrence_orders[lane] =
                u32::try_from(recurrence.len()).map_err(|_| GraphError::GraphTooLarge)?;
            recurrences.push((lane_seed, recurrence));
        }
        let traces = plan
            .lengths
            .iter()
            .map(|&length| {
                core::array::from_fn(|lane| {
                    linear_recurrence_term(&recurrences[lane].0, &recurrences[lane].1, length)
                })
            })
            .collect();
        Ok(Self {
            id,
            plan,
            operator: operator_kind,
            status: ClosedWalkAnalysisStatus::Complete,
            estimated_field_operations,
            graph_count: 1,
            vertex_count,
            recurrence_orders,
            traces,
        })
    }

    /// Adds traces for a disjoint block-diagonal union.
    ///
    /// # Errors
    ///
    /// Rejects identity drift or incomplete operands.
    pub fn combine_disjoint(&self, other: &Self) -> Result<Self, GraphError> {
        if self.id != other.id {
            return Err(GraphError::ClosedWalkProfileMismatch);
        }
        if self.status != ClosedWalkAnalysisStatus::Complete
            || other.status != ClosedWalkAnalysisStatus::Complete
        {
            return Err(GraphError::ClosedWalkAnalysisIncomplete);
        }
        Ok(Self {
            id: self.id,
            plan: self.plan.clone(),
            operator: self.operator,
            status: ClosedWalkAnalysisStatus::Complete,
            estimated_field_operations: self
                .estimated_field_operations
                .checked_add(other.estimated_field_operations)
                .ok_or(GraphError::GraphTooLarge)?,
            graph_count: self
                .graph_count
                .checked_add(other.graph_count)
                .ok_or(GraphError::GraphTooLarge)?,
            vertex_count: self
                .vertex_count
                .checked_add(other.vertex_count)
                .ok_or(GraphError::GraphTooLarge)?,
            recurrence_orders: core::array::from_fn(|lane| {
                self.recurrence_orders[lane].max(other.recurrence_orders[lane])
            }),
            traces: self
                .traces
                .iter()
                .zip(&other.traces)
                .map(|(left, right)| core::array::from_fn(|lane| left[lane].add(right[lane])))
                .collect(),
        })
    }

    /// Complete field/encoder/plan identity.
    #[must_use]
    pub const fn id(&self) -> RelationalClosedWalkProfileId {
        self.id
    }

    /// Requested positive lengths.
    #[must_use]
    pub const fn plan(&self) -> &ClosedWalkQueryPlan {
        &self.plan
    }

    /// Operator whose traces are represented.
    #[must_use]
    pub const fn operator(&self) -> ClosedWalkOperator {
        self.operator
    }

    /// Complete or atomically skipped status.
    #[must_use]
    pub const fn status(&self) -> ClosedWalkAnalysisStatus {
        self.status
    }

    /// Conservative preflight estimate.
    #[must_use]
    pub const fn estimated_field_operations(&self) -> u64 {
        self.estimated_field_operations
    }

    /// Minimal recurrence degree observed in each lane.
    #[must_use]
    pub const fn recurrence_orders(&self) -> &[u32; K] {
        &self.recurrence_orders
    }

    /// One trace vector per requested length.
    #[must_use]
    pub fn traces(&self) -> &[[F; K]] {
        &self.traces
    }

    /// Finite-field traces are collision-prone fingerprints.
    #[must_use]
    pub const fn assurance(&self) -> SignatureAssurance {
        SignatureAssurance::Fingerprint
    }

    /// Serializes the complete identified profile.
    #[must_use]
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(WALK_MAGIC);
        bytes.extend_from_slice(&WALK_SCHEMA.to_be_bytes());
        bytes.extend_from_slice(self.id.as_bytes());
        bytes.extend_from_slice(self.plan.id().as_bytes());
        bytes.extend_from_slice(F::spec().field_id().as_bytes());
        bytes.push(self.operator as u8);
        bytes.push(self.status as u8);
        bytes.extend_from_slice(&self.graph_count.to_be_bytes());
        bytes.extend_from_slice(&self.vertex_count.to_be_bytes());
        bytes.extend_from_slice(&u64::try_from(K).unwrap_or(u64::MAX).to_be_bytes());
        for order in self.recurrence_orders {
            bytes.extend_from_slice(&order.to_be_bytes());
        }
        bytes.extend_from_slice(
            &u64::try_from(self.traces.len())
                .unwrap_or(u64::MAX)
                .to_be_bytes(),
        );
        for trace in &self.traces {
            for lane in trace {
                bytes.extend_from_slice(lane.to_canonical().as_ref());
            }
        }
        bytes
    }
}

fn derive_id<F, E, const K: usize>(
    plan: &ClosedWalkQueryPlan,
    operator: ClosedWalkOperator,
    encoder: &E,
) -> Result<RelationalClosedWalkProfileId, GraphError>
where
    F: Field + StaticField,
    E: StructuralLaneEncoder<F, K>,
{
    let mut hasher = Sha256::new();
    hasher.update(b"microfield/relational-closed-walk-profile/v1\0");
    hasher.update(F::spec().field_id().as_bytes());
    hasher.update(encoder.encoder_id().as_bytes());
    hasher.update([operator as u8]);
    hasher.update(
        u64::try_from(K)
            .map_err(|_| GraphError::GraphTooLarge)?
            .to_be_bytes(),
    );
    hasher.update(plan.id().as_bytes());
    Ok(RelationalClosedWalkProfileId(hasher.finalize().into()))
}

fn estimate_work<const K: usize>(
    operator_order: usize,
    transition_count: usize,
    queries: usize,
) -> Result<u64, GraphError> {
    let n = u64::try_from(operator_order).map_err(|_| GraphError::GraphTooLarge)?;
    let transitions = u64::try_from(transition_count).map_err(|_| GraphError::GraphTooLarge)?;
    let q = u64::try_from(queries).map_err(|_| GraphError::GraphTooLarge)?;
    let lanes = u64::try_from(K).map_err(|_| GraphError::GraphTooLarge)?;
    let seed_steps = n.checked_mul(2).ok_or(GraphError::GraphTooLarge)?;
    let propagation = lanes
        .checked_mul(seed_steps)
        .and_then(|value| value.checked_mul(n))
        .and_then(|value| value.checked_mul(transitions))
        .ok_or(GraphError::GraphTooLarge)?;
    let recurrence = lanes
        .checked_mul(n)
        .and_then(|value| value.checked_mul(n))
        .and_then(|value| value.checked_mul(q.checked_add(4)?))
        .ok_or(GraphError::GraphTooLarge)?;
    propagation
        .checked_add(recurrence)
        .ok_or(GraphError::GraphTooLarge)
}

fn build_adjacency_operator<F, E, const K: usize>(
    graph: &IncidenceGraph,
    encoder: &E,
) -> Result<Vec<(usize, usize, [F; K])>, GraphError>
where
    F: Field,
    E: StructuralLaneEncoder<F, K>,
{
    let mut cells = std::collections::BTreeMap::<(usize, usize), [F; K]>::new();
    for source_index in 0..graph.vertex_count() {
        let source = VertexId::new(source_index);
        let mut vertex_token = vec![1, graph.vertex_kind(source) as u8];
        append_framed(&mut vertex_token, graph.vertex_label(source))?;
        let diagonal = encoder.encode_lanes(&vertex_token)?;
        add_lanes(
            cells
                .entry((source_index, source_index))
                .or_insert([F::ZERO; K]),
            diagonal,
        );
        for incidence in graph.outgoing(source) {
            let descriptor = graph.relation(incidence.relation());
            let mut relation_token = vec![2];
            append_framed(&mut relation_token, descriptor.relation())?;
            append_framed(&mut relation_token, descriptor.role())?;
            let relation = encoder.encode_lanes(&relation_token)?;
            let scaled =
                core::array::from_fn(|lane| scale_by_u64(relation[lane], incidence.multiplicity()));
            add_lanes(
                cells
                    .entry((source_index, incidence.neighbor().index()))
                    .or_insert([F::ZERO; K]),
                scaled,
            );
        }
    }
    Ok(cells
        .into_iter()
        .filter_map(|((row, column), value)| {
            value
                .iter()
                .any(|lane| !lane.is_zero())
                .then_some((row, column, value))
        })
        .collect())
}

#[derive(Clone, Copy)]
struct ArcState {
    source: usize,
    target: usize,
    relation: crate::graph::RelationId,
    multiplicity: u64,
}

fn arc_states(graph: &IncidenceGraph) -> (Vec<ArcState>, Vec<usize>) {
    let mut states = Vec::with_capacity(graph.incidence_count());
    let mut row_offsets = Vec::with_capacity(graph.vertex_count() + 1);
    for source in 0..graph.vertex_count() {
        row_offsets.push(states.len());
        states.extend(
            graph
                .outgoing(VertexId::new(source))
                .iter()
                .map(|incidence| ArcState {
                    source,
                    target: incidence.neighbor().index(),
                    relation: incidence.relation(),
                    multiplicity: incidence.multiplicity(),
                }),
        );
    }
    row_offsets.push(states.len());
    (states, row_offsets)
}

fn non_backtracking_transition_count(graph: &IncidenceGraph) -> Result<usize, GraphError> {
    let (states, row_offsets) = arc_states(graph);
    let mut transitions = 0_usize;
    for state in &states {
        for next in &states[row_offsets[state.target]..row_offsets[state.target + 1]] {
            if next.target != state.source || state.source == state.target {
                transitions = transitions
                    .checked_add(1)
                    .ok_or(GraphError::GraphTooLarge)?;
            }
        }
    }
    Ok(transitions)
}

fn build_non_backtracking_operator<F, E, const K: usize>(
    graph: &IncidenceGraph,
    encoder: &E,
) -> Result<Vec<(usize, usize, [F; K])>, GraphError>
where
    F: Field,
    E: StructuralLaneEncoder<F, K>,
{
    let (states, row_offsets) = arc_states(graph);
    let mut operator = Vec::new();
    operator
        .try_reserve_exact(non_backtracking_transition_count(graph)?)
        .map_err(|_| GraphError::GraphTooLarge)?;
    for (current_index, current) in states.iter().enumerate() {
        let start = row_offsets[current.target];
        let end = row_offsets[current.target + 1];
        for (offset, next) in states[start..end].iter().copied().enumerate() {
            let next_index = start + offset;
            if next.target == current.source && current.source != current.target {
                continue;
            }
            let vertex = VertexId::new(current.target);
            let descriptor = graph.relation(next.relation);
            let mut token = vec![3, graph.vertex_kind(vertex) as u8];
            append_framed(&mut token, graph.vertex_label(vertex))?;
            append_framed(&mut token, descriptor.relation())?;
            append_framed(&mut token, descriptor.role())?;
            let encoded = encoder.encode_lanes(&token)?;
            let weight =
                core::array::from_fn(|lane| scale_by_u64(encoded[lane], next.multiplicity));
            if weight.iter().any(|lane| !lane.is_zero()) {
                operator.push((current_index, next_index, weight));
            }
        }
    }
    Ok(operator)
}

fn seed_traces<F: Field, const K: usize>(
    order: usize,
    operator: &[(usize, usize, [F; K])],
) -> Result<Vec<[F; K]>, GraphError> {
    let matrix_len = order.checked_mul(order).ok_or(GraphError::GraphTooLarge)?;
    let mut power = Vec::new();
    power
        .try_reserve_exact(matrix_len)
        .map_err(|_| GraphError::GraphTooLarge)?;
    power.resize(matrix_len, [F::ZERO; K]);
    for diagonal in 0..order {
        power[diagonal * order + diagonal] = [F::ONE; K];
    }
    let mut traces = Vec::new();
    traces
        .try_reserve_exact(order.saturating_mul(2).saturating_add(1))
        .map_err(|_| GraphError::GraphTooLarge)?;
    traces.push(matrix_trace(&power, order));
    for _ in 0..order.saturating_mul(2) {
        let mut next = Vec::new();
        next.try_reserve_exact(matrix_len)
            .map_err(|_| GraphError::GraphTooLarge)?;
        next.resize(matrix_len, [F::ZERO; K]);
        for &(row, inner, value) in operator {
            for column in 0..order {
                let right = power[inner * order + column];
                let output = &mut next[row * order + column];
                for lane in 0..K {
                    output[lane] = output[lane].add(value[lane].mul(right[lane]));
                }
            }
        }
        traces.push(matrix_trace(&next, order));
        power = next;
    }
    Ok(traces)
}

fn matrix_trace<F: Field, const K: usize>(matrix: &[[F; K]], order: usize) -> [F; K] {
    let mut trace = [F::ZERO; K];
    for diagonal in 0..order {
        add_lanes(&mut trace, matrix[diagonal * order + diagonal]);
    }
    trace
}

fn berlekamp_massey<F: Field + Invert>(sequence: &[F]) -> Result<Vec<F>, GraphError> {
    let mut current = vec![F::ONE];
    let mut previous = vec![F::ONE];
    let mut order = 0_usize;
    let mut shift = 1_usize;
    let mut previous_discrepancy = F::ONE;
    for index in 0..sequence.len() {
        let mut discrepancy = sequence[index];
        for offset in 1..=order {
            discrepancy = discrepancy.add(current[offset].mul(sequence[index - offset]));
        }
        if discrepancy.is_zero() {
            shift = shift.checked_add(1).ok_or(GraphError::GraphTooLarge)?;
            continue;
        }
        let inverse = previous_discrepancy
            .invert()
            .ok_or(GraphError::CanonicalizationInvariantViolation)?;
        let factor = discrepancy.mul(inverse);
        let saved = current.clone();
        let required = previous
            .len()
            .checked_add(shift)
            .ok_or(GraphError::GraphTooLarge)?;
        current.resize(current.len().max(required), F::ZERO);
        for (offset, coefficient) in previous.iter().copied().enumerate() {
            let target = offset + shift;
            current[target] = current[target].sub(factor.mul(coefficient));
        }
        if order.saturating_mul(2) <= index {
            order = index + 1 - order;
            previous = saved;
            previous_discrepancy = discrepancy;
            shift = 1;
        } else {
            shift = shift.checked_add(1).ok_or(GraphError::GraphTooLarge)?;
        }
    }
    current.resize(order.saturating_add(1), F::ZERO);
    Ok(current.into_iter().skip(1).map(Field::neg).collect())
}

fn linear_recurrence_term<F: Field>(seed: &[F], recurrence: &[F], index: u64) -> F {
    if let Ok(index) = usize::try_from(index) {
        if let Some(value) = seed.get(index) {
            return *value;
        }
    }
    let order = recurrence.len();
    if order == 0 {
        return F::ZERO;
    }
    let mut result = vec![F::ZERO; order];
    result[0] = F::ONE;
    let mut power = vec![F::ZERO; order];
    if order == 1 {
        power[0] = recurrence[0];
    } else {
        power[1] = F::ONE;
    }
    let mut exponent = index;
    while exponent != 0 {
        if exponent & 1 == 1 {
            result = combine_polynomials(&result, &power, recurrence);
        }
        exponent >>= 1;
        if exponent != 0 {
            power = combine_polynomials(&power, &power, recurrence);
        }
    }
    result
        .into_iter()
        .zip(seed.iter().copied())
        .fold(F::ZERO, |sum, (coefficient, value)| {
            sum.add(coefficient.mul(value))
        })
}

fn combine_polynomials<F: Field>(left: &[F], right: &[F], recurrence: &[F]) -> Vec<F> {
    let order = recurrence.len();
    let mut product = vec![F::ZERO; order.saturating_mul(2).saturating_sub(1)];
    for (left_index, left_value) in left.iter().copied().enumerate() {
        for (right_index, right_value) in right.iter().copied().enumerate() {
            let index = left_index + right_index;
            product[index] = product[index].add(left_value.mul(right_value));
        }
    }
    for degree in (order..product.len()).rev() {
        let coefficient = product[degree];
        for (offset, recurrence_value) in recurrence.iter().copied().enumerate() {
            let target = degree - 1 - offset;
            product[target] = product[target].add(coefficient.mul(recurrence_value));
        }
    }
    product.truncate(order);
    product
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

fn add_lanes<F: Field, const K: usize>(target: &mut [F; K], value: [F; K]) {
    for lane in 0..K {
        target[lane] = target[lane].add(value[lane]);
    }
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
