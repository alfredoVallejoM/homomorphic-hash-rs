pub mod algebra;
pub mod engine;
pub mod topology;

// Re-exports publicos para facilitar el consumo de la libreria
pub use algebra::galois_256::GaloisSignature256;
pub use algebra::traits::FiniteField;

pub use topology::bloom_l1::{TopoBloomMask, TopologicalMask};
pub use topology::multiset::MultisetAggregator;
pub use topology::sequence::SequenceAggregator;
pub use topology::symmetric_difference::SymmetricDifferenceAggregator;
pub use topology::traits::HomomorphicAggregator;

pub use engine::canonizer::{CanonicalNode, CellularGaloisCanonizer, TopologyProvider};
pub use engine::hasher::TopoHasher;
pub use engine::spectral_f251::SpectralEngineF251;
pub mod domains;
pub mod harness;
