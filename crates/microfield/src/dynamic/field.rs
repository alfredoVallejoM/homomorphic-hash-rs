//! Immutable dynamic contexts and non-`Copy` element value objects.

use std::{fmt::Write as _, sync::Arc};

use num_bigint::BigUint;
use num_integer::Integer as _;
use num_traits::{One as _, ToPrimitive as _, Zero as _};
use sha2::{Digest as _, Sha256};

use crate::{FieldId, PocklingtonCertificate, ValidationAssurance};

use super::DynFieldError;

const FIELD_ID_DOMAIN: &[u8] = b"microfield:field-id:v1\0";
const DEFAULT_MAXIMUM_BITS: u32 = 4_096;
const DEFAULT_MAXIMUM_DEGREE: u32 = 4_096;
const DEFAULT_MAXIMUM_TERMS: u32 = 1_024;
const DEFAULT_MAXIMUM_STEPS: u64 = 20_000_000;

/// Validation ceilings applied before allocating or executing long proofs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DynValidationLimits {
    /// Maximum prime characteristic size.
    pub maximum_characteristic_bits: u32,
    /// Maximum binary extension degree.
    pub maximum_degree: u32,
    /// Maximum binary modulus terms.
    pub maximum_modulus_terms: u32,
    /// Maximum counted validation operations.
    pub maximum_validation_steps: u64,
    /// Maximum Miller-Rabin rounds for probable-prime contexts.
    pub maximum_probable_prime_rounds: u32,
}

impl Default for DynValidationLimits {
    fn default() -> Self {
        Self {
            maximum_characteristic_bits: DEFAULT_MAXIMUM_BITS,
            maximum_degree: DEFAULT_MAXIMUM_DEGREE,
            maximum_modulus_terms: DEFAULT_MAXIMUM_TERMS,
            maximum_validation_steps: DEFAULT_MAXIMUM_STEPS,
            maximum_probable_prime_rounds: 128,
        }
    }
}

/// Supported runtime field family.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum DynFamilyKind {
    /// `GF(2^m)` in the canonical polynomial basis.
    BinaryPolynomial,
    /// Prime field `GF(p)` in canonical integer encoding.
    Prime,
}

#[derive(Clone, Debug)]
enum DynFamily {
    Binary {
        degree: u32,
        modulus_exponents: Box<[u32]>,
        modulus: BigUint,
    },
    Prime {
        modulus: BigUint,
        certificate: Option<PocklingtonCertificate>,
    },
}

#[derive(Debug)]
struct DynFieldInner {
    id: FieldId,
    name: String,
    descriptor_json: String,
    assurance: ValidationAssurance,
    canonical_bytes: usize,
    limbs: u16,
    family: DynFamily,
}

/// One validated runtime field context.
#[derive(Clone, Debug)]
pub struct DynField {
    inner: Arc<DynFieldInner>,
}

/// Inline-first private storage for one dynamic field element.
///
/// Up to eight 64-bit limbs require no per-element heap allocation. The
/// internal limbs are intentionally not exposed.
#[derive(Clone, Eq, PartialEq)]
pub struct DynLimbStorage {
    inline: [u64; 8],
    heap: Option<Box<[u64]>>,
    len: u16,
}

impl core::fmt::Debug for DynLimbStorage {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("DynLimbStorage")
            .field("limbs", &self.len)
            .field("inline", &self.heap.is_none())
            .finish_non_exhaustive()
    }
}

impl DynLimbStorage {
    fn zero(limbs: u16) -> Self {
        let heap = (usize::from(limbs) > 8).then(|| vec![0_u64; usize::from(limbs)].into());
        Self {
            inline: [0; 8],
            heap,
            len: limbs,
        }
    }

    fn from_biguint(value: &BigUint, limbs: u16) -> Self {
        let mut storage = Self::zero(limbs);
        storage.assign_biguint(value);
        storage
    }

    pub(super) fn assign_biguint(&mut self, value: &BigUint) {
        let words = value.to_u64_digits();
        let target = self.words_mut();
        target.fill(0);
        target[..words.len()].copy_from_slice(&words);
    }

