//! Executable completeness checks for the RC.7 correctness matrix.

use std::{collections::BTreeSet, path::Path};

use serde_json::Value;

#[test]
fn every_frozen_capability_has_concrete_correctness_evidence() {
    let supported: Value =
        serde_json::from_str(include_str!("../validation/rc/supported-surface-v1.json")).unwrap();
    let matrix: Value =
        serde_json::from_str(include_str!("../validation/rc/correctness-matrix-v1.json")).unwrap();
    assert_eq!(matrix["schema"], "microfield-rc-correctness-matrix-v1");
    assert_eq!(matrix["workstream"], "RC.7");

    let expected = supported["capabilities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["id"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    let entries = matrix["entries"].as_array().unwrap();
    let mut actual = BTreeSet::new();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    for entry in entries {
        let capability = entry["capability"].as_str().unwrap();
        assert!(
            actual.insert(capability),
            "duplicate matrix entry: {capability}"
        );
        let status = entry["status"].as_str().unwrap();
        assert!(
            matches!(status, "covered" | "baseline"),
            "invalid coverage status for {capability}: {status}"
        );
        if capability != "legacy.proof-facade" {
            assert_eq!(status, "covered", "maintained capability lacks coverage");
        }
        let methods = entry["methods"].as_array().unwrap();
        assert!(!methods.is_empty(), "no methods declared for {capability}");
        let evidence = entry["evidence"].as_array().unwrap();
        assert!(
            !evidence.is_empty(),
            "no evidence declared for {capability}"
        );
        for path in evidence {
            let path = path.as_str().unwrap();
            assert!(
                root.join(path).is_file(),
                "missing evidence for {capability}: {path}"
            );
        }
        for target in entry["fuzz_targets"].as_array().unwrap() {
            let target = target.as_str().unwrap();
            let path = root.join("fuzz/fuzz_targets").join(format!("{target}.rs"));
            assert!(
                path.is_file(),
                "missing fuzz target for {capability}: {target}"
            );
        }
    }

    assert_eq!(
        actual, expected,
        "correctness matrix and frozen surface drifted"
    );
}

#[test]
fn fuzz_seed_generator_and_locked_harness_are_versioned() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for path in [
        "fuzz/Cargo.lock",
        "fuzz/src/bin/generate_seed_corpus.rs",
        "fuzz/corpus/microfield_manifests/minimal-binary.toml",
        "fuzz/corpus/structural_wires/valid-additive-mfsg",
        "fuzz/corpus/structural_wires/valid-transaction-mftx",
        "fuzz/corpus/graph_wires/valid-canonical-graph-mfc2",
        "fuzz/corpus/graph_wires/valid-canonical-dag-mfgd",
    ] {
        assert!(
            root.join(path).is_file(),
            "missing reproducible fuzz asset: {path}"
        );
    }
}
