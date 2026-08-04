//! Opt-in validation against pinned public graph, molecule, network and hypergraph data.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use homomorphic_hash_rs::{
    DiscriminatingGraphComparison, FastGraphLabeler, GraphDiscriminationPolicy, HyperedgeIncidence,
    IncidenceGraph, IncidenceGraphBuilder, PrimeIntegerEncoder, RefinementProfile, VertexId,
};
use microfield::Fp251V1;
use serde_json::Value;

const DOMAIN: u64 = 0x4558_5445_524e_414c;

fn corpus() -> PathBuf {
    std::env::var_os("MICROFIELD_GRAPH_CORPUS").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".cache/graph-corpus/expanded"),
        PathBuf::from,
    )
}

fn labeler() -> FastGraphLabeler<Fp251V1, PrimeIntegerEncoder, 3> {
    FastGraphLabeler::new(
        PrimeIntegerEncoder::new(DOMAIN),
        RefinementProfile::Fast { rounds: 8 },
    )
    .unwrap()
}

fn relabel_reverse(graph: &IncidenceGraph) -> IncidenceGraph {
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
                .unwrap();
        }
    }
    builder.build().unwrap()
}

fn parse_atlas(path: &Path) -> Vec<IncidenceGraph> {
    let text = fs::read_to_string(path).unwrap();
    let lines: Vec<_> = text.lines().collect();
    let mut graphs = Vec::new();
    let mut cursor = 0;
    while cursor < lines.len() {
        let graph_index: usize = lines[cursor]
            .strip_prefix("GRAPH ")
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(graph_index, graphs.len());
        cursor += 1;
        let vertex_count: usize = lines[cursor]
            .strip_prefix("NODES ")
            .unwrap()
            .parse()
            .unwrap();
        cursor += 1;
        let mut builder = IncidenceGraphBuilder::new();
        let vertices: Vec<_> = (0..vertex_count)
            .map(|_| builder.add_vertex(Vec::new()))
            .collect();
        while cursor < lines.len() && !lines[cursor].starts_with("GRAPH ") {
            let mut endpoints = lines[cursor]
                .split_ascii_whitespace()
                .map(|value| value.parse::<usize>().unwrap());
            let left = endpoints.next().unwrap();
            let right = endpoints.next().unwrap();
            assert!(endpoints.next().is_none());
            builder
                .add_undirected_relation(
                    vertices[left],
                    vertices[right],
                    b"edge".to_vec(),
                    Vec::new(),
                    1,
                )
                .unwrap();
            cursor += 1;
        }
        graphs.push(builder.build().unwrap());
    }
    graphs
}

fn read_i64_lines(path: &Path) -> Vec<i64> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| line.trim().parse().unwrap())
        .collect()
}

fn parse_mutag(root: &Path) -> Vec<(IncidenceGraph, i64)> {
    let directory = root.join("MUTAG");
    let indicators = read_i64_lines(&directory.join("MUTAG_graph_indicator.txt"));
    let node_labels = read_i64_lines(&directory.join("MUTAG_node_labels.txt"));
    let graph_labels = read_i64_lines(&directory.join("MUTAG_graph_labels.txt"));
    let edge_labels = read_i64_lines(&directory.join("MUTAG_edge_labels.txt"));
    let edge_lines: Vec<_> = fs::read_to_string(directory.join("MUTAG_A.txt"))
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(indicators.len(), node_labels.len());
    assert_eq!(edge_lines.len(), edge_labels.len());

    let mut builders: Vec<_> = (0..graph_labels.len())
        .map(|_| IncidenceGraphBuilder::new())
        .collect();
    let mut global_to_local = Vec::with_capacity(indicators.len());
    for (&graph, &atom) in indicators.iter().zip(&node_labels) {
        let graph = usize::try_from(graph - 1).unwrap();
        let label = format!("atom/{atom}").into_bytes();
        global_to_local.push((graph, builders[graph].add_vertex(label)));
    }
    for (line, &bond) in edge_lines.iter().zip(&edge_labels) {
        let endpoints: Vec<_> = line
            .split(',')
            .map(|value| value.trim().parse::<usize>().unwrap() - 1)
            .collect();
        assert_eq!(endpoints.len(), 2);
        let (graph, source) = global_to_local[endpoints[0]];
        let (target_graph, target) = global_to_local[endpoints[1]];
        assert_eq!(graph, target_graph);
        builders[graph]
            .add_directed_relation(
                source,
                target,
                format!("bond/{bond}").into_bytes(),
                Vec::new(),
                1,
            )
            .unwrap();
    }
    builders
        .into_iter()
        .zip(graph_labels)
        .map(|(builder, class)| (builder.build().unwrap(), class))
        .collect()
}

