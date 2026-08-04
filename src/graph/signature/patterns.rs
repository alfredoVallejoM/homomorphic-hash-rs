//! Exact connected induced-pattern counts and compressed field fingerprints.

use core::fmt;
use std::collections::BTreeMap;

use microfield::{CanonicalEncoding, Field, Pow, StaticField};
use sha2::{Digest as _, Sha256};

use crate::structural::{SignatureAssurance, StructuralLaneEncoder};

use super::super::{GraphError, IncidenceGraph, VertexId};

const PATTERN_MAGIC: &[u8; 4] = b"MFPC";
const PATTERN_SCHEMA: u16 = 1;
const FINGERPRINT_MAGIC: &[u8; 4] = b"MFPF";
const FINGERPRINT_SCHEMA: u16 = 1;
const PRODUCT_FINGERPRINT_MAGIC: &[u8; 4] = b"MFPP";
const PRODUCT_FINGERPRINT_SCHEMA: u16 = 1;

/// Stable identity of one connected induced-pattern catalog.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct LoopPatternCatalogId([u8; 32]);

impl LoopPatternCatalogId {
    /// Borrows the stable digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Stable identity of a field-compressed pattern channel.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct PatternFingerprintId([u8; 32]);

impl PatternFingerprintId {
    /// Borrows the stable digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

macro_rules! impl_id_format {
    ($type:ty, $label:literal) => {
        impl fmt::Display for $type {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                for byte in self.as_bytes() {
                    write!(formatter, "{byte:02x}")?;
                }
                Ok(())
            }
        }

        impl fmt::Debug for $type {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, concat!($label, "({})"), self)
            }
        }
    };
}

impl_id_format!(LoopPatternCatalogId, "LoopPatternCatalogId");
impl_id_format!(PatternFingerprintId, "PatternFingerprintId");

/// Versioned family of connected induced relational patterns.
///
/// Version 1 supports orders one through four. Its loop order is the cycle
/// rank of the simple undirected support plus normalized self-loop records.
/// This is an exact induced-subgraph catalog, not yet a homomorphism-count
/// catalog and not a complete graph invariant.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LoopPatternCatalog {
    id: LoopPatternCatalogId,
    maximum_order: u8,
    maximum_loops: u8,
}

impl LoopPatternCatalog {
    /// Complete L0–L3 catalog through four selected vertices.
    #[must_use]
    pub fn l0_to_l3() -> Self {
        Self::new(4, 3).expect("the built-in catalog bounds are valid")
    }

    /// Builds a bounded catalog.
    ///
    /// # Errors
    ///
    /// Accepts only orders `1..=4` and loop bounds `0..=3` in schema v1.
    pub fn new(maximum_order: u8, maximum_loops: u8) -> Result<Self, GraphError> {
        if !(1..=4).contains(&maximum_order) || maximum_loops > 3 {
            return Err(GraphError::InvalidPatternCatalog);
        }
        let mut hasher = Sha256::new();
        hasher.update(b"microfield/connected-pattern-catalog/v1\0");
        hasher.update([maximum_order, maximum_loops]);
        Ok(Self {
            id: LoopPatternCatalogId(hasher.finalize().into()),
            maximum_order,
            maximum_loops,
        })
    }

    /// Stable catalog identity.
    #[must_use]
    pub const fn id(self) -> LoopPatternCatalogId {
        self.id
    }

    /// Largest selected vertex count.
    #[must_use]
    pub const fn maximum_order(self) -> u8 {
        self.maximum_order
    }

    /// Largest support cycle rank retained.
    #[must_use]
    pub const fn maximum_loops(self) -> u8 {
        self.maximum_loops
    }