    pub(super) fn to_biguint(&self) -> BigUint {
        let mut bytes = Vec::with_capacity(usize::from(self.len) * 8);
        for word in self.words() {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        BigUint::from_bytes_le(&bytes)
    }

    fn words(&self) -> &[u64] {
        match self.heap.as_deref() {
            Some(words) => words,
            None => &self.inline[..usize::from(self.len)],
        }
    }

    fn words_mut(&mut self) -> &mut [u64] {
        match self.heap.as_deref_mut() {
            Some(words) => words,
            None => &mut self.inline[..usize::from(self.len)],
        }
    }

    /// Returns the fixed limb count selected by the context.
    #[must_use]
    pub const fn limb_count(&self) -> u16 {
        self.len
    }

    /// Reports whether the value is held in the inline eight-limb region.
    #[must_use]
    pub const fn is_inline(&self) -> bool {
        self.heap.is_none()
    }
}

/// A dynamic element carrying its nominal field identity.
#[derive(Clone, Eq, PartialEq)]
pub struct DynElement {
    field_id: FieldId,
    storage: DynLimbStorage,
}

impl core::fmt::Debug for DynElement {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("DynElement")
            .field("field_id", &self.field_id)
            .field("storage", &self.storage)
            .finish()
    }
}

impl DynElement {
    /// Returns the field identity attached to this element.
    #[must_use]
    pub fn field_id(&self) -> FieldId {
        self.field_id
    }

    /// Returns diagnostics about inline versus heap-backed storage.
    #[must_use]
    pub const fn storage(&self) -> &DynLimbStorage {
        &self.storage
    }

    pub(super) fn value(&self) -> BigUint {
        self.storage.to_biguint()
    }
}

#[derive(Clone, Debug)]
enum PendingDefinition {
    Binary {
        degree: u32,
        modulus_exponents: Vec<u32>,
    },
    Prime {
        modulus_decimal: String,
    },
}

/// Builder that performs complete validation before publishing a context.
#[derive(Clone, Debug)]
pub struct DynFieldBuilder {
    name: String,
    definition: Option<PendingDefinition>,
    assurance: ValidationAssurance,
    limits: DynValidationLimits,
    certificate: Option<PocklingtonCertificate>,
}

impl DynFieldBuilder {
    /// Selects a binary polynomial definition.
    #[must_use]
    pub fn binary(mut self, degree: u32, modulus_exponents: impl Into<Vec<u32>>) -> Self {
        self.definition = Some(PendingDefinition::Binary {
            degree,
            modulus_exponents: modulus_exponents.into(),
        });
        self.assurance = ValidationAssurance::Proven;
        self.certificate = None;
        self
    }

    /// Selects a prime field definition from an unsigned decimal modulus.
    #[must_use]
    pub fn prime(mut self, modulus_decimal: impl Into<String>) -> Self {
        self.definition = Some(PendingDefinition::Prime {
            modulus_decimal: modulus_decimal.into(),
        });
        self
    }

    /// Supplies a replayable proof for a proven prime above `u64`.
    #[must_use]
    pub fn pocklington_certificate(mut self, certificate: PocklingtonCertificate) -> Self {
        self.certificate = Some(certificate);
        self
    }

    /// Chooses deterministic proof or explicit probable-prime assurance.
    #[must_use]
    pub const fn assurance(mut self, assurance: ValidationAssurance) -> Self {
        self.assurance = assurance;
        self
    }

    /// Replaces the defensive resource ceilings.
    #[must_use]
    pub const fn limits(mut self, limits: DynValidationLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Validates and freezes the complete context.
    ///
    /// # Errors
    ///
    /// Rejects malformed definitions, reducible binary polynomials,
    /// composites, insufficient proof strength and exceeded limits.
    pub fn build(self) -> Result<DynField, DynFieldError> {
        validate_name(&self.name)?;
        match self.definition {
            Some(PendingDefinition::Binary {
                degree,
                modulus_exponents,
            }) => build_binary(self.name, degree, modulus_exponents, self.limits),
            Some(PendingDefinition::Prime { modulus_decimal }) => build_prime(
                self.name,
                &modulus_decimal,
                self.assurance,
                self.certificate,
                self.limits,
            ),
            None => Err(DynFieldError::InvalidDefinition(
                "a binary or prime definition is required".to_owned(),
            )),
        }
    }
}

impl DynField {
    /// Starts a dynamic field definition with a presentation name.
    #[must_use]
    pub fn builder(name: impl Into<String>) -> DynFieldBuilder {
        DynFieldBuilder {
            name: name.into(),
            definition: None,
            assurance: ValidationAssurance::Proven,
            limits: DynValidationLimits::default(),
            certificate: None,
        }
    }

    /// Returns the semantic field identity, independent of its name.
    #[must_use]
    pub fn field_id(&self) -> FieldId {
        self.inner.id
    }

    /// Returns the human-facing presentation name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.inner.name
    }

    /// Returns the fixed-order minified identity descriptor.
    #[must_use]
    pub fn descriptor_json(&self) -> &str {
        &self.inner.descriptor_json
    }

    /// Returns the validation strength established during construction.
    #[must_use]
    pub fn assurance(&self) -> ValidationAssurance {
        self.inner.assurance
    }

