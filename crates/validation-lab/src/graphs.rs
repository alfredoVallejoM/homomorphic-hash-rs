use std::{collections::BTreeMap, fs, path::Path};

use homomorphic_hash_rs::{
    BinaryPolynomialEncoder, CanonicalSearchBudget, ExactCanonicalOutcome, FastGraphLabeler,
    GraphDiscriminationPolicy, HyperedgeIncidence, IncidenceGraph, IncidenceGraphBuilder,
    IncrementalGraphWorkspace, PrimeIntegerEncoder, RefinementProfile, VertexId,
};
use microfield::{Fp251V1, Gf2_256HhV1};
use sha2::{Digest, Sha256};

use crate::model::{
    AdversarialFamilyResult, AppliedVerticalResult, GraphCampaignReport, GraphCollisionExample,
    GraphCollisionProfile, IncrementalCurvePoint, ValidationManifest,
};

const DOMAIN: u64 = 0x4636_5641_4c47_0001;
const SIMPLE_N8_SHA256: &str = "546a249902101c97d3aa590f93e53366854bd0a6f405aa59bdb32d25c57f845a";
const ADVERSARIAL_SHA256: &str = "572d093bc818f4f040e467847c6a0bbec99bf5f8d6eef263ef351536ff491484";

pub fn run_campaign(
    manifest: &ValidationManifest,
    root: &Path,
) -> Result<GraphCampaignReport, String> {
    let corpus_path = root.join("validation/f6/corpora/simple-n8.g6");
    let corpus_bytes = fs::read(&corpus_path)
        .map_err(|error| format!("read {}: {error}", corpus_path.display()))?;
    let corpus_sha256 = hex(&Sha256::digest(&corpus_bytes));
    if corpus_sha256 != SIMPLE_N8_SHA256 {
        return Err(format!(
            "simple-n8 corpus digest drift: {corpus_sha256}, expected {SIMPLE_N8_SHA256}"
        ));
    }
    validate_adversarial_oracle(root)?;
    let corpus = std::str::from_utf8(&corpus_bytes)
        .map_err(|error| error.to_string())?
        .lines()
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if corpus.len() != 12_346 {
        return Err(format!(
            "expected 12,346 non-isomorphic order-8 graphs, found {}",
            corpus.len()
        ));
    }

    let labeler = labeler(manifest)?;
    let exact_budget = CanonicalSearchBudget::new(manifest.graph.exact_node_budget)
        .with_max_retained_state_cells(manifest.graph.exact_retained_state_cells);
    let binary_labeler = FastGraphLabeler::<Gf2_256HhV1, _, 3>::new(
        BinaryPolynomialEncoder::new(DOMAIN),
        RefinementProfile::Fast {
            rounds: manifest.graph.rounds,
        },
    )
    .map_err(debug_error)?;
    let mut fast = BTreeMap::<Vec<u8>, Vec<&str>>::new();
    let mut hybrid = BTreeMap::<Vec<u8>, Vec<&str>>::new();
    let mut global = BTreeMap::<Vec<u8>, Vec<&str>>::new();
    let mut adaptive = BTreeMap::<Vec<u8>, Vec<&str>>::new();
    let mut multi_field = BTreeMap::<Vec<u8>, Vec<&str>>::new();
    let mut relabeling_checks = 0_u64;
    for (index, &encoded) in corpus.iter().enumerate() {
        let graph = parse_graph6(encoded)?;
        let analysis = labeler.analyze(&graph).map_err(debug_error)?;
        fast.entry(analysis.signature().to_canonical_bytes())
            .or_default()
            .push(encoded);
        let hybrid_analysis = labeler.analyze_hybrid(&graph).map_err(debug_error)?;
        hybrid
            .entry(hybrid_analysis.invariant_digest().as_bytes().to_vec())
            .or_default()
            .push(encoded);
        let v2 = labeler
            .analyze_discriminating(&graph, GraphDiscriminationPolicy::GlobalLinear)
            .map_err(debug_error)?;
        global
            .entry(v2.digest().as_bytes().to_vec())
            .or_default()
            .push(encoded);
        let adaptive_v2 = labeler
            .analyze_discriminating(&graph, GraphDiscriminationPolicy::adaptive())
            .map_err(debug_error)?;
        adaptive
            .entry(adaptive_v2.digest().as_bytes().to_vec())
            .or_default()
            .push(encoded);
        let binary_analysis = binary_labeler.analyze(&graph).map_err(debug_error)?;
        let mut bundle = analysis.signature().to_canonical_bytes();
        bundle.extend_from_slice(&binary_analysis.signature().to_canonical_bytes());
        multi_field.entry(bundle).or_default().push(encoded);

        // Every 97th class exercises invariance without making the full run
        // quadratic in graph construction.
        if index % 97 == 0 {
            let permuted = relabel_reverse(&graph)?;
            let permuted_analysis = labeler.analyze_hybrid(&permuted).map_err(debug_error)?;
            if hybrid_analysis.structural().signature()
                != permuted_analysis.structural().signature()
                || hybrid_analysis.invariant_digest() != permuted_analysis.invariant_digest()
            {
                return Err(format!("relabeling invariance failed for graph6 {encoded}"));
            }
            relabeling_checks += 1;
        }
    }

    let minimum_fast_collision = fast.values().find(|bucket| bucket.len() > 1).map(|bucket| {
        let left = bucket[0];
        let right = bucket[1];
        let left_graph = parse_graph6(left).expect("validated graph6");
        let right_graph = parse_graph6(right).expect("validated graph6");
        let left_hybrid = labeler
            .analyze_hybrid(&left_graph)
            .expect("validated graph")
            .invariant_digest();
        let right_hybrid = labeler
            .analyze_hybrid(&right_graph)
            .expect("validated graph")
            .invariant_digest();
        let left_global = labeler
            .analyze_discriminating(&left_graph, GraphDiscriminationPolicy::GlobalLinear)
            .expect("validated graph")
            .digest();
        let right_global = labeler
            .analyze_discriminating(&right_graph, GraphDiscriminationPolicy::GlobalLinear)
            .expect("validated graph")
            .digest();
        let left_adaptive = labeler
            .analyze_discriminating(&left_graph, GraphDiscriminationPolicy::adaptive())
            .expect("validated graph")
            .digest();
        let right_adaptive = labeler
            .analyze_discriminating(&right_graph, GraphDiscriminationPolicy::adaptive())
            .expect("validated graph")
            .digest();
        let left_binary = binary_labeler
            .analyze(&left_graph)
            .expect("validated graph")
            .signature()
            .to_canonical_bytes();
        let right_binary = binary_labeler
            .analyze(&right_graph)
            .expect("validated graph")
            .signature()
            .to_canonical_bytes();
        GraphCollisionExample {
            left_graph6: left.into(),
            right_graph6: right.into(),
            escalated_hybrid_distinguishes: left_hybrid != right_hybrid,
            escalated_global_v2_distinguishes: left_global != right_global,
            escalated_adaptive_v2_distinguishes: left_adaptive != right_adaptive,
            escalated_multi_field_distinguishes: left_binary != right_binary,
            exact_distinguishes: exact_bytes(&labeler, &left_graph, exact_budget)
                .expect("validated graph")
                .zip(exact_bytes(&labeler, &right_graph, exact_budget).expect("validated graph"))
                .map(|(left, right)| left != right),
        }
    });

    Ok(GraphCampaignReport {
        oracle: "SageMath 10.7 graphs.nauty_geng(8), one representative per isomorphism class"
            .into(),
        corpus_sha256,
        graph_count: corpus.len() as u64,
        relabeling_checks,
        collision_profiles: vec![
            collision_profile("f251-fast-v1", &fast),
            collision_profile("f251-hybrid-v1", &hybrid),
            collision_profile("f251-global-v2", &global),
            collision_profile("f251-adaptive-motifs-v2", &adaptive),
            collision_profile("f251-plus-gf2-256-fast", &multi_field),
        ],
        minimum_fast_collision,
        adversarial_families: adversarial_results(&labeler, exact_budget)?,
        applied_verticals: applied_verticals(&labeler)?,
        incremental_work_curve: incremental_work_curve(&labeler, manifest.graph.rounds)?,
    })
}