    /// Counts every admitted connected induced pattern or skips atomically.
    ///
    /// # Errors
    ///
    /// Returns stable-size and counter failures. If `maximum_work` is too
    /// small, a complete skipped profile is returned with no partial counts.
    pub fn analyze(
        self,
        graph: &IncidenceGraph,
        maximum_work: u64,
    ) -> Result<ConnectedPatternProfile, GraphError> {
        let estimated_work = estimate_work(graph.vertex_count(), self.maximum_order)?;
        if estimated_work > maximum_work {
            return Ok(ConnectedPatternProfile {
                catalog: self,
                status: PatternAnalysisStatus::SkippedBudget,
                estimated_work,
                graph_count: 1,
                vertex_count: u64::try_from(graph.vertex_count())
                    .map_err(|_| GraphError::GraphTooLarge)?,
                counts: Vec::new(),
            });
        }
        let mut counts = BTreeMap::<Vec<u8>, (u8, u8, u64)>::new();
        let maximum_order = usize::from(self.maximum_order).min(graph.vertex_count());
        for order in 1..=maximum_order {
            let mut selected = Vec::with_capacity(order);
            enumerate_subsets(
                graph.vertex_count(),
                order,
                0,
                &mut selected,
                &mut |subset| {
                    let Some(loop_order) = connected_loop_order(graph, subset)? else {
                        return Ok(());
                    };
                    if loop_order > self.maximum_loops {
                        return Ok(());
                    }
                    let canonical = canonical_pattern_bytes(graph, subset, loop_order)?;
                    let entry = counts.entry(canonical).or_insert((
                        u8::try_from(order).map_err(|_| GraphError::GraphTooLarge)?,
                        loop_order,
                        0,
                    ));
                    entry.2 = entry.2.checked_add(1).ok_or(GraphError::GraphTooLarge)?;
                    Ok(())
                },
            )?;
        }
        let counts = counts
            .into_iter()
            .map(
                |(canonical_bytes, (order, loop_order, count))| ConnectedPatternCount {
                    order,
                    loop_order,
                    canonical_bytes,
                    count,
                },
            )
            .collect();
        Ok(ConnectedPatternProfile {
            catalog: self,
            status: PatternAnalysisStatus::Complete,
            estimated_work,
            graph_count: 1,
            vertex_count: u64::try_from(graph.vertex_count())
                .map_err(|_| GraphError::GraphTooLarge)?,
            counts,
        })
    }
}

impl Default for LoopPatternCatalog {
    fn default() -> Self {
        Self::l0_to_l3()
    }
}

/// Whether a bounded catalog was completely evaluated.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum PatternAnalysisStatus {
    /// Every admitted subset was enumerated.
    Complete = 1,
    /// The invariant preflight exceeded the work ceiling; no count is exposed.
    SkippedBudget = 2,
}

/// Exact count of one canonical relational induced pattern.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectedPatternCount {
    order: u8,
    loop_order: u8,
    canonical_bytes: Vec<u8>,
    count: u64,
}

impl ConnectedPatternCount {
    /// Number of selected vertices.
    #[must_use]
    pub const fn order(&self) -> u8 {
        self.order
    }

    /// Support cycle rank used by the catalog tier.
    #[must_use]
    pub const fn loop_order(&self) -> u8 {
        self.loop_order
    }

    /// Injective canonical descriptor of this small relational pattern.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Exact number of induced occurrences.
    #[must_use]
    pub const fn count(&self) -> u64 {
        self.count
    }
}

/// Exact, relabeling-invariant counts for one catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectedPatternProfile {
    catalog: LoopPatternCatalog,
    status: PatternAnalysisStatus,
    estimated_work: u64,
    graph_count: u64,
    vertex_count: u64,
    counts: Vec<ConnectedPatternCount>,
}

impl ConnectedPatternProfile {
    /// Catalog that defines all retained patterns.
    #[must_use]
    pub const fn catalog(&self) -> LoopPatternCatalog {
        self.catalog
    }

    /// Complete or atomically skipped status.
    #[must_use]
    pub const fn status(&self) -> PatternAnalysisStatus {
        self.status
    }

    /// Invariant preflight work estimate.
    #[must_use]
    pub const fn estimated_work(&self) -> u64 {
        self.estimated_work
    }

    /// Number of disjoint source graphs represented.
    #[must_use]
    pub const fn graph_count(&self) -> u64 {
        self.graph_count
    }

    /// Total source vertices represented.
    #[must_use]
    pub const fn vertex_count(&self) -> u64 {
        self.vertex_count
    }

    /// Sorted exact counts. Empty when the tier was skipped.
    #[must_use]
    pub fn counts(&self) -> &[ConnectedPatternCount] {
        &self.counts
    }