    /// Returns the mathematical family.
    #[must_use]
    pub fn family(&self) -> DynFamilyKind {
        match &self.inner.family {
            DynFamily::Binary { .. } => DynFamilyKind::BinaryPolynomial,
            DynFamily::Prime { .. } => DynFamilyKind::Prime,
        }
    }

    /// Reports whether graph channels operate in characteristic two.
    #[must_use]
    pub fn characteristic_is_two(&self) -> bool {
        match &self.inner.family {
            DynFamily::Binary { .. } => true,
            DynFamily::Prime { modulus, .. } => modulus == &BigUint::from(2_u8),
        }
    }

    /// Extension degree of the runtime presentation.
    #[must_use]
    pub fn extension_degree(&self) -> u32 {
        match &self.inner.family {
            DynFamily::Binary { degree, .. } => *degree,
            DynFamily::Prime { .. } => 1,
        }
    }

    /// Returns the exact canonical byte width.
    #[must_use]
    pub fn canonical_bytes(&self) -> usize {
        self.inner.canonical_bytes
    }

    /// Constructs zero without a scalar identity check.
    #[must_use]
    pub fn zero(&self) -> DynElement {
        self.element_unchecked(&BigUint::zero())
    }

    /// Constructs one without a scalar identity check.
    #[must_use]
    pub fn one(&self) -> DynElement {
        self.element_unchecked(&BigUint::one())
    }

    /// Decodes exactly one canonical little-endian representation.
    ///
    /// # Errors
    ///
    /// Rejects wrong lengths and values outside the canonical range.
    pub fn decode(&self, bytes: &[u8]) -> Result<DynElement, DynFieldError> {
        if bytes.len() != self.inner.canonical_bytes {
            return Err(DynFieldError::LengthMismatch {
                expected: self.inner.canonical_bytes,
                actual: bytes.len(),
            });
        }
        let value = BigUint::from_bytes_le(bytes);
        if !self.is_canonical(&value) {
            return Err(DynFieldError::NonCanonicalValue);
        }
        Ok(self.element_unchecked(&value))
    }

    /// Interprets arbitrary little-endian bytes in this context and reduces
    /// them according to its declared family.
    ///
    /// Binary contexts interpret bits as polynomial coefficients. Prime
    /// contexts interpret the bytes as an unsigned integer. This is reduction,
    /// not canonical decoding; callers that require a unique representation
    /// must use [`Self::decode`].
    #[must_use]
    pub fn reduce_bytes_mod_order(&self, bytes_le: &[u8]) -> DynElement {
        let value = BigUint::from_bytes_le(bytes_le);
        let reduced = match &self.inner.family {
            DynFamily::Binary {
                degree, modulus, ..
            } => polynomial_reduce(value, *degree, modulus),
            DynFamily::Prime { modulus, .. } => value % modulus,
        };
        self.element_unchecked(&reduced)
    }

    /// Encodes an element into an exact-size caller buffer.
    ///
    /// # Errors
    ///
    /// Rejects mixed fields and wrong output lengths without modifying output.
    pub fn encode(&self, value: &DynElement, out: &mut [u8]) -> Result<(), DynFieldError> {
        self.check_element(value)?;
        if out.len() != self.inner.canonical_bytes {
            return Err(DynFieldError::LengthMismatch {
                expected: self.inner.canonical_bytes,
                actual: out.len(),
            });
        }
        let bytes = value.value().to_bytes_le();
        out.fill(0);
        out[..bytes.len()].copy_from_slice(&bytes);
        Ok(())
    }

    /// Adds two elements after nominal identity checks.
    ///
    /// # Errors
    ///
    /// Rejects elements carrying another `FieldId`.
    pub fn add(&self, lhs: &DynElement, rhs: &DynElement) -> Result<DynElement, DynFieldError> {
        let (lhs, rhs) = self.checked_pair(lhs, rhs)?;
        Ok(self.element_unchecked(&self.add_values(&lhs, &rhs)))
    }

    /// Subtracts two elements after nominal identity checks.
    ///
    /// # Errors
    ///
    /// Rejects elements carrying another `FieldId`.
    pub fn sub(&self, lhs: &DynElement, rhs: &DynElement) -> Result<DynElement, DynFieldError> {
        let (lhs, rhs) = self.checked_pair(lhs, rhs)?;
        Ok(self.element_unchecked(&self.sub_values(&lhs, &rhs)))
    }

    /// Multiplies two elements after nominal identity checks.
    ///
    /// # Errors
    ///
    /// Rejects elements carrying another `FieldId`.
    pub fn mul(&self, lhs: &DynElement, rhs: &DynElement) -> Result<DynElement, DynFieldError> {
        let (lhs, rhs) = self.checked_pair(lhs, rhs)?;
        Ok(self.element_unchecked(&self.mul_values(&lhs, &rhs)))
    }