fn validate_adversarial_oracle(root: &Path) -> Result<(), String> {
    let path = root.join("validation/f6/corpora/adversarial-oracle.json");
    let bytes = fs::read(&path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let digest = hex(&Sha256::digest(&bytes));
    if digest != ADVERSARIAL_SHA256 {
        return Err(format!(
            "adversarial oracle digest drift: {digest}, expected {ADVERSARIAL_SHA256}"
        ));
    }
    let oracle: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse {}: {error}", path.display()))?;
    let families = oracle["families"]
        .as_array()
        .ok_or("adversarial oracle has no families array")?;
    if oracle["schema_version"].as_u64() != Some(1)
        || families.len() != 3
        || families
            .iter()
            .any(|family| family["isomorphic"].as_bool() != Some(false))
    {
        return Err("adversarial Sage oracle does not certify three non-isomorphic pairs".into());
    }
    Ok(())
}

fn labeler(
    manifest: &ValidationManifest,
) -> Result<FastGraphLabeler<Fp251V1, PrimeIntegerEncoder, 3>, String> {
    FastGraphLabeler::new(
        PrimeIntegerEncoder::new(DOMAIN),
        RefinementProfile::Fast {
            rounds: manifest.graph.rounds,
        },
    )
    .map_err(debug_error)
}

fn collision_profile(tier: &str, buckets: &BTreeMap<Vec<u8>, Vec<&str>>) -> GraphCollisionProfile {
    GraphCollisionProfile {
        tier: tier.into(),
        distinct_outputs: buckets.len() as u64,
        collision_buckets: buckets.values().filter(|bucket| bucket.len() > 1).count() as u64,
        colliding_graphs: buckets
            .values()
            .filter(|bucket| bucket.len() > 1)
            .map(|bucket| bucket.len() as u64)
            .sum(),
        colliding_pairs: buckets
            .values()
            .map(|bucket| {
                let size = bucket.len() as u64;
                size.saturating_mul(size.saturating_sub(1)) / 2
            })
            .sum(),
        maximum_bucket_size: buckets.values().map(Vec::len).max().unwrap_or(0) as u64,
    }
}

/// Decodes the compact graph6 representation for simple graphs up to order 62.
pub fn parse_graph6(encoded: &str) -> Result<IncidenceGraph, String> {
    let encoded = encoded.strip_prefix(">>graph6<<").unwrap_or(encoded);
    let bytes = encoded.as_bytes();
    let Some(&header) = bytes.first() else {
        return Err("empty graph6 record".into());
    };
    if !(63..=125).contains(&header) || header == 126 {
        return Err("only graph6 order 0..62 is supported by the F6 harness".into());
    }
    let order = usize::from(header - 63);
    let needed_bits = order.saturating_mul(order.saturating_sub(1)) / 2;
    let payload = &bytes[1..];
    if payload.len() != needed_bits.div_ceil(6)
        || payload.iter().any(|byte| !(63..=126).contains(byte))
    {
        return Err("non-canonical graph6 payload length or character".into());
    }
    let bits = payload.iter().flat_map(|byte| {
        let value = byte - 63;
        (0..6).rev().map(move |shift| value & (1 << shift) != 0)
    });
    let mut builder = IncidenceGraphBuilder::new();
    let vertices: Vec<_> = (0..order).map(|_| builder.add_vertex(Vec::new())).collect();
    let mut bits = bits.take(needed_bits);
    for right in 1..order {
        for left in 0..right {
            if bits.next().ok_or("truncated graph6 bitstream")? {
                builder
                    .add_undirected_relation(
                        vertices[left],
                        vertices[right],
                        b"edge".to_vec(),
                        Vec::new(),
                        1,
                    )
                    .map_err(debug_error)?;
            }
        }
    }
    builder.build().map_err(debug_error)
}

fn relabel_reverse(graph: &IncidenceGraph) -> Result<IncidenceGraph, String> {
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

fn adversarial_results(
    labeler: &FastGraphLabeler<Fp251V1, PrimeIntegerEncoder, 3>,
    exact_budget: CanonicalSearchBudget,
) -> Result<Vec<AdversarialFamilyResult>, String> {
    let cycle = cycles(&[6])?;
    let triangles = cycles(&[3, 3])?;
    let rook = rook_graph()?;
    let shrikhande = shrikhande_graph()?;
    let cfi_even = cfi_k4(None)?;
    let cfi_odd = cfi_k4(Some(0))?;
    Ok(vec![
        compare_adversarial(
            labeler,
            "C6 versus 2C3",
            &cycle,
            &triangles,
            true,
            exact_budget,
        )?,
        compare_adversarial(
            labeler,
            "Shrikhande versus 4x4 rook (strongly regular)",
            &shrikhande,
            &rook,
            true,
            exact_budget,
        )?,
        compare_adversarial(
            labeler,
            "CFI(K4) even versus one twisted edge",
            &cfi_even,
            &cfi_odd,
            true,
            exact_budget,
        )?,
    ])
}

fn compare_adversarial(
    labeler: &FastGraphLabeler<Fp251V1, PrimeIntegerEncoder, 3>,
    family: &str,
    left: &IncidenceGraph,
    right: &IncidenceGraph,
    non_isomorphic: bool,
    exact_budget: CanonicalSearchBudget,
) -> Result<AdversarialFamilyResult, String> {
    let left_fast = labeler.analyze(left).map_err(debug_error)?;
    let right_fast = labeler.analyze(right).map_err(debug_error)?;
    let left_hybrid = labeler.analyze_hybrid(left).map_err(debug_error)?;
    let right_hybrid = labeler.analyze_hybrid(right).map_err(debug_error)?;
    let exact_distinguishes = exact_bytes(labeler, left, exact_budget)?
        .zip(exact_bytes(labeler, right, exact_budget)?)
        .map(|(left, right)| left != right);
    Ok(AdversarialFamilyResult {
        family: family.into(),
        non_isomorphic,
        fast_distinguishes: left_fast.signature() != right_fast.signature(),
        hybrid_distinguishes: left_hybrid.invariant_digest() != right_hybrid.invariant_digest(),
        exact_distinguishes,
    })
}

fn exact_bytes(
    labeler: &FastGraphLabeler<Fp251V1, PrimeIntegerEncoder, 3>,
    graph: &IncidenceGraph,
    budget: CanonicalSearchBudget,
) -> Result<Option<Vec<u8>>, String> {
    match labeler
        .canonicalize_exact(graph, budget)
        .map_err(debug_error)?
    {
        ExactCanonicalOutcome::Exact { form, .. } => Ok(Some(form.bytes().to_vec())),
        ExactCanonicalOutcome::BudgetExhausted { .. } => Ok(None),
    }
}

fn cycles(lengths: &[usize]) -> Result<IncidenceGraph, String> {
    let mut builder = IncidenceGraphBuilder::new();
    let total = lengths.iter().sum();
    let vertices: Vec<_> = (0..total).map(|_| builder.add_vertex(Vec::new())).collect();
    let mut offset = 0;
    for &length in lengths {
        for index in 0..length {
            builder
                .add_undirected_relation(
                    vertices[offset + index],
                    vertices[offset + (index + 1) % length],
                    b"edge".to_vec(),
                    Vec::new(),
                    1,
                )
                .map_err(debug_error)?;
        }
        offset += length;
    }
    builder.build().map_err(debug_error)
}

fn rook_graph() -> Result<IncidenceGraph, String> {
    graph_from_predicate(16, |left, right| {
        left / 4 == right / 4 || left % 4 == right % 4
    })
}

fn shrikhande_graph() -> Result<IncidenceGraph, String> {
    let generators = [(1, 0), (3, 0), (0, 1), (0, 3), (1, 1), (3, 3)];
    graph_from_predicate(16, |left, right| {
        let (lr, lc) = (left / 4, left % 4);
        let (rr, rc) = (right / 4, right % 4);
        generators.iter().any(|&(dr, dc)| {
            ((lr + dr) % 4, (lc + dc) % 4) == (rr, rc) || ((rr + dr) % 4, (rc + dc) % 4) == (lr, lc)
        })
    })
}

fn graph_from_predicate(
    order: usize,
    adjacent: impl Fn(usize, usize) -> bool,
) -> Result<IncidenceGraph, String> {
    let mut builder = IncidenceGraphBuilder::new();
    let vertices: Vec<_> = (0..order).map(|_| builder.add_vertex(Vec::new())).collect();
    for left in 0..order {
        for right in left + 1..order {
            if adjacent(left, right) {
                builder
                    .add_undirected_relation(
                        vertices[left],
                        vertices[right],
                        b"edge".to_vec(),
                        Vec::new(),
                        1,
                    )
                    .map_err(debug_error)?;
            }
        }
    }
    builder.build().map_err(debug_error)
}

fn cfi_k4(twisted_edge: Option<usize>) -> Result<IncidenceGraph, String> {
    let base_edges = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
    let incident: Vec<Vec<usize>> = (0..4)
        .map(|vertex| {
            base_edges
                .iter()
                .enumerate()
                .filter_map(|(edge, &(left, right))| {
                    (left == vertex || right == vertex).then_some(edge)
                })
                .collect()
        })
        .collect();
    let mut builder = IncidenceGraphBuilder::new();
    let mut outer = vec![vec![[VertexId::new(0); 2]; 3]; 4];
    for vertex_outer in &mut outer {
        for edge_outer in vertex_outer {
            *edge_outer = [
                builder.add_vertex(Vec::new()),
                builder.add_vertex(Vec::new()),
            ];
        }
    }
    for vertex_outer in &outer {
        for mask in 0_u8..8 {
            if mask.count_ones() % 2 != 0 {
                continue;
            }
            let middle = builder.add_vertex(Vec::new());
            for (local_edge, edge_outer) in vertex_outer.iter().enumerate() {
                let bit = usize::from((mask >> local_edge) & 1);
                builder
                    .add_undirected_relation(
                        middle,
                        edge_outer[bit],
                        b"edge".to_vec(),
                        Vec::new(),
                        1,
                    )
                    .map_err(debug_error)?;
            }
        }
    }
    for (edge, &(left, right)) in base_edges.iter().enumerate() {
        let left_local = incident[left]
            .iter()
            .position(|&candidate| candidate == edge)
            .expect("incident base edge");
        let right_local = incident[right]
            .iter()
            .position(|&candidate| candidate == edge)
            .expect("incident base edge");
        let twist = usize::from(twisted_edge == Some(edge));
        for bit in 0..2 {
            builder
                .add_undirected_relation(
                    outer[left][left_local][bit],
                    outer[right][right_local][bit ^ twist],
                    b"edge".to_vec(),
                    Vec::new(),
                    1,
                )
                .map_err(debug_error)?;
        }
    }
    builder.build().map_err(debug_error)
}

fn applied_verticals(
    labeler: &FastGraphLabeler<Fp251V1, PrimeIntegerEncoder, 3>,
) -> Result<Vec<AppliedVerticalResult>, String> {
    let cases = [
        molecule_case()?,
        directed_network_case()?,
        knowledge_graph_case()?,
        hypergraph_case()?,
    ];
    cases
        .into_iter()
        .map(|(name, original, perturbed)| {
            let relabeled = relabel_reverse(&original)?;
            let original_digest = labeler
                .analyze_discriminating(&original, GraphDiscriminationPolicy::adaptive())
                .map_err(debug_error)?
                .digest();
            let relabeled_digest = labeler
                .analyze_discriminating(&relabeled, GraphDiscriminationPolicy::adaptive())
                .map_err(debug_error)?
                .digest();
            let perturbed_digest = labeler
                .analyze_discriminating(&perturbed, GraphDiscriminationPolicy::adaptive())
                .map_err(debug_error)?
                .digest();
            Ok(AppliedVerticalResult {
                vertical: name.into(),
                relabeling_invariant: original_digest == relabeled_digest,
                typed_perturbation_detected: original_digest != perturbed_digest,
            })
        })
        .collect()
}

fn molecule_case() -> Result<(&'static str, IncidenceGraph, IncidenceGraph), String> {
    let build = |bond: &[u8]| {
        let mut builder = IncidenceGraphBuilder::new();
        let carbon = builder.add_vertex(b"C".to_vec());
        let oxygen = builder.add_vertex(b"O".to_vec());
        builder
            .add_undirected_relation(carbon, oxygen, bond.to_vec(), Vec::new(), 1)
            .map_err(debug_error)?;
        builder.build().map_err(debug_error)
    };
    Ok(("molecule", build(b"single")?, build(b"double")?))
}

fn directed_network_case() -> Result<(&'static str, IncidenceGraph, IncidenceGraph), String> {
    let build = |reverse: bool| {
        let mut builder = IncidenceGraphBuilder::new();
        let service = builder.add_vertex(b"service".to_vec());
        let database = builder.add_vertex(b"database".to_vec());
        let (source, target) = if reverse {
            (database, service)
        } else {
            (service, database)
        };
        builder
            .add_directed_relation(source, target, b"calls".to_vec(), b"client".to_vec(), 1)
            .map_err(debug_error)?;
        builder.build().map_err(debug_error)
    };
    Ok(("directed-network", build(false)?, build(true)?))
}

fn knowledge_graph_case() -> Result<(&'static str, IncidenceGraph, IncidenceGraph), String> {
    let build = |relation: &[u8]| {
        let mut builder = IncidenceGraphBuilder::new();
        let person = builder.add_vertex(b"person".to_vec());
        let city = builder.add_vertex(b"city".to_vec());
        builder
            .add_directed_relation(person, city, relation.to_vec(), Vec::new(), 1)
            .map_err(debug_error)?;
        builder.build().map_err(debug_error)
    };
    Ok(("knowledge-graph", build(b"born-in")?, build(b"works-in")?))
}

fn hypergraph_case() -> Result<(&'static str, IncidenceGraph, IncidenceGraph), String> {
    let build = |second_role: &[u8]| {
        let mut builder = IncidenceGraphBuilder::new();
        let enzyme = builder.add_vertex(b"enzyme".to_vec());
        let substrate = builder.add_vertex(b"substrate".to_vec());
        let product = builder.add_vertex(b"product".to_vec());
        builder
            .add_hyperedge(
                b"reaction".to_vec(),
                &[
                    HyperedgeIncidence::new(enzyme, b"catalyst".to_vec()),
                    HyperedgeIncidence::new(substrate, second_role.to_vec()),
                    HyperedgeIncidence::new(product, b"output".to_vec()),
                ],
            )
            .map_err(debug_error)?;
        builder.build().map_err(debug_error)
    };
    Ok(("hypergraph", build(b"input")?, build(b"cofactor")?))
}

fn incremental_work_curve(
    labeler: &FastGraphLabeler<Fp251V1, PrimeIntegerEncoder, 3>,
    rounds: usize,
) -> Result<Vec<IncrementalCurvePoint>, String> {
    let vertex_count = 1_000_usize;
    let baseline_labels: Vec<u64> = (0..vertex_count as u64).collect();
    let baseline = labeled_cycle(&baseline_labels)?;
    let full_vertex_rounds = vertex_count
        .checked_mul(rounds)
        .ok_or("incremental full-work overflow")?;
    [1_usize, 10, 100, 250, 500]
        .into_iter()
        .map(|edited_vertices| {
            let mut edited_labels = baseline_labels.clone();
            for index in 0..edited_vertices {
                let vertex = index * vertex_count / edited_vertices;
                edited_labels[vertex] ^= 0xa5a5_5a5a_d3c1_b7e9;
            }
            let edited = labeled_cycle(&edited_labels)?;
            let expected = labeler.analyze(&edited).map_err(debug_error)?;
            let mut state = labeler
                .incremental_state(baseline.clone())
                .map_err(debug_error)?;
            let mut workspace = IncrementalGraphWorkspace::new();
            workspace
                .reserve_for(vertex_count, edited.incidence_count(), rounds)
                .map_err(debug_error)?;
            let stats = labeler
                .update_incremental(&mut state, edited, &mut workspace)
                .map_err(debug_error)?;
            let matches = state.analysis().to_owned() == expected;
            let recomputed = stats.recomputed_vertex_rounds();
            Ok(IncrementalCurvePoint {
                vertex_count,
                edited_vertices,
                recomputed_vertex_rounds: recomputed,
                full_vertex_rounds,
                work_ratio: recomputed as f64 / full_vertex_rounds as f64,
                matches_full_recomputation: matches,
            })
        })
        .collect()
}

fn labeled_cycle(labels: &[u64]) -> Result<IncidenceGraph, String> {
    let mut builder = IncidenceGraphBuilder::new();
    let vertices: Vec<_> = labels
        .iter()
        .map(|label| builder.add_vertex(label.to_le_bytes().to_vec()))
        .collect();
    if vertices.len() > 1 {
        for index in 0..vertices.len() {
            builder
                .add_undirected_relation(
                    vertices[index],
                    vertices[(index + 1) % vertices.len()],
                    b"edge".to_vec(),
                    Vec::new(),
                    1,
                )
                .map_err(debug_error)?;
        }
    }
    builder.build().map_err(debug_error)
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        result.push(char::from(DIGITS[usize::from(byte >> 4)]));
        result.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    result
}

fn debug_error(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph6_parser_matches_known_small_records() {
        let empty4 = parse_graph6("C?").unwrap();
        assert_eq!(empty4.vertex_count(), 4);
        assert_eq!(empty4.incidence_count(), 0);
        let complete4 = parse_graph6("C~").unwrap();
        assert_eq!(complete4.vertex_count(), 4);
        assert_eq!(complete4.incidence_count(), 12);
    }

    #[test]
    fn adversarial_regular_pairs_are_not_mislabeled_as_proven_isomorphic() {
        let manifest = ValidationManifest {
            schema_version: 1,
            campaign_id: "test".into(),
            seed: 1,
            signature: crate::model::SignatureManifest {
                alphabet_size: 4,
                exhaustive_max_length: 2,
                collision_max_length: 2,
                reconciliation_universe: 8,
                reconciliation_max_difference: 2,
            },
            graph: crate::model::GraphManifest {
                exhaustive_max_vertices_ci: 7,
                exhaustive_max_vertices_full: 8,
                rounds: 8,
                exact_node_budget: 1_000_000,
                exact_retained_state_cells: 1024,
            },
            performance: crate::model::PerformanceManifest {
                warmup_iterations: 1,
                measured_iterations: 1,
                sparse_graph_vertices: vec![8],
            },
        };
        let results = adversarial_results(
            &labeler(&manifest).unwrap(),
            CanonicalSearchBudget::new(1_000_000).with_max_retained_state_cells(2_000_000),
        )
        .unwrap();
        assert!(results.iter().all(|result| result.non_isomorphic));
        assert!(results
            .iter()
            .all(|result| result.exact_distinguishes == Some(true)));
    }

    #[test]
    fn every_applied_vertical_is_invariant_and_detects_semantic_change() {
        let manifest_json = include_str!("../../../validation/f6/manifest.json");
        let manifest: ValidationManifest = serde_json::from_str(manifest_json).unwrap();
        for result in applied_verticals(&labeler(&manifest).unwrap()).unwrap() {
            assert!(result.relabeling_invariant, "{}", result.vertical);
            assert!(result.typed_perturbation_detected, "{}", result.vertical);
        }
    }
}