    /// Counts of connected patterns add under disjoint union.
    ///
    /// # Errors
    ///
    /// Rejects different catalogs, incomplete profiles and counter overflow.
    pub fn combine_disjoint(&self, other: &Self) -> Result<Self, GraphError> {
        if self.catalog != other.catalog {
            return Err(GraphError::PatternProfileMismatch);
        }
        if self.status != PatternAnalysisStatus::Complete
            || other.status != PatternAnalysisStatus::Complete
        {
            return Err(GraphError::PatternAnalysisIncomplete);
        }
        let mut merged = BTreeMap::<Vec<u8>, (u8, u8, u64)>::new();
        for pattern in self.counts.iter().chain(&other.counts) {
            let entry = merged.entry(pattern.canonical_bytes.clone()).or_insert((
                pattern.order,
                pattern.loop_order,
                0,
            ));
            entry.2 = entry
                .2
                .checked_add(pattern.count)
                .ok_or(GraphError::GraphTooLarge)?;
        }
        Ok(Self {
            catalog: self.catalog,
            status: PatternAnalysisStatus::Complete,
            estimated_work: self
                .estimated_work
                .checked_add(other.estimated_work)
                .ok_or(GraphError::GraphTooLarge)?,
            graph_count: self
                .graph_count
                .checked_add(other.graph_count)
                .ok_or(GraphError::GraphTooLarge)?,
            vertex_count: self
                .vertex_count
                .checked_add(other.vertex_count)
                .ok_or(GraphError::GraphTooLarge)?,
            counts: merged
                .into_iter()
                .map(
                    |(canonical_bytes, (order, loop_order, count))| ConnectedPatternCount {
                        order,
                        loop_order,
                        canonical_bytes,
                        count,
                    },
                )
                .collect(),
        })
    }

    /// Exact tracking assurance for the defined bounded catalog.
    #[must_use]
    pub const fn assurance(&self) -> SignatureAssurance {
        SignatureAssurance::ExactTracked
    }

    /// Stable wire for persistence and differential fixtures.
    #[must_use]
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(PATTERN_MAGIC);
        bytes.extend_from_slice(&PATTERN_SCHEMA.to_be_bytes());
        bytes.extend_from_slice(self.catalog.id.as_bytes());
        bytes.push(self.status as u8);
        bytes.push(self.catalog.maximum_order);
        bytes.push(self.catalog.maximum_loops);
        bytes.push(0);
        bytes.extend_from_slice(&self.estimated_work.to_be_bytes());
        bytes.extend_from_slice(&self.graph_count.to_be_bytes());
        bytes.extend_from_slice(&self.vertex_count.to_be_bytes());
        bytes.extend_from_slice(
            &u64::try_from(self.counts.len())
                .unwrap_or(u64::MAX)
                .to_be_bytes(),
        );
        for pattern in &self.counts {
            bytes.push(pattern.order);
            bytes.push(pattern.loop_order);
            bytes.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
            bytes.extend_from_slice(&pattern.count.to_be_bytes());
            append_framed(&mut bytes, &pattern.canonical_bytes);
        }
        bytes
    }
}

/// Collision-prone field compression of complete connected-pattern counts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PatternFieldFingerprint<F, const K: usize>
where
    F: Field,
{
    id: PatternFingerprintId,
    catalog_id: LoopPatternCatalogId,
    lanes: [F; K],
    graph_count: u64,
    vertex_count: u64,
}

/// Multiplicative field compression of connected-pattern multiplicities.
///
/// Unlike additive compression, this law does not reduce counts to parity in
/// characteristic two. Every lane evaluates the multiset polynomial of exact
/// canonical pattern descriptors at an independently derived offset. A zero
/// factor is retained explicitly so composition remains total.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PatternProductFingerprint<F, const K: usize>
where
    F: Field,
{
    id: PatternFingerprintId,
    catalog_id: LoopPatternCatalogId,
    nonzero_products: [F; K],
    zero_factor_counts: [u64; K],
    graph_count: u64,
    vertex_count: u64,
}

