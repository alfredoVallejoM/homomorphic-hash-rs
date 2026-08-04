//! Deterministic discovery/holdout campaign for G11 invariant channels.

use std::{collections::BTreeMap, fs, path::Path};

use homomorphic_hash_rs::{
    CellMomentProfile, ClosedWalkQueryPlan, DegreeHistogramProfile,
    DomainSeparatedHashToFieldEncoder, FastGraphLabeler, IncidenceGraph, LoopPatternCatalog,
    PatternFieldFingerprint, PatternProductFingerprint, PrimeIntegerEncoder, RefinementProfile,
    RelationalClosedWalkProfile, RelationalMatrixProfile, RelationalThetaProfile,
};
use microfield::{
    CanonicalEncoding, Field, Fp251V1, FpGoldilocks64V1, Gf2_256HhV1, Invert, Pow, StaticField,
};
use serde::Serialize;
use sha2::{Digest as _, Sha256};

use crate::{
    graphs::{g11_adversarial_pairs, parse_graph6},
    model::{GraphCollisionProfile, ValidationManifest},
};

const PROFILE: [u8; 32] = [0x47; 32];
const DOMAIN: u64 = 0x4636_4731_3157_4c01;
const SIMPLE_N8_SHA256: &str = "546a249902101c97d3aa590f93e53366854bd0a6f405aa59bdb32d25c57f845a";
const CHANNEL_NAMES: [&str; 19] = [
    "baseline/f251-fast-1wl",
    "degree-histogram/f251-multiset-k3",
    "moments/goldilocks-k3-d4",
    "patterns/exact-induced-l0-l3",
    "patterns/additive-f251-k3",
    "patterns/additive-goldilocks-k3",
    "patterns/additive-gf2-256-k3",
    "patterns/product-f251-k3",
    "patterns/product-goldilocks-k3",
    "patterns/product-gf2-256-k3",
    "matrix/f251-k3-trace6-char",
    "matrix/goldilocks-k3-trace6-char",
    "matrix/gf2-256-k3-trace6-char",
    "theta-rg2/f251-k3",
    "theta-rg2/goldilocks-k3",
    "theta-rg2/gf2-256-k3",
    "closed-walk/goldilocks-k3-lengths-8-16-64-1e12",
    "bundle/goldilocks-pattern-product-plus-matrix-plus-theta",
    "bundle/1wl-plus-goldilocks-pattern-product-plus-matrix-plus-theta",
];

/// Reproducible G11 evaluation with a frozen, pre-selection holdout split.
#[derive(Clone, Debug, Serialize)]
pub struct G11CampaignReport {
    /// Report schema.
    pub schema_version: u32,
    /// Frozen corpus authority.
    pub oracle: String,
    /// SHA-256 of the exact input corpus.
    pub corpus_sha256: String,
    /// Rule fixed before observing channel results.
    pub split_rule: String,
    /// Channel assurance shared by the report.
    pub assurance_note: String,
    /// Discovery half, available for future profile selection.
    pub discovery: G11SplitReport,
    /// Untuned holdout half.
    pub holdout: G11SplitReport,
    /// Frozen adversarial families evaluated separately.
    pub adversarial: Vec<G11AdversarialResult>,
    /// Evidence-based continuation decision; not a completeness claim.
    pub decision: String,
}

/// Collision statistics for one immutable corpus split.
#[derive(Clone, Debug, Serialize)]
pub struct G11SplitReport {
    /// Split label.
    pub split: String,
    /// Number of pairwise non-isomorphic oracle representatives.
    pub graph_count: u64,
    /// Number of independently reversed relabeling checks.
    pub relabeling_checks: u64,
    /// One collision profile per frozen channel.
    pub collision_profiles: Vec<GraphCollisionProfile>,
}

/// Per-channel separation result on one known non-isomorphic pair.
#[derive(Clone, Debug, Serialize)]
pub struct G11AdversarialResult {
    /// Frozen family name.
    pub family: String,
    /// Whether each channel differs between the pair.
    pub channel_distinguishes: BTreeMap<String, bool>,
}