    /// Squares one element after a nominal identity check.
    ///
    /// # Errors
    ///
    /// Rejects an element carrying another `FieldId`.
    pub fn square(&self, value: &DynElement) -> Result<DynElement, DynFieldError> {
        self.check_element(value)?;
        Ok(self.element_unchecked(&self.square_value(&value.value())))
    }

    /// Computes a multiplicative inverse.
    ///
    /// # Errors
    ///
    /// Rejects a mixed-field value and zero.
    pub fn invert(&self, value: &DynElement) -> Result<DynElement, DynFieldError> {
        self.check_element(value)?;
        let value = value.value();
        if value.is_zero() {
            return Err(DynFieldError::DivisionByZero);
        }
        let exponent = match &self.inner.family {
            DynFamily::Binary { degree, .. } => (BigUint::one() << *degree) - 2_u8,
            DynFamily::Prime { modulus, .. } => modulus - 2_u8,
        };
        Ok(self.element_unchecked(&self.pow_value(&value, exponent)))
    }

    /// Exports the exact canonical manifest accepted by the corresponding
    /// static generator family.
    #[must_use]
    pub fn export_manifest(&self) -> String {
        match &self.inner.family {
            DynFamily::Binary {
                degree,
                modulus_exponents,
                ..
            } => {
                let terms = join_u32(modulus_exponents);
                format!(
                    "schema_version = 1\n\n[field]\nname = \"{}\"\ncharacteristic = 2\ndegree = {degree}\n\n[field.basis]\nkind = \"polynomial\"\ncoefficient_order = \"ascending\"\n\n[field.modulus]\nnonzero_exponents = [{terms}]\n\n[field.encoding]\nbyte_order = \"little\"\nbit_order = \"lsb0\"\ncanonical_bytes = {}\n\n[build]\nlimb_bits = 64\nproduct_strategies = [\"schoolbook\"]\nreduction_style = \"generated_fold\"\nrequested_backends = [\"portable\"]\n",
                    self.inner.name, self.inner.canonical_bytes
                )
            }
            DynFamily::Prime {
                modulus,
                certificate,
            } => {
                let validation = match self.inner.assurance {
                    ValidationAssurance::Proven => "assurance = \"proven\"\n".to_owned(),
                    ValidationAssurance::ProbablePrime { rounds } => {
                        format!("assurance = \"probable_prime\"\nrounds = {rounds}\n")
                    }
                };
                let mut manifest = format!(
                    "prime_schema_version = 1\n\n[prime]\nname = \"{}\"\nmodulus = \"{}\"\n\n[encoding]\nbyte_order = \"little\"\ninteger = \"canonical\"\ncanonical_bytes = {}\n\n[validation]\n{validation}",
                    self.inner.name, modulus, self.inner.canonical_bytes,
                );
                if let Some(certificate) = certificate {
                    let _ = write!(
                        manifest,
                        "\n[certificate]\nalgorithm = \"{}\"\nknown_factor_product = \"{}\"\ncofactor = \"{}\"\n",
                        certificate.algorithm,
                        certificate.known_factor_product,
                        certificate.cofactor
                    );
                    let mut factors = certificate.factors.clone();
                    factors.sort_unstable_by_key(|factor| factor.prime);
                    for factor in factors {
                        let _ = write!(
                            manifest,
                            "\n[[certificate.factors]]\nprime = {}\nexponent = {}\nwitness = {}\n",
                            factor.prime, factor.exponent, factor.witness
                        );
                    }
                }
                manifest
            }
        }
    }

    pub(super) fn same_field(&self, other: &Self) -> bool {
        self.inner.id == other.inner.id
    }

    pub(super) fn limbs(&self) -> u16 {
        self.inner.limbs
    }

    pub(super) fn storage_from_value(&self, value: &BigUint) -> DynLimbStorage {
        DynLimbStorage::from_biguint(value, self.inner.limbs)
    }

    pub(super) fn element_from_storage(&self, storage: DynLimbStorage) -> DynElement {
        DynElement {
            field_id: self.inner.id,
            storage,
        }
    }

    pub(super) fn add_values(&self, lhs: &BigUint, rhs: &BigUint) -> BigUint {
        match &self.inner.family {
            DynFamily::Binary { .. } => lhs ^ rhs,
            DynFamily::Prime { modulus, .. } => (lhs + rhs) % modulus,
        }
    }