impl<F, const K: usize> PatternProductFingerprint<F, K>
where
    F: Field + CanonicalEncoding + Pow + StaticField,
{
    /// Compresses one complete exact profile as a multiset polynomial.
    ///
    /// # Errors
    ///
    /// Rejects incomplete profiles, empty lane sets, encoding failures and
    /// zero-factor counter overflow.
    pub fn from_profile<E>(
        profile: &ConnectedPatternProfile,
        encoder: &E,
    ) -> Result<Self, GraphError>
    where
        E: StructuralLaneEncoder<F, K>,
    {
        if profile.status != PatternAnalysisStatus::Complete {
            return Err(GraphError::PatternAnalysisIncomplete);
        }
        if K == 0 {
            return Err(GraphError::InvalidPatternFingerprint);
        }
        let offsets = encoder.encode_lanes(b"pattern/product-offset/v1")?;
        let mut nonzero_products = [F::ONE; K];
        let mut zero_factor_counts = [0_u64; K];
        for pattern in &profile.counts {
            let encoded = encoder.encode_lanes(&pattern.canonical_bytes)?;
            for lane in 0..K {
                let factor = encoded[lane].add(offsets[lane]);
                if factor.is_zero() {
                    zero_factor_counts[lane] = zero_factor_counts[lane]
                        .checked_add(pattern.count)
                        .ok_or(GraphError::GraphTooLarge)?;
                } else {
                    nonzero_products[lane] =
                        nonzero_products[lane].mul(factor.pow(&[pattern.count]));
                }
            }
        }
        let mut hasher = Sha256::new();
        hasher.update(b"microfield/pattern-product-fingerprint/v1\0");
        hasher.update(F::spec().field_id().as_bytes());
        hasher.update(profile.catalog.id.as_bytes());
        hasher.update(encoder.encoder_id().as_bytes());
        hasher.update(u64::try_from(K).unwrap_or(u64::MAX).to_be_bytes());
        Ok(Self {
            id: PatternFingerprintId(hasher.finalize().into()),
            catalog_id: profile.catalog.id,
            nonzero_products,
            zero_factor_counts,
            graph_count: profile.graph_count,
            vertex_count: profile.vertex_count,
        })
    }

    /// Multiplies fingerprints representing a disjoint union.
    ///
    /// # Errors
    ///
    /// Rejects profile drift and stable counter overflow.
    pub fn combine_disjoint(&self, other: &Self) -> Result<Self, GraphError> {
        if self.id != other.id {
            return Err(GraphError::PatternProfileMismatch);
        }
        let mut zero_factor_counts = [0_u64; K];
        for (output, (left, right)) in zero_factor_counts.iter_mut().zip(
            self.zero_factor_counts
                .iter()
                .zip(&other.zero_factor_counts),
        ) {
            *output = left.checked_add(*right).ok_or(GraphError::GraphTooLarge)?;
        }
        Ok(Self {
            nonzero_products: core::array::from_fn(|lane| {
                self.nonzero_products[lane].mul(other.nonzero_products[lane])
            }),
            zero_factor_counts,
            graph_count: self
                .graph_count
                .checked_add(other.graph_count)
                .ok_or(GraphError::GraphTooLarge)?,
            vertex_count: self
                .vertex_count
                .checked_add(other.vertex_count)
                .ok_or(GraphError::GraphTooLarge)?,
            ..self.clone()
        })
    }

    /// Complete product-compression identity.
    #[must_use]
    pub const fn id(&self) -> PatternFingerprintId {
        self.id
    }

    /// Exact catalog compressed by this state.
    #[must_use]
    pub const fn catalog_id(&self) -> LoopPatternCatalogId {
        self.catalog_id
    }

    /// Product excluding explicitly tracked zero factors.
    #[must_use]
    pub const fn nonzero_products(&self) -> &[F; K] {
        &self.nonzero_products
    }

    /// Exact number of zero factors in every evaluation lane.
    #[must_use]
    pub const fn zero_factor_counts(&self) -> &[u64; K] {
        &self.zero_factor_counts
    }

    /// Evaluated product, including the effect of zero factors.
    #[must_use]
    pub fn evaluated_products(&self) -> [F; K] {
        core::array::from_fn(|lane| {
            if self.zero_factor_counts[lane] == 0 {
                self.nonzero_products[lane]
            } else {
                F::ZERO
            }
        })
    }

    /// Multiplicative finite-field compression remains a fingerprint.
    #[must_use]
    pub const fn assurance(&self) -> SignatureAssurance {
        SignatureAssurance::Fingerprint
    }

    /// Stable field-specific wire.
    #[must_use]
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let repr_len = F::ZERO.to_canonical().as_ref().len();
        let mut bytes = Vec::with_capacity(110 + K.saturating_mul(repr_len.saturating_add(8)));
        bytes.extend_from_slice(PRODUCT_FINGERPRINT_MAGIC);
        bytes.extend_from_slice(&PRODUCT_FINGERPRINT_SCHEMA.to_be_bytes());
        bytes.extend_from_slice(self.id.as_bytes());
        bytes.extend_from_slice(F::spec().field_id().as_bytes());
        bytes.extend_from_slice(self.catalog_id.as_bytes());
        bytes.extend_from_slice(&self.graph_count.to_be_bytes());
        bytes.extend_from_slice(&self.vertex_count.to_be_bytes());
        bytes.extend_from_slice(&u64::try_from(K).unwrap_or(u64::MAX).to_be_bytes());
        for lane in 0..K {
            bytes.extend_from_slice(&self.zero_factor_counts[lane].to_be_bytes());
            bytes.extend_from_slice(self.nonzero_products[lane].to_canonical().as_ref());
        }
        bytes
    }
}

