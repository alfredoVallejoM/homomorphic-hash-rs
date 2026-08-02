//! Unambiguous Horner plans for the two common batch orientations.

use core::{fmt, marker::PhantomData};

use crate::{__private::PortableField, BackendId, Engine, FieldId, StaticField};

use super::{
    AlgorithmFamily, AlgorithmId, AllocationBehavior, BatchPlan, OperationKind, WorkspaceLayout,
};

/// Physical order of a rectangular polynomial coefficient matrix.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CoefficientLayout {
    /// All coefficients of polynomial zero, then polynomial one, and so on.
    PolynomialMajor,
    /// Coefficient zero of every polynomial, then coefficient one, and so on.
    CoefficientMajor,
}

/// Failure while planning or executing Horner evaluation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum HornerError {
    /// A polynomial must contain at least one coefficient.
    EmptyCoefficientShape,
    /// Multiplying the rectangular shape overflowed `usize`.
    SizeOverflow,
    /// A supplied slice does not match the shape fixed by the plan.
    LengthMismatch {
        /// Name of the incompatible slice.
        argument: &'static str,
        /// Length fixed by the plan.
        expected: usize,
        /// Supplied length.
        actual: usize,
    },
    /// The plan was created for another selected backend.
    BackendMismatch {
        /// Backend selected by the executing engine.
        expected: BackendId,
        /// Backend recorded by the plan.
        actual: BackendId,
    },
}

impl fmt::Display for HornerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCoefficientShape => {
                formatter.write_str("Horner evaluation requires at least one coefficient")
            }
            Self::SizeOverflow => formatter.write_str("Horner shape overflow"),
            Self::LengthMismatch {
                argument,
                expected,
                actual,
            } => write!(
                formatter,
                "Horner `{argument}` length mismatch: expected={expected}, actual={actual}"
            ),
            Self::BackendMismatch { expected, actual } => write!(
                formatter,
                "Horner backend mismatch: engine={expected:?}, plan={actual:?}"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for HornerError {}

/// Plan for evaluating one polynomial at many points.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ManyPointsHornerPlan<F: StaticField> {
    point_count: usize,
    coefficient_count: usize,
    backend: BackendId,
    field_id: FieldId,
    field: PhantomData<F>,
}

impl<F> ManyPointsHornerPlan<F>
where
    F: PortableField + StaticField,
{
    /// Creates a reusable plan for coefficients stored in ascending degree.
    ///
    /// # Errors
    ///
    /// Returns [`HornerError::EmptyCoefficientShape`] for zero coefficients.
    pub fn new(
        engine: &Engine<F>,
        point_count: usize,
        coefficient_count: usize,
    ) -> Result<Self, HornerError> {
        if coefficient_count == 0 {
            return Err(HornerError::EmptyCoefficientShape);
        }
        Ok(Self {
            point_count,
            coefficient_count,
            backend: engine.backend_id(),
            field_id: F::spec().field_id(),
            field: PhantomData,
        })
    }

    /// Returns the number of evaluation points.
    #[must_use]
    pub const fn point_count(&self) -> usize {
        self.point_count
    }

    /// Returns the number of coefficients, including leading zeros.
    #[must_use]
    pub const fn coefficient_count(&self) -> usize {
        self.coefficient_count
    }

    /// Evaluates one polynomial at every supplied point.
    ///
    /// Coefficients use ascending degree order: `coefficients[i]` multiplies
    /// `x^i`.
    ///
    /// # Errors
    ///
    /// Returns a typed error without modifying `out` when any length or the
    /// selected backend differs from this plan.
    pub fn execute(
        &self,
        engine: &Engine<F>,
        out: &mut [F],
        coefficients: &[F],
        points: &[F],
    ) -> Result<(), HornerError> {
        self.validate(engine, out.len(), coefficients.len(), points.len())?;

        out.fill(coefficients[self.coefficient_count - 1]);
        for coefficient in coefficients[..self.coefficient_count - 1].iter().rev() {
            // Lengths were validated above; this kernel call cannot fail.
            let result = engine.mul_assign(out, points);
            debug_assert!(result.is_ok());
            for value in &mut *out {
                *value = value.add(*coefficient);
            }
        }
        Ok(())
    }

    fn validate(
        &self,
        engine: &Engine<F>,
        out: usize,
        coefficients: usize,
        points: usize,
    ) -> Result<(), HornerError> {
        if engine.backend_id() != self.backend {
            return Err(HornerError::BackendMismatch {
                expected: engine.backend_id(),
                actual: self.backend,
            });
        }
        validate_len("out", self.point_count, out)?;
        validate_len("points", self.point_count, points)?;
        validate_len("coefficients", self.coefficient_count, coefficients)
    }
}

impl<F: StaticField> BatchPlan<F> for ManyPointsHornerPlan<F> {
    fn algorithm_id(&self) -> AlgorithmId {
        AlgorithmId::new(OperationKind::HornerManyPoints, AlgorithmFamily::Horner, 1)
    }

    fn logical_len(&self) -> usize {
        self.point_count
    }

    fn backend_id(&self) -> BackendId {
        self.backend
    }

    fn field_id(&self) -> FieldId {
        self.field_id
    }

    fn workspace_layout(&self) -> WorkspaceLayout {
        WorkspaceLayout::new(0, 0, 1, false, AllocationBehavior::None)
    }
}