    pub(super) fn sub_values(&self, lhs: &BigUint, rhs: &BigUint) -> BigUint {
        match &self.inner.family {
            DynFamily::Binary { .. } => lhs ^ rhs,
            DynFamily::Prime { .. } if lhs >= rhs => lhs - rhs,
            DynFamily::Prime { modulus, .. } => modulus - (rhs - lhs),
        }
    }

    pub(super) fn mul_values(&self, lhs: &BigUint, rhs: &BigUint) -> BigUint {
        match &self.inner.family {
            DynFamily::Binary {
                degree, modulus, ..
            } => polynomial_mul_mod(lhs, rhs, *degree, modulus),
            DynFamily::Prime { modulus, .. } => (lhs * rhs) % modulus,
        }
    }

    pub(super) fn square_value(&self, value: &BigUint) -> BigUint {
        self.mul_values(value, value)
    }

    pub(super) fn invert_value(&self, value: &BigUint) -> Result<BigUint, DynFieldError> {
        if value.is_zero() {
            return Err(DynFieldError::DivisionByZero);
        }
        let exponent = match &self.inner.family {
            DynFamily::Binary { degree, .. } => (BigUint::one() << *degree) - 2_u8,
            DynFamily::Prime { modulus, .. } => modulus - 2_u8,
        };
        Ok(self.pow_value(value, exponent))
    }

    fn pow_value(&self, value: &BigUint, mut exponent: BigUint) -> BigUint {
        let mut result = BigUint::one();
        let mut base = value.clone();
        while !exponent.is_zero() {
            if exponent.bit(0) {
                result = self.mul_values(&result, &base);
            }
            exponent >>= 1_u8;
            if !exponent.is_zero() {
                base = self.square_value(&base);
            }
        }
        result
    }

    fn checked_pair(
        &self,
        lhs: &DynElement,
        rhs: &DynElement,
    ) -> Result<(BigUint, BigUint), DynFieldError> {
        self.check_element(lhs)?;
        self.check_element(rhs)?;
        Ok((lhs.value(), rhs.value()))
    }

    fn check_element(&self, value: &DynElement) -> Result<(), DynFieldError> {
        if value.field_id != self.inner.id {
            return Err(DynFieldError::FieldMismatch {
                expected: self.inner.id,
                actual: value.field_id,
            });
        }
        if value.storage.len != self.inner.limbs {
            return Err(DynFieldError::InvalidDefinition(
                "element storage shape disagrees with its context".to_owned(),
            ));
        }
        Ok(())
    }

    fn element_unchecked(&self, value: &BigUint) -> DynElement {
        DynElement {
            field_id: self.inner.id,
            storage: DynLimbStorage::from_biguint(value, self.inner.limbs),
        }
    }

    fn is_canonical(&self, value: &BigUint) -> bool {
        match &self.inner.family {
            DynFamily::Binary { degree, .. } => value.bits() <= u64::from(*degree),
            DynFamily::Prime { modulus, .. } => value < modulus,
        }
    }
}

fn build_binary(
    name: String,
    degree: u32,
    mut exponents: Vec<u32>,
    limits: DynValidationLimits,
) -> Result<DynField, DynFieldError> {
    if degree == 0 || degree > limits.maximum_degree {
        return Err(DynFieldError::LimitExceeded {
            limit: "maximum_degree",
            maximum: u64::from(limits.maximum_degree),
        });
    }
    if exponents.len() > limits.maximum_modulus_terms as usize {
        return Err(DynFieldError::LimitExceeded {
            limit: "maximum_modulus_terms",
            maximum: u64::from(limits.maximum_modulus_terms),
        });
    }
    exponents.sort_unstable_by(|lhs, rhs| rhs.cmp(lhs));
    exponents.dedup();
    if exponents.first() != Some(&degree) || exponents.last() != Some(&0) {
        return Err(DynFieldError::InvalidDefinition(
            "binary modulus must contain exactly its leading degree and constant term".to_owned(),
        ));
    }
    if exponents.iter().any(|exponent| *exponent > degree) {
        return Err(DynFieldError::InvalidDefinition(
            "binary modulus contains an exponent above its degree".to_owned(),
        ));
    }
    let modulus = polynomial_from_exponents(&exponents);
    let mut steps = 0_u64;
    if !rabin_irreducible(
        &modulus,
        degree,
        &mut steps,
        limits.maximum_validation_steps,
    )? {
        return Err(DynFieldError::InvalidDefinition(
            "binary modulus is reducible over GF(2)".to_owned(),
        ));
    }
    let bytes = usize::try_from(degree.div_ceil(8)).expect("u32 fits usize");
    let terms = join_u32(&exponents);
    let descriptor = format!(
        "{{\"schema\":1,\"characteristic\":\"2\",\"degree\":{degree},\"basis\":{{\"kind\":\"polynomial\",\"coefficient_order\":\"ascending\"}},\"modulus\":[{terms}],\"encoding\":{{\"byte_order\":\"little\",\"bit_order\":\"lsb0\",\"bytes\":{bytes}}}}}"
    );
    Ok(freeze(
        name,
        descriptor,
        ValidationAssurance::Proven,
        bytes,
        DynFamily::Binary {
            degree,
            modulus_exponents: exponents.into_boxed_slice(),
            modulus,
        },
    ))
}

