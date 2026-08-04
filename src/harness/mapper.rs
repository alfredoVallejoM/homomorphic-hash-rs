use crate::algebra::galois_256::GaloisSignature256;
use crate::engine::canonizer::TopologyProvider;

/// Universal interface for mapping real-world data into homomorphic topology.
pub trait DomainMapper {
    type RawInput;

    /// Translates raw domain data (e.g., SMILES string, DIMACS file) into a mathematical graph.
    /// Returns the Bipartite Topology and the Initial Seed States for the variables.
    fn map_to_topology(
        input: &Self::RawInput,
    ) -> (
        Box<dyn TopologyProvider + Send + Sync>,
        Vec<GaloisSignature256>,
    );
}