/// Plan for evaluating many equally shaped polynomials at one point.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ManyPolynomialsHornerPlan<F: StaticField> {
    polynomial_count: usize,
    coefficient_count: usize,
    coefficient_storage_len: usize,
    layout: CoefficientLayout,
    backend: BackendId,
    field_id: FieldId,
    field: PhantomData<F>,
}

impl<F> ManyPolynomialsHornerPlan<F>
where
    F: PortableField + StaticField,
{
    /// Creates a reusable plan with an explicit coefficient layout.
    ///
    /// # Errors
    ///
    /// Returns a typed error for an empty coefficient shape or size overflow.
    pub fn new(
        engine: &Engine<F>,
        polynomial_count: usize,
        coefficient_count: usize,
        layout: CoefficientLayout,
    ) -> Result<Self, HornerError> {
        if coefficient_count == 0 {
            return Err(HornerError::EmptyCoefficientShape);
        }
        let coefficient_storage_len = polynomial_count
            .checked_mul(coefficient_count)
            .ok_or(HornerError::SizeOverflow)?;
        Ok(Self {
            polynomial_count,
            coefficient_count,
            coefficient_storage_len,
            layout,
            backend: engine.backend_id(),
            field_id: F::spec().field_id(),
            field: PhantomData,
        })
    }

    /// Returns the number of polynomials.
    #[must_use]
    pub const fn polynomial_count(&self) -> usize {
        self.polynomial_count
    }

    /// Returns the number of coefficients per polynomial.
    #[must_use]
    pub const fn coefficient_count(&self) -> usize {
        self.coefficient_count
    }

    /// Returns the fixed physical coefficient layout.
    #[must_use]
    pub const fn layout(&self) -> CoefficientLayout {
        self.layout
    }

    /// Evaluates every polynomial at `point`.
    ///
    /// # Errors
    ///
    /// Returns a typed error without modifying `out` when any length or the
    /// selected backend differs from this plan.
    pub fn execute(
        &self,
        engine: &Engine<F>,
        out: &mut [F],
        coefficients: &[F],
        point: F,
    ) -> Result<(), HornerError> {
        self.validate(engine, out.len(), coefficients.len())?;

        for (polynomial, output) in out.iter_mut().enumerate() {
            let mut value = self.coefficient(coefficients, polynomial, self.coefficient_count - 1);
            for degree in (0..self.coefficient_count - 1).rev() {
                value = value
                    .mul(point)
                    .add(self.coefficient(coefficients, polynomial, degree));
            }
            *output = value;
        }
        Ok(())
    }

    fn coefficient(&self, coefficients: &[F], polynomial: usize, degree: usize) -> F {
        match self.layout {
            CoefficientLayout::PolynomialMajor => {
                coefficients[polynomial * self.coefficient_count + degree]
            }
            CoefficientLayout::CoefficientMajor => {
                coefficients[degree * self.polynomial_count + polynomial]
            }
        }
    }

    fn validate(
        &self,
        engine: &Engine<F>,
        out: usize,
        coefficients: usize,
    ) -> Result<(), HornerError> {
        if engine.backend_id() != self.backend {
            return Err(HornerError::BackendMismatch {
                expected: engine.backend_id(),
                actual: self.backend,
            });
        }
        validate_len("out", self.polynomial_count, out)?;
        validate_len("coefficients", self.coefficient_storage_len, coefficients)
    }
}

impl<F: StaticField> BatchPlan<F> for ManyPolynomialsHornerPlan<F> {
    fn algorithm_id(&self) -> AlgorithmId {
        AlgorithmId::new(
            OperationKind::HornerManyPolynomials,
            AlgorithmFamily::Horner,
            1,
        )
    }

    fn logical_len(&self) -> usize {
        self.polynomial_count
    }

    fn backend_id(&self) -> BackendId {
        self.backend
    }

    fn field_id(&self) -> FieldId {
        self.field_id
    }

    fn workspace_layout(&self) -> WorkspaceLayout {
        WorkspaceLayout::new(0, 0, 1, false, AllocationBehavior::None)
    }
}

impl<F> Engine<F>
where
    F: PortableField + StaticField,
{
    /// Evaluates one ascending-degree polynomial at many points.
    ///
    /// # Errors
    ///
    /// Returns a shape or length error before modifying `out`.
    pub fn horner_many_points_into(
        &self,
        out: &mut [F],
        coefficients: &[F],
        points: &[F],
    ) -> Result<(), HornerError> {
        ManyPointsHornerPlan::new(self, points.len(), coefficients.len())?.execute(
            self,
            out,
            coefficients,
            points,
        )
    }

    /// Evaluates many equally shaped polynomials at one point.
    ///
    /// # Errors
    ///
    /// Returns a shape, overflow or length error before modifying `out`.
    pub fn horner_many_polynomials_into(
        &self,
        out: &mut [F],
        coefficients: &[F],
        coefficient_count: usize,
        layout: CoefficientLayout,
        point: F,
    ) -> Result<(), HornerError> {
        ManyPolynomialsHornerPlan::new(self, out.len(), coefficient_count, layout)?.execute(
            self,
            out,
            coefficients,
            point,
        )
    }
}

fn validate_len(argument: &'static str, expected: usize, actual: usize) -> Result<(), HornerError> {
    if expected != actual {
        return Err(HornerError::LengthMismatch {
            argument,
            expected,
            actual,
        });
    }
    Ok(())
}