fn build_prime(
    name: String,
    modulus_decimal: &str,
    assurance: ValidationAssurance,
    certificate: Option<PocklingtonCertificate>,
    limits: DynValidationLimits,
) -> Result<DynField, DynFieldError> {
    if modulus_decimal.is_empty()
        || modulus_decimal.starts_with('0')
        || !modulus_decimal.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(DynFieldError::InvalidDefinition(
            "prime modulus must be canonical unsigned decimal".to_owned(),
        ));
    }
    let modulus = BigUint::parse_bytes(modulus_decimal.as_bytes(), 10).ok_or_else(|| {
        DynFieldError::InvalidDefinition("prime modulus is not an integer".to_owned())
    })?;
    let bits = u32::try_from(modulus.bits()).unwrap_or(u32::MAX);
    if bits > limits.maximum_characteristic_bits {
        return Err(DynFieldError::LimitExceeded {
            limit: "maximum_characteristic_bits",
            maximum: u64::from(limits.maximum_characteristic_bits),
        });
    }
    if modulus < BigUint::from(3_u8) || modulus.is_even() {
        return Err(DynFieldError::InvalidDefinition(
            "prime modulus must be an odd integer of at least three".to_owned(),
        ));
    }
    match assurance {
        ValidationAssurance::Proven => {
            if let Some(candidate) = modulus.to_u64() {
                if !is_prime_u64(candidate) {
                    return Err(DynFieldError::InvalidDefinition(
                        "prime modulus is composite".to_owned(),
                    ));
                }
            } else {
                let certificate = certificate.as_ref().ok_or(DynFieldError::ProofRequired)?;
                verify_dynamic_pocklington(&modulus, certificate, limits)?;
            }
        }
        ValidationAssurance::ProbablePrime { rounds } => {
            if certificate.is_some() {
                return Err(DynFieldError::InvalidDefinition(
                    "a probable-prime context cannot carry a proof certificate".to_owned(),
                ));
            }
            if rounds < 16 || rounds > limits.maximum_probable_prime_rounds {
                return Err(DynFieldError::InvalidDefinition(format!(
                    "probable-prime rounds must be in 16..={}",
                    limits.maximum_probable_prime_rounds
                )));
            }
            if u64::from(rounds) > limits.maximum_validation_steps {
                return Err(DynFieldError::LimitExceeded {
                    limit: "maximum_validation_steps",
                    maximum: limits.maximum_validation_steps,
                });
            }
            if !miller_rabin(&modulus, rounds) {
                return Err(DynFieldError::InvalidDefinition(
                    "prime modulus is composite".to_owned(),
                ));
            }
        }
    }
    let bytes = usize::try_from(modulus.bits().div_ceil(8)).expect("bounded bits fit usize");
    let descriptor = format!(
        "{{\"schema\":2,\"characteristic\":\"{modulus}\",\"degree\":1,\"basis\":{{\"kind\":\"prime\"}},\"modulus\":\"{modulus}\",\"encoding\":{{\"byte_order\":\"little\",\"integer\":\"canonical\",\"bytes\":{bytes}}}}}"
    );
    Ok(freeze(
        name,
        descriptor,
        assurance,
        bytes,
        DynFamily::Prime {
            modulus,
            certificate,
        },
    ))
}

fn freeze(
    name: String,
    descriptor_json: String,
    assurance: ValidationAssurance,
    canonical_bytes: usize,
    family: DynFamily,
) -> DynField {
    let id = field_id(descriptor_json.as_bytes());
    let limbs = u16::try_from(canonical_bytes.div_ceil(8)).expect("validation limits fit u16");
    DynField {
        inner: Arc::new(DynFieldInner {
            id,
            name,
            descriptor_json,
            assurance,
            canonical_bytes,
            limbs,
            family,
        }),
    }
}

fn field_id(descriptor: &[u8]) -> FieldId {
    let mut hasher = Sha256::new();
    hasher.update(FIELD_ID_DOMAIN);
    hasher.update(descriptor);
    FieldId::from_bytes(hasher.finalize().into())
}

