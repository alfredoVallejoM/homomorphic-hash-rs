//! Executable RC.9 packaging, consumer and operations inventory.

use std::{collections::BTreeSet, fs};

use serde_json::Value;

#[test]
fn rc9_inventory_is_locked_bounded_and_explicit_about_publication_gaps() {
    let inventory: Value = serde_json::from_str(include_str!(
        "../validation/rc/dependency-inventory-v1.json"
    ))
    .unwrap();
    assert_eq!(
        inventory["schema"],
        "microfield-rc9-dependency-inventory-v1"
    );
    let lockfiles = inventory["lockfiles"].as_array().unwrap();
    let paths = lockfiles
        .iter()
        .map(|entry| entry["path"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    for required in [
        "Cargo.lock",
        "fuzz/Cargo.lock",
        "test-fixtures/rc-consumer/Cargo.lock",
        "crates/microfield/test-fixtures/external-consumer/Cargo.lock",
    ] {
        assert!(paths.contains(required));
        assert!(fs::metadata(required).unwrap().len() > 0);
    }
    for lockfile in lockfiles {
        let path = lockfile["path"].as_str().unwrap();
        let declared = lockfile["packages"].as_u64().unwrap() as usize;
        let observed = fs::read_to_string(path)
            .unwrap()
            .matches("[[package]]")
            .count();
        assert_eq!(declared, observed, "dependency inventory drift in {path}");
    }
    assert_eq!(inventory["feature_policy"]["legacy_default"], false);
    assert_eq!(
        inventory["package_boundary"]["microfield_archive_dry_run"],
        "passed"
    );
    assert_eq!(
        inventory["advisory_audit"]["classification"],
        "external-publication-blocker"
    );
}

#[test]
fn independent_consumer_and_runbook_are_present_and_nontrivial() {
    for required in [
        "test-fixtures/rc-consumer/Cargo.toml",
        "test-fixtures/rc-consumer/Cargo.lock",
        "test-fixtures/rc-consumer/src/lib.rs",
        "test-fixtures/rc-consumer/src/main.rs",
        "docs/microfield/rc-9-operations-runbook.md",
        "tools/audit_rc_package.sh",
    ] {
        assert!(
            fs::metadata(required).unwrap().len() > 100,
            "missing or empty RC.9 artifact {required}"
        );
    }
    let manifest = fs::read_to_string("test-fixtures/rc-consumer/Cargo.toml").unwrap();
    assert!(manifest.contains("default-features = false"));
    assert!(manifest.contains("features = [\"signatures\", \"graph\"]"));
    let root_manifest = fs::read_to_string("Cargo.toml").unwrap();
    assert!(root_manifest.contains("default = [\"signatures\", \"graph\"]"));
    assert!(!root_manifest.contains("default = [\"legacy\""));
}

#[test]
fn rc10_decision_contract_is_versioned_fail_closed_and_wired_to_ci() {
    let manifest: Value =
        serde_json::from_str(include_str!("../validation/rc/decision-manifest-v1.json")).unwrap();
    assert_eq!(manifest["schema"], "microfield-rc10-decision-manifest-v1");
    assert_eq!(manifest["campaign_id"], "internal-rc-v1");
    assert_eq!(
        manifest["required_architectures"],
        serde_json::json!(["x86_64", "aarch64"])
    );

    for path in manifest["inputs"].as_object().unwrap().values() {
        let path = path.as_str().unwrap();
        assert!(
            fs::metadata(path).unwrap().len() > 100,
            "missing RC.10 evidence input {path}"
        );
    }
    for path in manifest["corpus_paths"].as_array().unwrap() {
        let path = path.as_str().unwrap();
        assert!(fs::metadata(path).is_ok(), "missing RC.10 corpus {path}");
    }

    let limitations = manifest["known_limitations"].as_array().unwrap();
    assert!(limitations.iter().any(|entry| {
        entry["id"] == "non-cryptographic-signatures"
            && entry["condition"]
                .as_str()
                .unwrap()
                .contains("do not authenticate")
            && entry["blocks_internal_use"] == false
    }));
    assert!(limitations.iter().any(|entry| {
        entry["id"] == "external-publication-metadata"
            && entry["blocks_external_publication"] == true
    }));

    let workflow = fs::read_to_string(".github/workflows/microfield.yml").unwrap();
    for required in [
        "rc10-decision:",
        "RC.10 reproducible go-no-go",
        "rc8-capacity-x86_64",
        "rc8-capacity-aarch64",
        "rc9-consumer-x86_64",
        "rc9-consumer-aarch64",
        "--required-ci-gates-passed",
        "'.final_decision == \"ReadyForInternalUse\"'",
    ] {
        assert!(
            workflow.contains(required),
            "missing RC.10 CI contract {required}"
        );
    }
}
