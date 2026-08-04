//! Bounded set reconciliation over the maintained Fp251 field.
//!
//! Version 1 accepts sorted unique set members only. Multiset multiplicities
//! are deliberately rejected rather than silently collapsed.

use core::fmt;

use microfield::{CanonicalEncoding, Field, Fp251V1, Invert, Pow};
use sha2::{Digest as _, Sha256};

const SKETCH_MAGIC: &[u8; 4] = b"MFRS";
const SKETCH_SCHEMA: u16 = 1;
const SKETCH_HEADER_BYTES: usize = 56;

/// Stable identity of universe, evaluation points and decoding ceilings.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct ReconciliationProfileId([u8; 32]);

impl ReconciliationProfileId {
    /// Borrows the canonical bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for ReconciliationProfileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ReconciliationProfileId(")?;
        for byte in self.as_bytes() {
            write!(formatter, "{byte:02x}")?;
        }
        formatter.write_str(")")
    }
}

/// Explicit algorithm and resource limits for the maintained decoder.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReconciliationLimits {
    /// Exclusive upper bound for every set member.
    pub universe: u16,
    /// Maximum recoverable symmetric difference.
    pub max_difference: usize,
    /// Maximum polynomial/system degree admitted by policy.
    pub max_degree: usize,
    /// Maximum temporary matrix bytes estimated before decoding.
    pub max_memory_bytes: usize,
}

impl ReconciliationLimits {
    /// Conservative defaults for an in-process bounded protocol.
    #[must_use]
    pub const fn new(
        universe: u16,
        max_difference: usize,
        max_degree: usize,
        max_memory_bytes: usize,
    ) -> Self {
        Self {
            universe,
            max_difference,
            max_degree,
            max_memory_bytes,
        }
    }
}

/// Typed reconciliation failure with no partial candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ReconciliationError {
    /// Universe, degree or memory policy is inconsistent with Fp251 v1.
    InvalidLimits(&'static str),
    /// Inputs are not strictly sorted unique members inside the universe.
    InvalidSet,
    /// Sketches were produced by different profiles.
    ProfileMismatch,
    /// No exact difference was found within the declared bound.
    DifferenceExceedsBound,
    /// A denominator evaluation unexpectedly became zero.
    DenominatorRoot,
    /// A sketch envelope is malformed or non-canonical.
    InvalidWire(&'static str),
    /// Allocation failed before a candidate was returned.
    AllocationFailed,
}

impl fmt::Display for ReconciliationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimits(reason) => {
                write!(formatter, "invalid reconciliation limits: {reason}")
            }
            Self::InvalidSet => formatter
                .write_str("reconciliation set must be sorted, unique and inside the universe"),
            Self::ProfileMismatch => formatter.write_str("reconciliation profile mismatch"),
            Self::DifferenceExceedsBound => formatter
                .write_str("no reconciliation solution inside the declared difference bound"),
            Self::DenominatorRoot => {
                formatter.write_str("reconciliation evaluation point is a denominator root")
            }
            Self::InvalidWire(reason) => {
                write!(formatter, "invalid reconciliation sketch: {reason}")
            }
            Self::AllocationFailed => formatter.write_str("reconciliation allocation failed"),
        }
    }
}

impl std::error::Error for ReconciliationError {}

/// Characteristic-polynomial evaluations sufficient for one bounded profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetReconciliationSketch {
    profile_id: ReconciliationProfileId,
    cardinality: u64,
    evaluations: Vec<Fp251V1>,
}

impl SetReconciliationSketch {
    /// Profile under which this sketch was generated.
    #[must_use]
    pub const fn profile_id(&self) -> ReconciliationProfileId {
        self.profile_id
    }

    /// Exact source cardinality before field evaluation.
    #[must_use]
    pub const fn cardinality(&self) -> u64 {
        self.cardinality
    }