impl<F, const K: usize> PatternFieldFingerprint<F, K>
where
    F: Field + CanonicalEncoding + StaticField,
{
    /// Compresses one complete exact pattern profile with independent lanes.
    ///
    /// # Errors
    ///
    /// Rejects incomplete input and lane-encoding failures.
    pub fn from_profile<E>(
        profile: &ConnectedPatternProfile,
        encoder: &E,
    ) -> Result<Self, GraphError>
    where
        E: StructuralLaneEncoder<F, K>,
    {
        if profile.status != PatternAnalysisStatus::Complete {
            return Err(GraphError::PatternAnalysisIncomplete);
        }
        if K == 0 {
            return Err(GraphError::InvalidPatternFingerprint);
        }
        let mut lanes = [F::ZERO; K];
        for pattern in &profile.counts {
            let encoded = encoder.encode_lanes(&pattern.canonical_bytes)?;
            for index in 0..K {
                lanes[index] = lanes[index].add(scale_by_u64(encoded[index], pattern.count));
            }
        }
        let mut hasher = Sha256::new();
        hasher.update(b"microfield/pattern-field-fingerprint/v1\0");
        hasher.update(F::spec().field_id().as_bytes());
        hasher.update(profile.catalog.id.as_bytes());
        hasher.update(encoder.encoder_id().as_bytes());
        hasher.update(u64::try_from(K).unwrap_or(u64::MAX).to_be_bytes());
        Ok(Self {
            id: PatternFingerprintId(hasher.finalize().into()),
            catalog_id: profile.catalog.id,
            lanes,
            graph_count: profile.graph_count,
            vertex_count: profile.vertex_count,
        })
    }

    /// Adds fingerprints representing a disjoint union.
    ///
    /// # Errors
    ///
    /// Rejects identity drift and counter overflow.
    pub fn combine_disjoint(&self, other: &Self) -> Result<Self, GraphError> {
        if self.id != other.id {
            return Err(GraphError::PatternProfileMismatch);
        }
        Ok(Self {
            lanes: core::array::from_fn(|index| self.lanes[index].add(other.lanes[index])),
            graph_count: self
                .graph_count
                .checked_add(other.graph_count)
                .ok_or(GraphError::GraphTooLarge)?,
            vertex_count: self
                .vertex_count
                .checked_add(other.vertex_count)
                .ok_or(GraphError::GraphTooLarge)?,
            ..self.clone()
        })
    }

    /// Complete field/catalog/lane-encoder identity.
    #[must_use]
    pub const fn id(&self) -> PatternFingerprintId {
        self.id
    }

    /// Exact catalog compressed by this state.
    #[must_use]
    pub const fn catalog_id(&self) -> LoopPatternCatalogId {
        self.catalog_id
    }

    /// Independent field lanes.
    #[must_use]
    pub const fn lanes(&self) -> &[F; K] {
        &self.lanes
    }

    /// Field compression is always a fingerprint.
    #[must_use]
    pub const fn assurance(&self) -> SignatureAssurance {
        SignatureAssurance::Fingerprint
    }

    /// Stable field-specific wire.
    #[must_use]
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let repr_len = F::ZERO.to_canonical().as_ref().len();
        let mut bytes = Vec::with_capacity(110 + K.saturating_mul(repr_len));
        bytes.extend_from_slice(FINGERPRINT_MAGIC);
        bytes.extend_from_slice(&FINGERPRINT_SCHEMA.to_be_bytes());
        bytes.extend_from_slice(self.id.as_bytes());
        bytes.extend_from_slice(F::spec().field_id().as_bytes());
        bytes.extend_from_slice(self.catalog_id.as_bytes());
        bytes.extend_from_slice(&self.graph_count.to_be_bytes());
        bytes.extend_from_slice(&self.vertex_count.to_be_bytes());
        bytes.extend_from_slice(&u64::try_from(K).unwrap_or(u64::MAX).to_be_bytes());
        for lane in self.lanes {
            bytes.extend_from_slice(lane.to_canonical().as_ref());
        }
        bytes
    }
}

