//! Non-authoritative G11 graph fingerprint channels.
//!
//! Every channel is invariant under vertex renumbering and exposes its own
//! composition law and assurance. None can publish an isomorphism result or
//! alter Microcanon bytes.

mod degree;
mod field_profile;
mod green;
mod matrix;
mod moments;
mod pair_refinement;
mod patterns;
mod walks;

pub(super) use degree::exact_degree_histograms_equal;
pub use degree::{
    DegreeHistogram, DegreeHistogramBin, DegreeHistogramProfile, DegreeHistogramProfileId,
};
#[cfg(feature = "dynamic-fields")]
pub use field_profile::DynamicGraphFieldProfile;
pub use field_profile::{GraphFieldChannel, GraphFieldSuitability, StaticGraphFieldProfile};
pub use green::{RelationalThetaProfile, RelationalThetaProfileId, ThetaAnalysisStatus};
pub use matrix::{MatrixAnalysisStatus, RelationalMatrixProfile, RelationalMatrixProfileId};
pub use moments::{CellMomentCell, CellMomentProfile, CellMomentProfileId};
pub use pair_refinement::{
    LocalPairRefinementProfile, LocalPairRefinementProfileId, PairRefinementStatus,
};
pub use patterns::{
    ConnectedPatternCount, ConnectedPatternProfile, LoopPatternCatalog, LoopPatternCatalogId,
    PatternAnalysisStatus, PatternFieldFingerprint, PatternFingerprintId,
    PatternProductFingerprint,
};
pub use walks::{
    ClosedWalkAnalysisStatus, ClosedWalkOperator, ClosedWalkQueryPlan, ClosedWalkQueryPlanId,
    RelationalClosedWalkProfile, RelationalClosedWalkProfileId,
};