fn parse_email(root: &Path) -> IncidenceGraph {
    let mut departments = vec![None; 1_005];
    for line in fs::read_to_string(root.join("email-Eu-core-department-labels.txt"))
        .unwrap()
        .lines()
    {
        let values: Vec<_> = line
            .split_ascii_whitespace()
            .map(|value| value.parse::<usize>().unwrap())
            .collect();
        departments[values[0]] = Some(values[1]);
    }
    assert!(departments.iter().all(Option::is_some));

    let mut builder = IncidenceGraphBuilder::new();
    let vertices: Vec<_> = departments
        .into_iter()
        .map(|department| {
            builder.add_vertex(format!("department/{}", department.unwrap()).into_bytes())
        })
        .collect();
    let mut raw_edges = 0_usize;
    for line in fs::read_to_string(root.join("email-Eu-core.txt"))
        .unwrap()
        .lines()
    {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let endpoints: Vec<_> = line
            .split_ascii_whitespace()
            .map(|value| value.parse::<usize>().unwrap())
            .collect();
        builder
            .add_directed_relation(
                vertices[endpoints[0]],
                vertices[endpoints[1]],
                b"email".to_vec(),
                b"sender-to-recipient".to_vec(),
                1,
            )
            .unwrap();
        raw_edges += 1;
    }
    assert_eq!(raw_edges, 25_571);
    builder.build().unwrap()
}

fn parse_diseasome(path: &Path) -> IncidenceGraph {
    let document: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let nodes = document["node-data"].as_object().unwrap();
    let edges = document["edge-dict"].as_object().unwrap();
    let edge_data = document["edge-data"].as_object().unwrap();
    assert_eq!(nodes.len(), 516);
    assert_eq!(edges.len(), 903);

    let mut node_ids: Vec<_> = nodes.keys().collect();
    node_ids.sort_unstable();
    let mut builder = IncidenceGraphBuilder::new();
    let mut vertices = BTreeMap::new();
    for id in node_ids {
        let label = nodes[id]["label"].as_str().unwrap().as_bytes().to_vec();
        vertices.insert(id.clone(), builder.add_vertex(label));
    }
    let mut edge_ids: Vec<_> = edges.keys().collect();
    edge_ids.sort_unstable();
    for id in edge_ids {
        let incidences: Vec<_> = edges[id]
            .as_array()
            .unwrap()
            .iter()
            .map(|node| {
                HyperedgeIncidence::new(vertices[node.as_str().unwrap()], b"disease".to_vec())
            })
            .collect();
        let gene = edge_data[id]["label"].as_str().unwrap().as_bytes().to_vec();
        builder.add_hyperedge(gene, &incidences).unwrap();
    }
    builder.build().unwrap()
}