#[derive(Default)]
struct SplitBuckets {
    graph_count: u64,
    relabeling_checks: u64,
    channels: Vec<BTreeMap<Vec<u8>, u64>>,
}

struct FieldChannels {
    additive_pattern: Vec<u8>,
    product_pattern: Vec<u8>,
    matrix: Vec<u8>,
    theta: Vec<u8>,
}

impl SplitBuckets {
    fn new() -> Self {
        Self {
            channels: (0..CHANNEL_NAMES.len()).map(|_| BTreeMap::new()).collect(),
            ..Self::default()
        }
    }

    fn insert(&mut self, outputs: &[Vec<u8>]) -> Result<(), String> {
        if outputs.len() != self.channels.len() {
            return Err("G11 channel count drift".into());
        }
        self.graph_count = self
            .graph_count
            .checked_add(1)
            .ok_or("G11 graph counter overflow")?;
        for (buckets, output) in self.channels.iter_mut().zip(outputs) {
            let count = buckets.entry(output.clone()).or_default();
            *count = count.checked_add(1).ok_or("G11 bucket overflow")?;
        }
        Ok(())
    }

    fn report(self, split: &str) -> G11SplitReport {
        G11SplitReport {
            split: split.into(),
            graph_count: self.graph_count,
            relabeling_checks: self.relabeling_checks,
            collision_profiles: CHANNEL_NAMES
                .iter()
                .zip(self.channels)
                .map(|(&name, buckets)| collision_profile(name, &buckets))
                .collect(),
        }
    }
}

