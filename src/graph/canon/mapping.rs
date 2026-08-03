//! Exact verification of candidate graph isomorphisms.

use super::super::{GraphError, IncidenceGraph, VertexId};

/// A bijection rechecked against the complete normalized graph model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedGraphMapping {
    left_to_right: Vec<VertexId>,
    right_to_left: Vec<VertexId>,
}

impl VerifiedGraphMapping {
    /// Validates and owns a candidate mapping from `left` to `right`.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::InvalidGraphMapping`] unless the mapping is a
    /// complete bijection preserving vertex kinds/labels and every directed
    /// relation, role and multiplicity.
    pub fn verify(
        left: &IncidenceGraph,
        right: &IncidenceGraph,
        left_to_right: &[VertexId],
    ) -> Result<Self, GraphError> {
        if left.vertex_count() != right.vertex_count()
            || left.incidence_count() != right.incidence_count()
            || left.total_multiplicity() != right.total_multiplicity()
            || left_to_right.len() != left.vertex_count()
        {
            return Err(GraphError::InvalidGraphMapping);
        }

        let mut right_to_left = vec![VertexId::new(0); right.vertex_count()];
        let mut seen = vec![false; right.vertex_count()];
        for (left_index, right_vertex) in left_to_right.iter().copied().enumerate() {
            if right_vertex.index() >= right.vertex_count() || seen[right_vertex.index()] {
                return Err(GraphError::InvalidGraphMapping);
            }
            seen[right_vertex.index()] = true;
            right_to_left[right_vertex.index()] = VertexId::new(left_index);
            let left_vertex = VertexId::new(left_index);
            if left.vertex_kind(left_vertex) != right.vertex_kind(right_vertex)
                || left.vertex_label(left_vertex) != right.vertex_label(right_vertex)
            {
                return Err(GraphError::InvalidGraphMapping);
            }
        }

        for left_source in 0..left.vertex_count() {
            let right_source = left_to_right[left_source];
            let mut mapped = Vec::with_capacity(left.outgoing(VertexId::new(left_source)).len());
            for incidence in left.outgoing(VertexId::new(left_source)) {
                let descriptor = left.relation(incidence.relation());
                mapped.push(ArcRecord {
                    target: left_to_right[incidence.neighbor().index()].index(),
                    relation: descriptor.relation(),
                    role: descriptor.role(),
                    multiplicity: incidence.multiplicity(),
                });
            }
            mapped.sort_unstable();

            let mut expected = Vec::with_capacity(right.outgoing(right_source).len());
            for incidence in right.outgoing(right_source) {
                let descriptor = right.relation(incidence.relation());
                expected.push(ArcRecord {
                    target: incidence.neighbor().index(),
                    relation: descriptor.relation(),
                    role: descriptor.role(),
                    multiplicity: incidence.multiplicity(),
                });
            }
            expected.sort_unstable();
            if mapped != expected {
                return Err(GraphError::InvalidGraphMapping);
            }
        }

        Ok(Self {
            left_to_right: left_to_right.to_vec(),
            right_to_left,
        })
    }

    /// Mapping indexed by a vertex of the left graph.
    #[must_use]
    pub fn left_to_right(&self) -> &[VertexId] {
        &self.left_to_right
    }

    /// Inverse mapping indexed by a vertex of the right graph.
    #[must_use]
    pub fn right_to_left(&self) -> &[VertexId] {
        &self.right_to_left
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ArcRecord<'a> {
    target: usize,
    relation: &'a [u8],
    role: &'a [u8],
    multiplicity: u64,
}