fn validate_name(name: &str) -> Result<(), DynFieldError> {
    let mut bytes = name.bytes();
    let valid_first = bytes.next().is_some_and(|byte| byte.is_ascii_lowercase());
    if !valid_first
        || !bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        || name.ends_with('_')
        || name.contains("__")
    {
        return Err(DynFieldError::InvalidDefinition(
            "name must use normalized lower_snake_case".to_owned(),
        ));
    }
    Ok(())
}

fn join_u32(values: &[u32]) -> String {
    values
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn polynomial_from_exponents(exponents: &[u32]) -> BigUint {
    let mut polynomial = BigUint::zero();
    for exponent in exponents {
        polynomial.set_bit(u64::from(*exponent), true);
    }
    polynomial
}

fn polynomial_mul_mod(lhs: &BigUint, rhs: &BigUint, degree: u32, modulus: &BigUint) -> BigUint {
    let mut product = BigUint::zero();
    let mut left = lhs.clone();
    let mut right = rhs.clone();
    while !right.is_zero() {
        if right.bit(0) {
            product ^= &left;
        }
        left <<= 1_u8;
        right >>= 1_u8;
    }
    polynomial_reduce(product, degree, modulus)
}

fn polynomial_reduce(mut value: BigUint, degree: u32, modulus: &BigUint) -> BigUint {
    while value.bits() > u64::from(degree) {
        let shift = value.bits() - 1 - u64::from(degree);
        value ^= modulus << usize::try_from(shift).expect("bounded degree fits usize");
    }
    value
}

fn polynomial_gcd(mut lhs: BigUint, mut rhs: BigUint) -> BigUint {
    while !rhs.is_zero() {
        let remainder = polynomial_remainder(lhs, &rhs);
        lhs = rhs;
        rhs = remainder;
    }
    lhs
}

fn polynomial_remainder(mut dividend: BigUint, divisor: &BigUint) -> BigUint {
    let divisor_degree = divisor.bits() - 1;
    while !dividend.is_zero() && dividend.bits() > divisor_degree {
        let shift = dividend.bits() - 1 - divisor_degree;
        dividend ^= divisor << usize::try_from(shift).expect("bounded degree fits usize");
    }
    dividend
}

fn rabin_irreducible(
    modulus: &BigUint,
    degree: u32,
    steps: &mut u64,
    maximum_steps: u64,
) -> Result<bool, DynFieldError> {
    let x = BigUint::from(2_u8);
    for factor in distinct_prime_factors(degree) {
        let iterations = degree / factor;
        let mut power = x.clone();
        for _ in 0..iterations {
            consume_step(steps, maximum_steps)?;
            power = polynomial_mul_mod(&power, &power, degree, modulus);
        }
        if polynomial_gcd(&power ^ &x, modulus.clone()) != BigUint::one() {
            return Ok(false);
        }
    }
    let mut power = x.clone();
    for _ in 0..degree {
        consume_step(steps, maximum_steps)?;
        power = polynomial_mul_mod(&power, &power, degree, modulus);
    }
    Ok(power == x)
}

fn consume_step(steps: &mut u64, maximum: u64) -> Result<(), DynFieldError> {
    *steps = steps.saturating_add(1);
    if *steps > maximum {
        return Err(DynFieldError::LimitExceeded {
            limit: "maximum_validation_steps",
            maximum,
        });
    }
    Ok(())
}

fn distinct_prime_factors(mut value: u32) -> Vec<u32> {
    let mut factors = Vec::new();
    let mut divisor = 2_u32;
    while divisor.saturating_mul(divisor) <= value {
        if value.is_multiple_of(divisor) {
            factors.push(divisor);
            while value.is_multiple_of(divisor) {
                value /= divisor;
            }
        }
        divisor += 1;
    }
    if value > 1 {
        factors.push(value);
    }
    factors
}

fn is_prime_u64(candidate: u64) -> bool {
    if candidate < 2 {
        return false;
    }
    for prime in [2_u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        if candidate == prime {
            return true;
        }
        if candidate.is_multiple_of(prime) {
            return false;
        }
    }
    let modulus = BigUint::from(candidate);
    // This published base set is deterministic over the complete u64 range.
    miller_rabin_bases(
        &modulus,
        &[2, 325, 9_375, 28_178, 450_775, 9_780_504, 1_795_265_022],
    )
}

fn verify_dynamic_pocklington(
    modulus: &BigUint,
    certificate: &PocklingtonCertificate,
    limits: DynValidationLimits,
) -> Result<(), DynFieldError> {
    if certificate.algorithm != "pocklington-v1"
        || certificate.factors.is_empty()
        || certificate.factors.len() > limits.maximum_modulus_terms as usize
    {
        return Err(DynFieldError::InvalidDefinition(
            "invalid Pocklington algorithm or factor count".to_owned(),
        ));
    }
    let estimated_steps = modulus
        .bits()
        .saturating_mul(certificate.factors.len() as u64)
        .saturating_mul(4);
    if estimated_steps > limits.maximum_validation_steps {
        return Err(DynFieldError::LimitExceeded {
            limit: "maximum_validation_steps",
            maximum: limits.maximum_validation_steps,
        });
    }
    let known =
        parse_certificate_integer("known_factor_product", &certificate.known_factor_product)?;
    let cofactor = parse_certificate_integer("cofactor", &certificate.cofactor)?;
    let modulus_minus_one = modulus - 1_u8;
    if &known * &cofactor != modulus_minus_one || &known * &known <= *modulus {
        return Err(DynFieldError::InvalidDefinition(
            "Pocklington factorization or sqrt bound failed".to_owned(),
        ));
    }
    let mut factors = certificate.factors.clone();
    factors.sort_unstable_by_key(|factor| factor.prime);
    if factors
        .windows(2)
        .any(|pair| pair[0].prime == pair[1].prime)
    {
        return Err(DynFieldError::InvalidDefinition(
            "Pocklington factors must be distinct".to_owned(),
        ));
    }
    let mut reconstructed = BigUint::one();
    for factor in factors {
        if factor.exponent == 0
            || u64::from(factor.exponent) > modulus.bits()
            || !is_prime_u64(factor.prime)
        {
            return Err(DynFieldError::InvalidDefinition(format!(
                "invalid Pocklington factor {}",
                factor.prime
            )));
        }
        reconstructed *= BigUint::from(factor.prime).pow(factor.exponent);
        let witness = BigUint::from(factor.witness) % modulus;
        if witness.is_zero() || witness.modpow(&modulus_minus_one, modulus) != BigUint::one() {
            return Err(DynFieldError::InvalidDefinition(format!(
                "Pocklington Fermat witness failed for {}",
                factor.prime
            )));
        }
        let residue = witness.modpow(&(&modulus_minus_one / factor.prime), modulus);
        let difference = if residue.is_zero() {
            modulus_minus_one.clone()
        } else {
            residue - 1_u8
        };
        if difference.gcd(modulus) != BigUint::one() {
            return Err(DynFieldError::InvalidDefinition(format!(
                "Pocklington gcd failed for {}",
                factor.prime
            )));
        }
    }
    if reconstructed != known {
        return Err(DynFieldError::InvalidDefinition(
            "Pocklington factors do not reconstruct the known product".to_owned(),
        ));
    }
    Ok(())
}

fn parse_certificate_integer(name: &str, value: &str) -> Result<BigUint, DynFieldError> {
    if value.is_empty()
        || value.starts_with('0')
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(DynFieldError::InvalidDefinition(format!(
            "Pocklington {name} is not canonical decimal"
        )));
    }
    BigUint::parse_bytes(value.as_bytes(), 10).ok_or_else(|| {
        DynFieldError::InvalidDefinition(format!("Pocklington {name} is not an integer"))
    })
}

