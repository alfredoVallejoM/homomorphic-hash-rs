//! Reproducible, non-production harness for F6.V.

pub mod graphs;
pub mod model;
pub mod performance;
pub mod reconciliation;
pub mod signatures;

use std::{fs, path::Path};

use model::{SemanticReport, ValidationManifest};

/// Loads and validates a versioned F6.V experiment manifest.
pub fn load_manifest(path: &Path) -> Result<ValidationManifest, String> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let manifest: ValidationManifest = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse {}: {error}", path.display()))?;
    manifest.validate()?;
    Ok(manifest)
}

/// Executes every deterministic semantic campaign.
pub fn run_semantic(manifest: &ValidationManifest, root: &Path) -> Result<SemanticReport, String> {
    Ok(SemanticReport {
        schema_version: 1,
        campaign_id: manifest.campaign_id.clone(),
        seed: manifest.seed,
        signatures: signatures::run_campaign(manifest)?,
        reconciliation: reconciliation::run_campaign(manifest)?,
        graphs: graphs::run_campaign(manifest, root)?,
    })
}

/// Writes stable pretty JSON with a trailing newline.
pub fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
    }
    fs::write(path, bytes).map_err(|error| format!("write {}: {error}", path.display()))
}

/// Writes the compact, deterministic metrics table paired with a semantic report.
pub fn write_semantic_csv(path: &Path, report: &SemanticReport) -> Result<(), String> {
    let mut rows = vec!["section,variant,metric,value".to_owned()];
    rows.push(format!(
        "signatures,all,metamorphic_checks,{}",
        report.signatures.metamorphic_checks
    ));
    for profile in &report.signatures.collision_profiles {
        rows.push(format!(
            "signatures,{},semantic_inputs,{}",
            profile.signature, profile.semantic_inputs
        ));
        rows.push(format!(
            "signatures,{},collision_buckets,{}",
            profile.signature, profile.collision_buckets
        ));
        rows.push(format!(
            "signatures,{},colliding_inputs,{}",
            profile.signature, profile.colliding_inputs
        ));
    }
    rows.push(format!(
        "reconciliation,f251,exhaustive_pairs,{}",
        report.reconciliation.exhaustive_pairs
    ));
    rows.push(format!(
        "reconciliation,f251,recovered_pairs,{}",
        report.reconciliation.recovered_pairs
    ));
    rows.push(format!(
        "graphs,oracle,graph_count,{}",
        report.graphs.graph_count
    ));
    for profile in &report.graphs.collision_profiles {
        rows.push(format!(
            "graphs,{},distinct_outputs,{}",
            profile.tier, profile.distinct_outputs
        ));
        rows.push(format!(
            "graphs,{},collision_buckets,{}",
            profile.tier, profile.collision_buckets
        ));
        rows.push(format!(
            "graphs,{},colliding_graphs,{}",
            profile.tier, profile.colliding_graphs
        ));
        rows.push(format!(
            "graphs,{},colliding_pairs,{}",
            profile.tier, profile.colliding_pairs
        ));
    }
    for point in &report.graphs.incremental_work_curve {
        rows.push(format!(
            "graphs,incremental-{}-edits,work_ratio,{:.8}",
            point.edited_vertices, point.work_ratio
        ));
    }
    let mut contents = rows.join("\n");
    contents.push('\n');
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
    }
    fs::write(path, contents).map_err(|error| format!("write {}: {error}", path.display()))
}
