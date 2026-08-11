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
    assert_eq!(ledger["status"], "running");

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
fn c3_operation_inventory_owns_every_suite_and_exposes_every_gap() {
    let ledger: Value = serde_json::from_str(include_str!(
        "../validation/benchmarks/c3-coverage-ledger-v1.json"
    ))
    .unwrap();
    let inventory: Value = serde_json::from_str(include_str!(
        "../validation/benchmarks/c3-operation-inventory-v1.json"
    ))
    .unwrap();
    assert_eq!(inventory["schema"], "algesum-c3-operation-inventory-v1");

    let expected = ledger["suites"]
        .as_array()
        .unwrap()
        .iter()
        .map(|suite| suite["id"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    let observed = inventory["suites"]
        .as_array()
        .unwrap()
        .iter()
        .map(|suite| suite["id"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(observed, expected);

    for suite in inventory["suites"].as_array().unwrap() {
        let status = suite["status"].as_str().unwrap();
        let implemented = suite["implemented_operations"].as_array().unwrap();
        let missing = suite["missing_operations"].as_array().unwrap();
        assert!(
            !implemented.is_empty() || matches!(status, "external" | "missing"),
            "suite {} claims partial/complete without a workload",
            suite["id"]
        );
        assert!(
            !missing.is_empty()
                || status == "implemented"
                || (status == "external"
                    && suite["executed_external_operations"]
                        .as_array()
                        .is_some_and(|operations| !operations.is_empty())),
            "suite {} hides its remaining workload gaps",
            suite["id"]
        );
    }
}

#[test]
fn c3_generated_scaling_inventory_reaches_the_declared_volume_floor() {
    let ledger: Value = serde_json::from_str(include_str!(
        "../validation/benchmarks/c3-coverage-ledger-v1.json"
    ))
    .unwrap();
    let p0: Value = serde_json::from_str(include_str!(
        "../validation/benchmarks/manifests/c3-p0/c3-expansion-report-v1.json"
    ))
    .unwrap();
    let f3_s3: Value = serde_json::from_str(include_str!(
        "../validation/benchmarks/manifests/c3-f3-s3/c3-expansion-report-v1.json"
    ))
    .unwrap();
    let t1_r1_d1: Value = serde_json::from_str(include_str!(
        "../validation/benchmarks/manifests/c3-t1-r1-d1/c3-expansion-report-v1.json"
    ))
    .unwrap();
    let g1_g2: Value = serde_json::from_str(include_str!(
        "../validation/benchmarks/manifests/c3-g1-g2/c3-expansion-report-v1.json"
    ))
    .unwrap();
    let x2: Value = serde_json::from_str(include_str!(
        "../validation/benchmarks/manifests/c3-x2/c3-expansion-report-v1.json"
    ))
    .unwrap();
    let f1_f2_closure: Value = serde_json::from_str(include_str!(
        "../validation/benchmarks/manifests/c3-f1-f2-closure/c3-expansion-report-v1.json"
    ))
    .unwrap();

    let generated_cells = p0["total_cells"].as_u64().unwrap()
        + f3_s3["total_cells"].as_u64().unwrap()
        + t1_r1_d1["total_cells"].as_u64().unwrap()
        + g1_g2["total_cells"].as_u64().unwrap()
        + x2["total_cells"].as_u64().unwrap()
        + f1_f2_closure["total_cells"].as_u64().unwrap();
    let targets = &ledger["targets"];
    assert!(
        generated_cells >= targets["timed_cells_lower_bound"].as_u64().unwrap(),
        "the generated C3 inventory has not reached its declared lower bound"
    );
    assert!(
        generated_cells <= targets["timed_cells_upper_bound"].as_u64().unwrap(),
        "the generated C3 inventory exceeds its declared reviewable envelope"
    );
    assert_eq!(generated_cells, 3_609);
}

#[test]
fn c3_consolidated_status_matches_generated_inventory_and_remaining_gap() {
    let ledger: Value = serde_json::from_str(include_str!(
        "../validation/benchmarks/c3-coverage-ledger-v1.json"
    ))
    .unwrap();
    let inventory: Value = serde_json::from_str(include_str!(
        "../validation/benchmarks/c3-operation-inventory-v1.json"
    ))
    .unwrap();
    let status: Value = serde_json::from_str(include_str!(
        "../validation/benchmarks/runs/c3-preflight-consolidated-status-v1.json"
    ))
    .unwrap();

    assert_eq!(status["generated_publication_cells"], 3_609);
    assert_eq!(status["local_timed_cells_and_calibrations_executed"], 229);
    assert_eq!(status["independent_worker_processes"], 485);
    assert_eq!(status["observations"], 2_770);
    assert_eq!(status["semantic_failures"], 0);
    assert_eq!(status["unstable_checksums"], 0);
    assert_eq!(status["c3_c0_semantic"]["counted_cases"], 221_342);
    assert_eq!(status["c3_c0_semantic"]["fuzz_runs"], 15_000);
    assert_eq!(status["remaining_gates"][0]["status"], "complete");

    let semantic: Value = serde_json::from_str(include_str!(
        "../validation/benchmarks/runs/c3-c0-semantic-summary-v1.json"
    ))
    .unwrap();
    assert!(
        semantic["counted_cases"]["total"].as_u64().unwrap()
            >= ledger["targets"]["minimum_semantic_cases"]
                .as_u64()
                .unwrap()
    );
    assert_eq!(semantic["gate"], "passed");

    let minimum_processes = status["generated_publication_cells"].as_u64().unwrap()
        * ledger["targets"]["minimum_processes_per_timed_cell"]
            .as_u64()
            .unwrap();
    assert_eq!(
        status["remaining_gates"][1]["minimum_processes_per_host"],
        minimum_processes
    );

    let missing = inventory["suites"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|suite| suite["missing_operations"].as_array().unwrap())
        .map(|operation| operation.as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(missing, ["postgres.backpressure-soak"]);
    assert_eq!(
        status["operation_inventory"]["only_missing_operation"],
        missing[0]
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
