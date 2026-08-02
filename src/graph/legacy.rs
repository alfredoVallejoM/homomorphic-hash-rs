//! Explicit migration adapter for the historical bipartite topology API.

use crate::engine::canonizer::TopologyProvider;

use super::{GraphError, HyperedgeIncidence, IncidenceGraph, IncidenceGraphBuilder, VertexId};

/// Converts the historical variable/clause model into an exact incidence graph.
///
/// Each clause becomes one auxiliary hyperedge vertex, so neither clause
/// identity nor repeated membership is squashed into a clique. Historical
/// `initial_state` values remain compatibility labels; applications should use
/// [`IncidenceGraphBuilder`](super::IncidenceGraphBuilder) directly whenever
/// exact source labels are available.
///
/// # Errors
///
/// Rejects out-of-range variable indices and multiplicity overflow without
/// publishing a partial normalized graph.
pub fn from_legacy_topology<T: TopologyProvider + ?Sized>(
    provider: &T,
) -> Result<IncidenceGraph, GraphError> {
    let variable_count = provider.num_variables();
    let mut builder = IncidenceGraphBuilder::new();
    let variables: Vec<VertexId> = (0..variable_count)
        .map(|index| {
            let mut label = Vec::with_capacity(33);
            if let Some(seed) = provider.initial_state(index) {
                label.push(1);
                label.extend_from_slice(&seed.to_canonical_bytes());
            } else {
                label.push(0);
            }
            builder.add_vertex(label)
        })
        .collect();

    for clause in 0..provider.num_clauses() {
        let members = provider.variables_in_clause(clause);
        let mut incidences = Vec::with_capacity(members.len());
        for member in members {
            let vertex = variables
                .get(member)
                .copied()
                .ok_or(GraphError::InvalidVertex {
                    index: member,
                    vertex_count: variable_count,
                })?;
            incidences.push(HyperedgeIncidence::new(vertex, Vec::new()));
        }
        builder.add_hyperedge(Vec::new(), &incidences)?;
    }
    builder.build()
}