    /// Canonical `MFRS` schema 1 representation.
    #[must_use]
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(SKETCH_HEADER_BYTES + self.evaluations.len());
        bytes.extend_from_slice(SKETCH_MAGIC);
        bytes.extend_from_slice(&SKETCH_SCHEMA.to_le_bytes());
        bytes.extend_from_slice(&[0, 0]);
        bytes.extend_from_slice(self.profile_id.as_bytes());
        bytes.extend_from_slice(&self.cardinality.to_le_bytes());
        bytes.extend_from_slice(&(self.evaluations.len() as u64).to_le_bytes());
        for evaluation in &self.evaluations {
            bytes.push(evaluation.to_canonical().as_ref()[0]);
        }
        bytes
    }
}

/// Exact oriented difference recovered under the bounded-distance contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveredSetDifference {
    only_left: Vec<u16>,
    only_right: Vec<u16>,
}

impl RecoveredSetDifference {
    /// Members present only in the left/source sketch.
    #[must_use]
    pub fn only_left(&self) -> &[u16] {
        &self.only_left
    }

    /// Members present only in the receiver/right set.
    #[must_use]
    pub fn only_right(&self) -> &[u16] {
        &self.only_right
    }

    /// Exact recovered symmetric-difference cardinality.
    #[must_use]
    pub fn distance(&self) -> usize {
        self.only_left.len() + self.only_right.len()
    }
}

/// Maintained bounded set reconciler for one frozen Fp251 profile.
#[derive(Clone, Debug)]
pub struct BoundedSetReconciler {
    limits: ReconciliationLimits,
    profile_id: ReconciliationProfileId,
    points: Vec<Fp251V1>,
}

impl BoundedSetReconciler {
    /// Validates limits and fixes all evaluation points.
    pub fn new(limits: ReconciliationLimits) -> Result<Self, ReconciliationError> {
        validate_limits(limits)?;
        let mut points = Vec::new();
        points
            .try_reserve_exact(limits.max_difference)
            .map_err(|_| ReconciliationError::AllocationFailed)?;
        for index in 0..limits.max_difference {
            points.push(fp(u64::from(limits.universe) + 17 + index as u64));
        }
        let mut hasher = Sha256::new();
        hasher.update(b"microfield-set-reconciliation-fp251-v1\0");
        hasher.update(limits.universe.to_le_bytes());
        hasher.update((limits.max_difference as u64).to_le_bytes());
        hasher.update((limits.max_degree as u64).to_le_bytes());
        Ok(Self {
            limits,
            profile_id: ReconciliationProfileId(hasher.finalize().into()),
            points,
        })
    }

    /// Frozen profile identity.
    #[must_use]
    pub const fn profile_id(&self) -> ReconciliationProfileId {
        self.profile_id
    }

    /// Effective limits.
    #[must_use]
    pub const fn limits(&self) -> ReconciliationLimits {
        self.limits
    }

    /// Builds a sketch from strictly sorted unique set members.
    pub fn sketch(&self, values: &[u16]) -> Result<SetReconciliationSketch, ReconciliationError> {
        validate_set(values, self.limits.universe)?;
        Ok(SetReconciliationSketch {
            profile_id: self.profile_id,
            cardinality: values.len() as u64,
            evaluations: self
                .points
                .iter()
                .map(|&point| {
                    values.iter().fold(Fp251V1::ONE, |product, &value| {
                        product.mul(point.sub(fp(u64::from(value))))
                    })
                })
                .collect(),
        })
    }