fn estimate_work(vertex_count: usize, maximum_order: u8) -> Result<u64, GraphError> {
    let n = u64::try_from(vertex_count).map_err(|_| GraphError::GraphTooLarge)?;
    let mut total = 0_u64;
    for order in 1..=u64::from(maximum_order).min(n) {
        let subsets = choose(n, order)?;
        let permutations = factorial(order)?;
        total = total
            .checked_add(
                subsets
                    .checked_mul(permutations)
                    .ok_or(GraphError::GraphTooLarge)?,
            )
            .ok_or(GraphError::GraphTooLarge)?;
    }
    Ok(total)
}

fn choose(n: u64, k: u64) -> Result<u64, GraphError> {
    let k = k.min(n - k);
    let mut result = 1_u64;
    for index in 0..k {
        result = result
            .checked_mul(n - index)
            .ok_or(GraphError::GraphTooLarge)?
            / (index + 1);
    }
    Ok(result)
}

fn factorial(value: u64) -> Result<u64, GraphError> {
    (1..=value).try_fold(1_u64, |product, factor| {
        product.checked_mul(factor).ok_or(GraphError::GraphTooLarge)
    })
}

fn enumerate_subsets<F>(
    vertex_count: usize,
    required: usize,
    next: usize,
    selected: &mut Vec<usize>,
    visit: &mut F,
) -> Result<(), GraphError>
where
    F: FnMut(&[usize]) -> Result<(), GraphError>,
{
    if selected.len() == required {
        return visit(selected);
    }
    let remaining = required - selected.len();
    let Some(last_start) = vertex_count.checked_sub(remaining) else {
        return Ok(());
    };
    for vertex in next..=last_start {
        selected.push(vertex);
        enumerate_subsets(vertex_count, required, vertex + 1, selected, visit)?;
        selected.pop();
    }
    Ok(())
}

fn connected_loop_order(
    graph: &IncidenceGraph,
    selected: &[usize],
) -> Result<Option<u8>, GraphError> {
    let order = selected.len();
    let mut adjacency = vec![false; order * order];
    let mut self_loops = 0_usize;
    for (local_source, &source) in selected.iter().enumerate() {
        for incidence in graph.outgoing(VertexId::new(source)) {
            let target = incidence.neighbor().index();
            let Ok(local_target) = selected.binary_search(&target) else {
                continue;
            };
            if local_source == local_target {
                self_loops = self_loops.checked_add(1).ok_or(GraphError::GraphTooLarge)?;
            } else {
                adjacency[local_source * order + local_target] = true;
                adjacency[local_target * order + local_source] = true;
            }
        }
    }
    if order > 1 {
        let mut seen = vec![false; order];
        let mut stack = vec![0_usize];
        seen[0] = true;
        while let Some(source) = stack.pop() {
            for target in 0..order {
                if adjacency[source * order + target] && !seen[target] {
                    seen[target] = true;
                    stack.push(target);
                }
            }
        }
        if seen.iter().any(|seen| !seen) {
            return Ok(None);
        }
    }
    let support_edges = (0..order)
        .flat_map(|left| (left + 1..order).map(move |right| (left, right)))
        .filter(|(left, right)| adjacency[left * order + right])
        .count();
    let loops = support_edges
        .checked_add(self_loops)
        .and_then(|value| value.checked_add(1))
        .and_then(|value| value.checked_sub(order))
        .ok_or(GraphError::GraphTooLarge)?;
    Ok(Some(
        u8::try_from(loops).map_err(|_| GraphError::GraphTooLarge)?,
    ))
}