/// Executes all frozen G11 channels on the order-eight oracle corpus.
pub fn run_campaign(
    manifest: &ValidationManifest,
    root: &Path,
) -> Result<G11CampaignReport, String> {
    let path = root.join("validation/f6/corpora/simple-n8.g6");
    let bytes = fs::read(&path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let digest = hex(&Sha256::digest(&bytes));
    if digest != SIMPLE_N8_SHA256 {
        return Err(format!(
            "simple-n8 corpus digest drift: {digest}, expected {SIMPLE_N8_SHA256}"
        ));
    }
    let records = std::str::from_utf8(&bytes)
        .map_err(|error| error.to_string())?
        .lines()
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if records.len() != 12_346 {
        return Err(format!(
            "expected 12,346 order-eight graphs, found {}",
            records.len()
        ));
    }
    let labeler = FastGraphLabeler::<Fp251V1, _, 3>::new(
        PrimeIntegerEncoder::new(DOMAIN),
        RefinementProfile::Fast {
            rounds: manifest.graph.rounds,
        },
    )
    .map_err(debug_error)?;
    let catalog = LoopPatternCatalog::l0_to_l3();
    let mut discovery = SplitBuckets::new();
    let mut holdout = SplitBuckets::new();
    for (index, record) in records.into_iter().enumerate() {
        let graph = parse_graph6(record)?;
        let outputs = evaluate_channels(&labeler, catalog, &graph)?;
        let record_digest = Sha256::digest(record.as_bytes());
        let split = if record_digest[0] & 1 == 0 {
            &mut discovery
        } else {
            &mut holdout
        };
        split.insert(&outputs)?;
        if index % 257 == 0 {
            let reversed = reverse_relabel(&graph)?;
            if evaluate_channels(&labeler, catalog, &reversed)? != outputs {
                return Err(format!("G11 relabeling invariance failed for {record}"));
            }
            split.relabeling_checks = split
                .relabeling_checks
                .checked_add(1)
                .ok_or("G11 relabeling counter overflow")?;
        }
    }

    let adversarial = g11_adversarial_pairs()?
        .into_iter()
        .map(|(family, left, right)| {
            let left = evaluate_channels(&labeler, catalog, &left)?;
            let right = evaluate_channels(&labeler, catalog, &right)?;
            Ok(G11AdversarialResult {
                family: family.into(),
                channel_distinguishes: CHANNEL_NAMES
                    .iter()
                    .enumerate()
                    .map(|(index, &name)| (name.into(), left[index] != right[index]))
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(G11CampaignReport {
        schema_version: 1,
        oracle: "SageMath 10.7 graphs.nauty_geng(8), one representative per isomorphism class"
            .into(),
        corpus_sha256: digest,
        split_rule: "SHA-256(graph6)[0] bit 0: 0=discovery, 1=holdout".into(),
        assurance_note: "Degree histograms and induced-pattern counts are exact only for their declared bounded data; every finite-field correlation, compression, moment and matrix evaluation is a fingerprint. No equality proves graph isomorphism.".into(),
        discovery: discovery.report("discovery"),
        holdout: holdout.report("holdout"),
        adversarial,
        decision: "Retain independent lane encoding, bounded induced patterns, RG1 matrix channels and the frozen RG2 theta prototype as non-authoritative routing/rejection evidence. Continue homomorphism/resolvent research only if it beats these frozen baselines on a future untouched corpus.".into(),
    })
}

fn evaluate_channels(
    labeler: &FastGraphLabeler<Fp251V1, PrimeIntegerEncoder, 3>,
    catalog: LoopPatternCatalog,
    graph: &IncidenceGraph,
) -> Result<Vec<Vec<u8>>, String> {
    let exact_patterns = catalog.analyze(graph, u64::MAX).map_err(debug_error)?;
    let f251 = field_channels::<Fp251V1>(graph, &exact_patterns)?;
    let goldilocks = field_channels::<FpGoldilocks64V1>(graph, &exact_patterns)?;
    let binary = field_channels::<Gf2_256HhV1>(graph, &exact_patterns)?;
    let fast = labeler
        .analyze(graph)
        .map_err(debug_error)?
        .signature()
        .to_canonical_bytes();
    let degree = DegreeHistogramProfile::<Fp251V1, _, 3>::analyze(
        graph,
        PrimeIntegerEncoder::new(DOMAIN ^ 0x4445_4752_4545),
        [
            Fp251V1::ONE,
            Fp251V1::from_u64_mod(2),
            Fp251V1::from_u64_mod(3),
        ],
    )
    .map_err(debug_error)?
    .to_canonical_bytes();
    let moments_encoder = DomainSeparatedHashToFieldEncoder::<3>::new(PROFILE, 31);
    let moments =
        CellMomentProfile::<FpGoldilocks64V1, 3, 4>::analyze_initial(graph, &moments_encoder)
            .map_err(debug_error)?
            .to_canonical_bytes();
    let walk_encoder = DomainSeparatedHashToFieldEncoder::<3>::new(PROFILE, 35);
    let walk_plan =
        ClosedWalkQueryPlan::new(vec![8, 16, 64, 1_000_000_000_000]).map_err(debug_error)?;
    let closed_walk = RelationalClosedWalkProfile::<FpGoldilocks64V1, 3>::analyze(
        graph,
        walk_plan,
        &walk_encoder,
        u64::MAX,
    )
    .map_err(debug_error)?
    .to_canonical_bytes();
    let mut goldilocks_bundle = goldilocks.product_pattern.clone();
    goldilocks_bundle.extend_from_slice(&goldilocks.matrix);
    goldilocks_bundle.extend_from_slice(&goldilocks.theta);
    let mut complete_bundle = fast.clone();
    complete_bundle.extend_from_slice(&goldilocks_bundle);
    Ok(vec![
        fast,
        degree,
        moments,
        exact_patterns.to_canonical_bytes(),
        f251.additive_pattern,
        goldilocks.additive_pattern,
        binary.additive_pattern,
        f251.product_pattern,
        goldilocks.product_pattern,
        binary.product_pattern,
        f251.matrix,
        goldilocks.matrix,
        binary.matrix,
        f251.theta,
        goldilocks.theta,
        binary.theta,
        closed_walk,
        goldilocks_bundle,
        complete_bundle,
    ])
}

fn field_channels<F>(
    graph: &IncidenceGraph,
    patterns: &homomorphic_hash_rs::ConnectedPatternProfile,
) -> Result<FieldChannels, String>
where
    F: Field + CanonicalEncoding + Invert + Pow + StaticField,
{
    let pattern_encoder = DomainSeparatedHashToFieldEncoder::<3>::new(PROFILE, 32);
    let matrix_encoder = DomainSeparatedHashToFieldEncoder::<3>::new(PROFILE, 33);
    let theta_encoder = DomainSeparatedHashToFieldEncoder::<3>::new(PROFILE, 34);
    let pattern = PatternFieldFingerprint::<F, 3>::from_profile(patterns, &pattern_encoder)
        .map_err(debug_error)?
        .to_canonical_bytes();
    let product = PatternProductFingerprint::<F, 3>::from_profile(patterns, &pattern_encoder)
        .map_err(debug_error)?
        .to_canonical_bytes();
    let matrix = RelationalMatrixProfile::<F, 3>::analyze(graph, 6, &matrix_encoder, u64::MAX)
        .map_err(debug_error)?
        .to_canonical_bytes();
    let theta = RelationalThetaProfile::<F, 3>::analyze(graph, &theta_encoder, u64::MAX)
        .map_err(debug_error)?
        .to_canonical_bytes();
    Ok(FieldChannels {
        additive_pattern: pattern,
        product_pattern: product,
        matrix,
        theta,
    })
}

fn collision_profile(name: &str, buckets: &BTreeMap<Vec<u8>, u64>) -> GraphCollisionProfile {
    GraphCollisionProfile {
        tier: name.into(),
        distinct_outputs: buckets.len() as u64,
        collision_buckets: buckets.values().filter(|&&count| count > 1).count() as u64,
        colliding_graphs: buckets.values().filter(|&&count| count > 1).copied().sum(),
        colliding_pairs: buckets
            .values()
            .map(|&count| count.saturating_mul(count.saturating_sub(1)) / 2)
            .sum(),
        maximum_bucket_size: buckets.values().copied().max().unwrap_or(0),
    }
}

fn reverse_relabel(graph: &IncidenceGraph) -> Result<IncidenceGraph, String> {
    use homomorphic_hash_rs::{IncidenceGraphBuilder, VertexId};

    let count = graph.vertex_count();
    let mut builder = IncidenceGraphBuilder::new();
    for new in 0..count {
        let old = VertexId::new(count - new - 1);
        builder.add_typed_vertex(graph.vertex_kind(old), graph.vertex_label(old).to_vec());
    }
    for source in 0..count {
        for incidence in graph.outgoing(VertexId::new(source)) {
            let descriptor = graph.relation(incidence.relation());
            builder
                .add_directed_relation(
                    VertexId::new(count - source - 1),
                    VertexId::new(count - incidence.neighbor().index() - 1),
                    descriptor.relation().to_vec(),
                    descriptor.role().to_vec(),
                    incidence.multiplicity(),
                )
                .map_err(debug_error)?;
        }
    }
    builder.build().map_err(debug_error)
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing into a string cannot fail");
    }
    output
}

fn debug_error(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use homomorphic_hash_rs::IncidenceGraphBuilder;

    #[test]
    fn channel_order_is_stable_and_relabeling_invariant() {
        let mut builder = IncidenceGraphBuilder::new();
        let vertices = (0..4)
            .map(|_| builder.add_vertex(Vec::new()))
            .collect::<Vec<_>>();
        for index in 0..4 {
            builder
                .add_undirected_relation(
                    vertices[index],
                    vertices[(index + 1) % 4],
                    b"edge".to_vec(),
                    Vec::new(),
                    1,
                )
                .unwrap();
        }
        let graph = builder.build().unwrap();
        let labeler = FastGraphLabeler::<Fp251V1, _, 3>::new(
            PrimeIntegerEncoder::new(DOMAIN),
            RefinementProfile::Fast { rounds: 4 },
        )
        .unwrap();
        let catalog = LoopPatternCatalog::l0_to_l3();
        let original = evaluate_channels(&labeler, catalog, &graph).unwrap();
        let reversed =
            evaluate_channels(&labeler, catalog, &reverse_relabel(&graph).unwrap()).unwrap();
        assert_eq!(original.len(), CHANNEL_NAMES.len());
        assert_eq!(original, reversed);
    }
}