fn miller_rabin(candidate: &BigUint, rounds: u32) -> bool {
    const BASES: [u64; 32] = [
        2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89,
        97, 101, 103, 107, 109, 113, 127, 131,
    ];
    let bases = (0..rounds)
        .map(|index| BASES[index as usize % BASES.len()])
        .collect::<Vec<_>>();
    miller_rabin_bases(candidate, &bases)
}

fn miller_rabin_bases(candidate: &BigUint, bases: &[u64]) -> bool {
    let one = BigUint::one();
    let minus_one = candidate - &one;
    let mut odd_part = minus_one.clone();
    let mut twos = 0_u32;
    while odd_part.is_even() {
        odd_part >>= 1_u8;
        twos += 1;
    }
    'bases: for base in bases {
        let base = BigUint::from(*base) % candidate;
        if base.is_zero() {
            continue;
        }
        let mut witness = base.modpow(&odd_part, candidate);
        if witness == one || witness == minus_one {
            continue;
        }
        for _ in 1..twos {
            witness = (&witness * &witness) % candidate;
            if witness == minus_one {
                continue 'bases;
            }
        }
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_u64_primality_rejects_strong_pseudoprimes() {
        assert!(is_prime_u64(4_294_967_291));
        assert!(!is_prime_u64(341_550_071_728_321));
        assert!(!is_prime_u64(3_825_123_056_546_413_051));
    }

    #[test]
    fn reducible_binary_polynomial_is_rejected() {
        let error = DynField::builder("bad_binary")
            .binary(4, vec![4, 2, 0])
            .build()
            .expect_err("x^4+x^2+1 is reducible");
        assert!(matches!(error, DynFieldError::InvalidDefinition(_)));
    }
}