#[test]
#[ignore = "requires tools/fetch_graph_corpus.py"]
fn graph_atlas_has_no_v2_indistinguishable_non_isomorphic_pair() {
    let graphs = parse_atlas(&corpus().join("networkx-atlas.dat"));
    assert_eq!(graphs.len(), 1_253);
    let labeler = labeler();
    let policy = GraphDiscriminationPolicy::Adaptive {
        max_motif_work: 100_000,
    };
    let mut digests = Vec::with_capacity(graphs.len());
    for (index, graph) in graphs.iter().enumerate() {
        let analysis = labeler.analyze_discriminating(graph, policy).unwrap();
        let relabeled = labeler
            .analyze_discriminating(&relabel_reverse(graph), policy)
            .unwrap();
        assert_eq!(analysis.digest(), relabeled.digest(), "atlas graph {index}");
        assert_eq!(analysis.global(), relabeled.global(), "atlas graph {index}");
        assert_eq!(
            analysis.compare(&relabeled).unwrap(),
            DiscriminatingGraphComparison::Indistinguishable,
            "atlas graph {index} is not invariant"
        );
        digests.push((analysis.digest().as_bytes().to_vec(), index));
    }
    digests.sort_unstable();
    for adjacent in digests.windows(2) {
        assert_ne!(
            adjacent[0].0, adjacent[1].0,
            "v2 collision between distinct atlas graphs {} and {}",
            adjacent[0].1, adjacent[1].1
        );
    }
}

#[test]
#[ignore = "requires tools/fetch_graph_corpus.py"]
fn mutag_molecules_preserve_atom_and_bond_labels_under_reindexing() {
    let molecules = parse_mutag(&corpus());
    assert_eq!(molecules.len(), 188);
    assert_eq!(
        molecules.iter().filter(|(_, class)| *class == 1).count(),
        125
    );
    assert_eq!(
        molecules.iter().filter(|(_, class)| *class == -1).count(),
        63
    );
    let labeler = labeler();
    for (index, (molecule, _)) in molecules.iter().enumerate() {
        assert!(molecule.vertex_count() > 0);
        let original = labeler
            .analyze_discriminating(molecule, GraphDiscriminationPolicy::default())
            .unwrap();
        let reversed = labeler
            .analyze_discriminating(
                &relabel_reverse(molecule),
                GraphDiscriminationPolicy::default(),
            )
            .unwrap();
        assert_eq!(
            original.digest(),
            reversed.digest(),
            "MUTAG molecule {index}"
        );
        assert_eq!(
            original.compare(&reversed).unwrap(),
            DiscriminatingGraphComparison::Indistinguishable,
            "MUTAG molecule {index}"
        );
    }
}

#[test]
#[ignore = "requires tools/fetch_graph_corpus.py"]
fn snap_directed_labeled_network_preserves_wcc_scc_and_relabeling() {
    let graph = parse_email(&corpus());
    assert_eq!(graph.vertex_count(), 1_005);
    assert_eq!(graph.incidence_count(), 25_571);
    let labeler = labeler();
    let policy = GraphDiscriminationPolicy::GlobalLinear;
    let original = labeler.analyze_discriminating(&graph, policy).unwrap();
    let reversed = labeler
        .analyze_discriminating(&relabel_reverse(&graph), policy)
        .unwrap();
    assert_eq!(original.digest(), reversed.digest());
    assert!(original.global().weak_component_count() > 1);
    assert!(original.global().strongly_connected_component_count() > 1);
    assert_eq!(
        original.compare(&reversed).unwrap(),
        DiscriminatingGraphComparison::Indistinguishable
    );
}

#[test]
#[ignore = "requires tools/fetch_graph_corpus.py"]
fn xgi_diseasome_preserves_labeled_hyperedges_and_roles() {
    let graph = parse_diseasome(&corpus().join("diseasome.json"));
    assert_eq!(graph.vertex_count(), 516 + 903);
    assert!(graph.incidence_count() > 903 * 2);
    let labeler = labeler();
    let policy = GraphDiscriminationPolicy::GlobalLinear;
    let original = labeler.analyze_discriminating(&graph, policy).unwrap();
    let reversed = labeler
        .analyze_discriminating(&relabel_reverse(&graph), policy)
        .unwrap();
    assert_eq!(original.digest(), reversed.digest());
    assert_eq!(original.global().weak_component_count(), 1);
    assert_eq!(
        original.global().weak_components()[0].hyperedge_count(),
        903
    );
}