    /// Restores a sketch under this exact profile.
    pub fn sketch_from_canonical_bytes(
        &self,
        bytes: &[u8],
    ) -> Result<SetReconciliationSketch, ReconciliationError> {
        let expected = SKETCH_HEADER_BYTES + self.points.len();
        if bytes.len() != expected || &bytes[..4] != SKETCH_MAGIC {
            return Err(ReconciliationError::InvalidWire("length or magic"));
        }
        if u16::from_le_bytes([bytes[4], bytes[5]]) != SKETCH_SCHEMA || bytes[6..8] != [0, 0] {
            return Err(ReconciliationError::InvalidWire("schema or reserved"));
        }
        if &bytes[8..40] != self.profile_id.as_bytes() {
            return Err(ReconciliationError::ProfileMismatch);
        }
        let cardinality = u64::from_le_bytes(bytes[40..48].try_into().expect("cardinality range"));
        if cardinality > u64::from(self.limits.universe) {
            return Err(ReconciliationError::InvalidWire(
                "cardinality exceeds universe",
            ));
        }
        let count = u64::from_le_bytes(bytes[48..56].try_into().expect("evaluation count range"));
        if count != self.points.len() as u64 {
            return Err(ReconciliationError::InvalidWire("evaluation count"));
        }
        let evaluations = bytes[56..]
            .iter()
            .map(|&value| {
                Fp251V1::from_canonical_slice(&[value])
                    .map_err(|_| ReconciliationError::InvalidWire("non-canonical evaluation"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(SetReconciliationSketch {
            profile_id: self.profile_id,
            cardinality,
            evaluations,
        })
    }

    /// Recovers the oriented difference using the exact right/receiver set.
    ///
    /// The receiver set validates membership/removal and the reconstructed
    /// source sketch. No partial roots escape when the bound is exceeded.
    pub fn reconcile(
        &self,
        left: &SetReconciliationSketch,
        right: &SetReconciliationSketch,
        receiver_set: &[u16],
    ) -> Result<RecoveredSetDifference, ReconciliationError> {
        if left.profile_id != self.profile_id || right.profile_id != self.profile_id {
            return Err(ReconciliationError::ProfileMismatch);
        }
        validate_set(receiver_set, self.limits.universe)?;
        if right != &self.sketch(receiver_set)? {
            return Err(ReconciliationError::InvalidSet);
        }
        if left.cardinality == right.cardinality && left.evaluations == right.evaluations {
            return Ok(RecoveredSetDifference {
                only_left: Vec::new(),
                only_right: Vec::new(),
            });
        }
        let cardinality_delta = i128::from(left.cardinality) - i128::from(right.cardinality);
        let ratios = left
            .evaluations
            .iter()
            .zip(&right.evaluations)
            .map(|(&numerator, &denominator)| {
                denominator
                    .invert()
                    .map(|inverse| numerator.mul(inverse))
                    .ok_or(ReconciliationError::DenominatorRoot)
            })
            .collect::<Result<Vec<_>, _>>()?;

        for difference in 1..=self.limits.max_difference {
            for positive_degree in 0..=difference {
                let negative_degree = difference - positive_degree;
                if positive_degree as i128 - negative_degree as i128 != cardinality_delta {
                    continue;
                }
                let Some(coefficients) = solve_candidate(
                    &self.points[..difference],
                    &ratios[..difference],
                    positive_degree,
                    negative_degree,
                ) else {
                    continue;
                };
                let numerator = monic_polynomial(&coefficients[..positive_degree]);
                let denominator = monic_polynomial(&coefficients[positive_degree..]);
                let only_left = roots_in_universe(&numerator, self.limits.universe);
                let only_right = roots_in_universe(&denominator, self.limits.universe);
                if only_left.len() != positive_degree
                    || only_right.len() != negative_degree
                    || only_left.iter().any(|value| receiver_set.contains(value))
                    || only_right.iter().any(|value| !receiver_set.contains(value))
                    || !sketch_equation_holds(left, right, &numerator, &denominator, &self.points)
                {
                    continue;
                }
                let mut reconstructed: Vec<_> = receiver_set
                    .iter()
                    .copied()
                    .filter(|value| !only_right.contains(value))
                    .chain(only_left.iter().copied())
                    .collect();
                reconstructed.sort_unstable();
                if self.sketch(&reconstructed)?.evaluations != left.evaluations {
                    continue;
                }
                return Ok(RecoveredSetDifference {
                    only_left,
                    only_right,
                });
            }
        }
        Err(ReconciliationError::DifferenceExceedsBound)
    }
}

fn validate_limits(limits: ReconciliationLimits) -> Result<(), ReconciliationError> {
    if limits.universe == 0 || limits.universe > 200 {
        return Err(ReconciliationError::InvalidLimits(
            "universe must be 1..=200",
        ));
    }
    if limits.max_difference == 0 || limits.max_difference > limits.max_degree {
        return Err(ReconciliationError::InvalidLimits("difference/degree"));
    }
    if usize::from(limits.universe) + 17 + limits.max_difference >= 251 {
        return Err(ReconciliationError::InvalidLimits(
            "evaluation points exceed Fp251",
        ));
    }
    let matrix_bytes = limits
        .max_difference
        .checked_mul(limits.max_difference + 1)
        .and_then(|entries| entries.checked_mul(8))
        .ok_or(ReconciliationError::InvalidLimits("matrix size overflow"))?;
    if matrix_bytes > limits.max_memory_bytes {
        return Err(ReconciliationError::InvalidLimits("matrix memory"));
    }
    Ok(())
}

fn validate_set(values: &[u16], universe: u16) -> Result<(), ReconciliationError> {
    if values.iter().any(|&value| value >= universe)
        || values.windows(2).any(|pair| pair[0] >= pair[1])
    {
        Err(ReconciliationError::InvalidSet)
    } else {
        Ok(())
    }
}

fn solve_candidate(
    points: &[Fp251V1],
    ratios: &[Fp251V1],
    positive_degree: usize,
    negative_degree: usize,
) -> Option<Vec<Fp251V1>> {
    let difference = positive_degree + negative_degree;
    let mut matrix = Vec::with_capacity(difference);
    for (&point, &ratio) in points.iter().zip(ratios) {
        let mut row = Vec::with_capacity(difference + 1);
        for power in 0..positive_degree {
            row.push(point.pow(&[power as u64]));
        }
        for power in 0..negative_degree {
            row.push(ratio.neg().mul(point.pow(&[power as u64])));
        }
        row.push(
            ratio
                .mul(point.pow(&[negative_degree as u64]))
                .sub(point.pow(&[positive_degree as u64])),
        );
        matrix.push(row);
    }
    solve_unique(matrix)
}

fn solve_unique(mut matrix: Vec<Vec<Fp251V1>>) -> Option<Vec<Fp251V1>> {
    let size = matrix.len();
    if matrix.iter().any(|row| row.len() != size + 1) {
        return None;
    }
    for column in 0..size {
        let pivot = (column..size).find(|&row| !matrix[row][column].is_zero())?;
        matrix.swap(column, pivot);
        let inverse = matrix[column][column].invert()?;
        for value in &mut matrix[column][column..=size] {
            *value = value.mul(inverse);
        }
        let pivot_row = matrix[column].clone();
        for (row_index, row) in matrix.iter_mut().enumerate() {
            if row_index == column {
                continue;
            }
            let factor = row[column];
            for entry in column..=size {
                row[entry] = row[entry].sub(factor.mul(pivot_row[entry]));
            }
        }
    }
    Some(matrix.into_iter().map(|row| row[size]).collect())
}

fn monic_polynomial(lower: &[Fp251V1]) -> Vec<Fp251V1> {
    let mut coefficients = lower.to_vec();
    coefficients.push(Fp251V1::ONE);
    coefficients
}

fn evaluate_polynomial(coefficients: &[Fp251V1], point: Fp251V1) -> Fp251V1 {
    coefficients
        .iter()
        .rev()
        .fold(Fp251V1::ZERO, |value, &coefficient| {
            value.mul(point).add(coefficient)
        })
}

fn roots_in_universe(coefficients: &[Fp251V1], universe: u16) -> Vec<u16> {
    (0..universe)
        .filter(|&value| evaluate_polynomial(coefficients, fp(u64::from(value))).is_zero())
        .collect()
}

fn sketch_equation_holds(
    left: &SetReconciliationSketch,
    right: &SetReconciliationSketch,
    numerator: &[Fp251V1],
    denominator: &[Fp251V1],
    points: &[Fp251V1],
) -> bool {
    points.iter().enumerate().all(|(index, &point)| {
        left.evaluations[index].mul(evaluate_polynomial(denominator, point))
            == right.evaluations[index].mul(evaluate_polynomial(numerator, point))
    })
}

fn fp(value: u64) -> Fp251V1 {
    Fp251V1::from_u64_mod(value)
}
