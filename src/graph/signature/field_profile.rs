//! Capability and characteristic policy for graph fingerprint fields.

use microfield::{FieldId, StaticField};

/// Algebraic graph channel whose suitability depends on field characteristic.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GraphFieldChannel {
    /// Products over exact per-vertex degree tuples.
    DegreeMultisets,
    /// Sums of exact counts or power moments.
    AdditiveCounts,
    /// Products that retain multiplicity through exponentiation.
    MultiplicativePatterns,
    /// Traces and characteristic-polynomial evaluations.
    RelationalMatrix,
    /// Symmetric theta contractions.
    ThetaContractions,
    /// Long closed-walk traces recovered from a linear recurrence.
    LongClosedWalks,
    /// Long non-backtracking closed walks over directed incidence states.
    NonBacktrackingWalks,
}

/// Evidence-backed recommendation for one field/channel combination.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GraphFieldSuitability {
    /// No characteristic-specific degeneration is expected.
    Preferred,
    /// Algebraically valid, but small characteristic can alias integer counts.
    CompatibleWithAliasing,
    /// Known to collapse badly on symmetric undirected graphs.
    AvoidForSymmetricGraphs,
}

/// Immutable graph-analysis view of one generated finite field.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StaticGraphFieldProfile {
    field_id: FieldId,
    characteristic_is_two: bool,
    extension_degree: u32,
}

impl StaticGraphFieldProfile {
    /// Derives a policy from certified static field metadata.
    #[must_use]
    pub fn for_field<F: StaticField>() -> Self {
        let spec = F::spec();
        Self {
            field_id: spec.field_id(),
            characteristic_is_two: spec.characteristic() == 2,
            extension_degree: spec.degree(),
        }
    }

    /// Stable mathematical presentation identity.
    #[must_use]
    pub const fn field_id(self) -> FieldId {
        self.field_id
    }

    /// Whether integer additions are reduced modulo two.
    #[must_use]
    pub const fn characteristic_is_two(self) -> bool {
        self.characteristic_is_two
    }

    /// Extension degree declared by the generated presentation.
    #[must_use]
    pub const fn extension_degree(self) -> u32 {
        self.extension_degree
    }

    /// Recommends use without pretending all fields discriminate equally.
    #[must_use]
    pub const fn suitability(self, channel: GraphFieldChannel) -> GraphFieldSuitability {
        suitability(self.characteristic_is_two, channel)
    }
}

/// Runtime-field equivalent used before exporting/generating a hot-path type.
#[cfg(feature = "dynamic-fields")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DynamicGraphFieldProfile {
    field_id: FieldId,
    characteristic_is_two: bool,
    extension_degree: u32,
}

#[cfg(feature = "dynamic-fields")]
impl DynamicGraphFieldProfile {
    /// Derives graph-channel policy from a completely validated runtime field.
    #[must_use]
    pub fn for_field(field: &microfield::DynField) -> Self {
        Self {
            field_id: field.field_id(),
            characteristic_is_two: field.characteristic_is_two(),
            extension_degree: field.extension_degree(),
        }
    }

    /// Stable mathematical presentation identity.
    #[must_use]
    pub const fn field_id(&self) -> FieldId {
        self.field_id
    }

    /// Whether graph additions reduce integer multiplicities modulo two.
    #[must_use]
    pub const fn characteristic_is_two(&self) -> bool {
        self.characteristic_is_two
    }

    /// Runtime extension degree.
    #[must_use]
    pub const fn extension_degree(&self) -> u32 {
        self.extension_degree
    }

    /// Uses the same characteristic policy as generated static fields.
    #[must_use]
    pub const fn suitability(&self, channel: GraphFieldChannel) -> GraphFieldSuitability {
        suitability(self.characteristic_is_two, channel)
    }
}

const fn suitability(
    characteristic_is_two: bool,
    channel: GraphFieldChannel,
) -> GraphFieldSuitability {
    if !characteristic_is_two {
        return GraphFieldSuitability::Preferred;
    }
    match channel {
        GraphFieldChannel::DegreeMultisets | GraphFieldChannel::MultiplicativePatterns => {
            GraphFieldSuitability::Preferred
        }
        GraphFieldChannel::AdditiveCounts
        | GraphFieldChannel::LongClosedWalks
        | GraphFieldChannel::NonBacktrackingWalks => GraphFieldSuitability::CompatibleWithAliasing,
        GraphFieldChannel::RelationalMatrix | GraphFieldChannel::ThetaContractions => {
            GraphFieldSuitability::AvoidForSymmetricGraphs
        }
    }
}
