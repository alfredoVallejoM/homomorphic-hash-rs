//! Locks the extensive C3 plan to the complete admitted RC surface.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

#[test]
fn c3_ledger_covers_every_admitted_capability_exactly_once() {
    let surface: Value =
        serde_json::from_str(include_str!("../validation/rc/supported-surface-v1.json")).unwrap();
    let ledger: Value = serde_json::from_str(include_str!(
        "../validation/benchmarks/c3-coverage-ledger-v1.json"
    ))
    .unwrap();

    assert_eq!(ledger["schema"], "algesum-c3-coverage-ledger-v1");
    assert_eq!(
        ledger["security_classification"],
        "non-cryptographic-algebraic-summaries"
    );
    assert_eq!(ledger["status"], "planned");

    let admitted = surface["capabilities"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|capability| capability["status"] != "rejected")
        .map(|capability| capability["id"].as_str().unwrap())
        .collect::<BTreeSet<_>>();

    let suite_ids = ledger["suites"]
        .as_array()
        .unwrap()
        .iter()
        .map(|suite| suite["id"].as_str().unwrap())
        .collect::<BTreeSet<_>>();

    let mut counts = BTreeMap::new();
    for entry in ledger["coverage"].as_array().unwrap() {
        let capability = entry["capability"].as_str().unwrap();
        *counts.entry(capability).or_insert(0usize) += 1;
        assert!(
            suite_ids.contains(entry["suite"].as_str().unwrap()),
            "unknown suite for {capability}"
        );
        assert!(
            !entry["questions"].as_array().unwrap().is_empty(),
            "missing C3 questions for {capability}"
        );
        assert_ne!(
            entry["mode"], "excluded",
            "the initial extensive C3 plan must not silently exclude {capability}"
        );
    }

    let covered = counts.keys().copied().collect::<BTreeSet<_>>();
    assert_eq!(
        covered, admitted,
        "C3 ledger and admitted RC surface drifted"
    );
    assert!(
        counts.values().all(|count| *count == 1),
        "each capability must have exactly one owning C3 suite"
    );

    let targets = &ledger["targets"];
    assert!(targets["minimum_semantic_cases"].as_u64().unwrap() >= 12_000);
    assert!(targets["timed_cells_lower_bound"].as_u64().unwrap() >= 2_500);
    assert!(targets["systems_scenarios_lower_bound"].as_u64().unwrap() >= 150);
    assert!(
        targets["minimum_processes_per_timed_cell"]
            .as_u64()
            .unwrap()
            >= 30
    );
}

#[test]
fn c3_plan_names_the_required_campaign_lanes_and_rejects_weak_selection() {
    let plan = include_str!("../docs/microfield/c3-extensive-campaign-plan.md");
    for required in [
        "C3-Semantic",
        "C3-Scaling",
        "C3-Systems",
        "covering array",
        "x86-64 Intel",
        "x86-64 AMD",
        "AArch64",
        "PostgreSQL",
        "canonización exacta",
        "no criptográficos",
    ] {
        assert!(
            plan.contains(required),
            "missing C3 requirement: {required}"
        );
    }
    assert!(plan.contains("no se reducirá"));
}