fn canonical_pattern_bytes(
    graph: &IncidenceGraph,
    selected: &[usize],
    loop_order: u8,
) -> Result<Vec<u8>, GraphError> {
    let mut permutation = (0..selected.len()).collect::<Vec<_>>();
    let mut minimum: Option<Vec<u8>> = None;
    for_each_permutation(&mut permutation, 0, &mut |order| {
        let candidate = encode_pattern_order(graph, selected, order, loop_order)?;
        if minimum
            .as_ref()
            .is_none_or(|current| candidate.as_slice() < current.as_slice())
        {
            minimum = Some(candidate);
        }
        Ok(())
    })?;
    minimum.ok_or(GraphError::CanonicalizationInvariantViolation)
}

fn for_each_permutation<F>(
    values: &mut [usize],
    start: usize,
    visit: &mut F,
) -> Result<(), GraphError>
where
    F: FnMut(&[usize]) -> Result<(), GraphError>,
{
    if start == values.len() {
        return visit(values);
    }
    for index in start..values.len() {
        values.swap(start, index);
        for_each_permutation(values, start + 1, visit)?;
        values.swap(start, index);
    }
    Ok(())
}

fn encode_pattern_order(
    graph: &IncidenceGraph,
    selected: &[usize],
    canonical_to_local: &[usize],
    loop_order: u8,
) -> Result<Vec<u8>, GraphError> {
    let order = selected.len();
    let mut local_to_canonical = vec![0_usize; order];
    for (canonical, &local) in canonical_to_local.iter().enumerate() {
        local_to_canonical[local] = canonical;
    }
    let mut bytes = Vec::new();
    bytes.push(u8::try_from(order).map_err(|_| GraphError::GraphTooLarge)?);
    bytes.push(loop_order);
    for &local in canonical_to_local {
        let vertex = VertexId::new(selected[local]);
        bytes.push(graph.vertex_kind(vertex) as u8);
        append_framed(&mut bytes, graph.vertex_label(vertex));
    }
    let mut arcs = Vec::new();
    for (local_source, &source) in selected.iter().enumerate() {
        for incidence in graph.outgoing(VertexId::new(source)) {
            let Ok(local_target) = selected.binary_search(&incidence.neighbor().index()) else {
                continue;
            };
            let descriptor = graph.relation(incidence.relation());
            let mut arc = Vec::new();
            append_u64(&mut arc, local_to_canonical[local_source])?;
            append_u64(&mut arc, local_to_canonical[local_target])?;
            append_framed(&mut arc, descriptor.relation());
            append_framed(&mut arc, descriptor.role());
            arc.extend_from_slice(&incidence.multiplicity().to_be_bytes());
            arcs.push(arc);
        }
    }
    arcs.sort_unstable();
    append_u64(&mut bytes, arcs.len())?;
    for arc in arcs {
        append_framed(&mut bytes, &arc);
    }
    Ok(bytes)
}

fn append_u64(output: &mut Vec<u8>, value: usize) -> Result<(), GraphError> {
    output.extend_from_slice(
        &u64::try_from(value)
            .map_err(|_| GraphError::GraphTooLarge)?
            .to_be_bytes(),
    );
    Ok(())
}

fn append_framed(output: &mut Vec<u8>, bytes: &[u8]) {
    output.extend_from_slice(&u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_be_bytes());
    output.extend_from_slice(bytes);
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
