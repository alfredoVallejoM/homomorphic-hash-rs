//! Versioned RC.10 go/no-go evidence assembly.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Deserialize)]
pub struct DecisionManifest {
    schema: String,
    campaign_id: String,
    required_architectures: Vec<String>,
    inputs: DecisionInputs,
    corpus_paths: Vec<String>,
    required_features: Vec<String>,
    known_limitations: Vec<KnownLimitation>,
}

#[derive(Clone, Debug, Deserialize)]
struct DecisionInputs {
    supported_surface: String,
    correctness_matrix: String,
    capacity_manifest: String,
    dependency_inventory: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KnownLimitation {
    id: String,
    scope: String,
    condition: String,
    blocks_internal_use: bool,
    blocks_external_publication: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct DecisionReport {
    pub schema: &'static str,
    pub campaign_id: String,
    pub repository: RepositoryEvidence,
    pub toolchains: ToolchainEvidence,
    pub required_features: Vec<String>,
    pub hardware_runners: Vec<HardwareEvidence>,
    pub field_matrix: Vec<CapabilityEvidence>,
    pub signature_matrix: Vec<CapabilityEvidence>,
    pub capability_classification: BTreeMap<String, usize>,
    pub corpus_manifests: Vec<CorpusEvidence>,
    pub semantic_gates: Vec<DecisionGate>,
    pub performance_gates: Vec<DecisionGate>,
    pub known_limitations: Vec<KnownLimitation>,
    pub final_decision: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct RepositoryEvidence {
    pub commit: String,
    pub dirty: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ToolchainEvidence {
    pub rustc: String,
    pub cargo: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct HardwareEvidence {
    pub source: String,
    pub architecture: String,
    pub operating_system: String,
    pub rustc: Option<String>,
    pub logical_threads: Option<u64>,
    pub detected_features: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CapabilityEvidence {
    pub id: String,
    pub status: String,
    pub feature: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct CorpusEvidence {
    pub path: String,
    pub files: usize,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DecisionGate {
    pub id: String,
    pub status: &'static str,
    pub evidence: String,
}

pub fn load_manifest(path: &Path) -> Result<DecisionManifest, String> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let manifest: DecisionManifest = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse {}: {error}", path.display()))?;
    manifest.validate()?;
    Ok(manifest)
}

impl DecisionManifest {
    fn validate(&self) -> Result<(), String> {
        let architectures = self.required_architectures.iter().collect::<BTreeSet<_>>();
        let limitation_ids = self
            .known_limitations
            .iter()
            .map(|limitation| limitation.id.as_str())
            .collect::<BTreeSet<_>>();
        if self.schema != "microfield-rc10-decision-manifest-v1"
            || self.campaign_id.is_empty()
            || architectures.len() != self.required_architectures.len()
            || architectures.is_empty()
            || self.corpus_paths.is_empty()
            || self.required_features.is_empty()
            || limitation_ids.len() != self.known_limitations.len()
            || self
                .known_limitations
                .iter()
                .any(|limitation| limitation.id.is_empty() || limitation.condition.is_empty())
        {
            return Err("invalid RC.10 decision manifest".into());
        }
        for path in [
            &self.inputs.supported_surface,
            &self.inputs.correctness_matrix,
            &self.inputs.capacity_manifest,
            &self.inputs.dependency_inventory,
        ] {
            if path.is_empty() {
                return Err("RC.10 input path is empty".into());
            }
        }
        Ok(())
    }
}

pub fn build_report(
    manifest: &DecisionManifest,
    capacity_reports: &[PathBuf],
    consumer_reports: &[PathBuf],
    required_ci_gates_passed: bool,
) -> Result<DecisionReport, String> {
    let repository = repository_evidence()?;
    let toolchains = ToolchainEvidence {
        rustc: command_output("rustc", &["--version"])?,
        cargo: command_output("cargo", &["--version"])?,
    };
    let surface = read_json(Path::new(&manifest.inputs.supported_surface))?;
    let correctness = read_json(Path::new(&manifest.inputs.correctness_matrix))?;
    let capacity_manifest = read_json(Path::new(&manifest.inputs.capacity_manifest))?;
    let dependency_inventory = read_json(Path::new(&manifest.inputs.dependency_inventory))?;

    let capabilities = surface["capabilities"]
        .as_array()
        .ok_or("RC.10 supported surface has no capabilities")?;
    let mut classification = BTreeMap::new();
    let mut field_matrix = Vec::new();
    let mut signature_matrix = Vec::new();
    for capability in capabilities {
        let evidence = CapabilityEvidence {
            id: string_field(capability, "id")?.into(),
            status: string_field(capability, "status")?.into(),
            feature: string_field(capability, "feature")?.into(),
        };
        *classification.entry(evidence.status.clone()).or_insert(0) += 1;
        if evidence.id.starts_with("field.") {
            field_matrix.push(evidence);
        } else if evidence.id.starts_with("signature.") {
            signature_matrix.push(evidence);
        }
    }

    let correctness_entries = correctness["entries"]
        .as_array()
        .ok_or("RC.10 correctness matrix has no entries")?;
    let correctness_passed = !correctness_entries.is_empty()
        && correctness_entries
            .iter()
            .all(|entry| matches!(entry["status"].as_str(), Some("covered" | "baseline")));
    let mut semantic_gates = vec![gate(
        "rc7-correctness-matrix",
        if correctness_passed { "Pass" } else { "Fail" },
        format!(
            "{} classified correctness entries",
            correctness_entries.len()
        ),
    )];
    semantic_gates.push(gate(
        "rc9-dependency-inventory",
        if dependency_inventory["schema"] == "microfield-rc9-dependency-inventory-v1" {
            "Pass"
        } else {
            "Fail"
        },
        manifest.inputs.dependency_inventory.clone(),
    ));
    semantic_gates.push(gate(
        "required-ci-gates",
        if required_ci_gates_passed {
            "Pass"
        } else {
            "Missing"
        },
        "stable, properties, fuzz, Miri, sanitizers, ISA and external-consumer jobs".into(),
    ));
    semantic_gates.push(gate(
        "clean-commit",
        if repository.dirty { "Missing" } else { "Pass" },
        repository.commit.clone(),
    ));

    let required_architectures = manifest
        .required_architectures
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let (capacity_architectures, mut hardware_runners, capacity_failed) =
        read_capacity_reports(capacity_reports)?;
    let (consumer_architectures, consumer_hardware, consumer_failed) =
        read_consumer_reports(consumer_reports)?;
    hardware_runners.extend(consumer_hardware);
    hardware_runners.sort_by(|left, right| {
        (&left.architecture, &left.source).cmp(&(&right.architecture, &right.source))
    });

    let capacity_status = evidence_status(
        &required_architectures,
        &capacity_architectures,
        capacity_failed,
    );
    let consumer_status = evidence_status(
        &required_architectures,
        &consumer_architectures,
        consumer_failed,
    );
    let performance_gates = vec![
        gate(
            "rc8-capacity-slo",
            capacity_status,
            format!(
                "campaign={} architectures={capacity_architectures:?}",
                capacity_manifest["campaign_id"]
                    .as_str()
                    .unwrap_or("invalid")
            ),
        ),
        gate(
            "rc9-external-consumer",
            consumer_status,
            format!("architectures={consumer_architectures:?}"),
        ),
    ];

    let corpus_manifests = manifest
        .corpus_paths
        .iter()
        .map(|path| corpus_evidence(Path::new(path)))
        .collect::<Result<Vec<_>, _>>()?;
    let internal_blocker = manifest
        .known_limitations
        .iter()
        .any(|limitation| limitation.blocks_internal_use);
    if internal_blocker {
        semantic_gates.push(gate(
            "known-internal-blockers",
            "Fail",
            "decision manifest contains an internal-use blocker".into(),
        ));
    } else {
        semantic_gates.push(gate(
            "known-internal-blockers",
            "Pass",
            "no declared limitation blocks conditioned internal use".into(),
        ));
    }

    let statuses = semantic_gates
        .iter()
        .chain(&performance_gates)
        .map(|gate| gate.status)
        .collect::<Vec<_>>();
    let final_decision = classify(&statuses);
    Ok(DecisionReport {
        schema: "microfield-rc10-decision-report-v1",
        campaign_id: manifest.campaign_id.clone(),
        repository,
        toolchains,
        required_features: manifest.required_features.clone(),
        hardware_runners,
        field_matrix,
        signature_matrix,
        capability_classification: classification,
        corpus_manifests,
        semantic_gates,
        performance_gates,
        known_limitations: manifest.known_limitations.clone(),
        final_decision,
    })
}

fn read_capacity_reports(
    paths: &[PathBuf],
) -> Result<(BTreeSet<String>, Vec<HardwareEvidence>, bool), String> {
    let mut architectures = BTreeSet::new();
    let mut hardware = Vec::new();
    let mut failed = false;
    for path in paths {
        let report = read_json(path)?;
        if report["schema"] != "microfield-rc8-capacity-report-v1" {
            return Err(format!("{} is not an RC.8 capacity report", path.display()));
        }
        failed |= report["passed"] != true;
        let environment = &report["environment"];
        let architecture = string_field(environment, "architecture")?.to_owned();
        if !architectures.insert(architecture.clone()) {
            return Err(format!("duplicate RC.8 architecture {architecture}"));
        }
        hardware.push(HardwareEvidence {
            source: "rc8-capacity".into(),
            architecture,
            operating_system: string_field(environment, "operating_system")?.into(),
            rustc: environment["rustc"].as_str().map(str::to_owned),
            logical_threads: environment["logical_threads"].as_u64(),
            detected_features: string_array(&environment["detected_features"]),
        });
    }
    Ok((architectures, hardware, failed))
}

fn read_consumer_reports(
    paths: &[PathBuf],
) -> Result<(BTreeSet<String>, Vec<HardwareEvidence>, bool), String> {
    let mut architectures = BTreeSet::new();
    let mut hardware = Vec::new();
    let mut failed = false;
    for path in paths {
        let report = read_json(path)?;
        if report["schema"] != "microfield-rc9-consumer-report-v1" {
            return Err(format!("{} is not an RC.9 consumer report", path.display()));
        }
        failed |= report["passed"] != true;
        let architecture = string_field(&report, "architecture")?.to_owned();
        if !architectures.insert(architecture.clone()) {
            return Err(format!("duplicate RC.9 architecture {architecture}"));
        }
        hardware.push(HardwareEvidence {
            source: "rc9-consumer".into(),
            architecture,
            operating_system: string_field(&report, "operating_system")?.into(),
            rustc: None,
            logical_threads: None,
            detected_features: vec![string_field(&report, "detected_backend")?.into()],
        });
    }
    Ok((architectures, hardware, failed))
}

fn evidence_status<'a>(
    required: &BTreeSet<&str>,
    observed: &BTreeSet<String>,
    failed: bool,
) -> &'a str {
    if failed {
        "Fail"
    } else if required.iter().all(|required| observed.contains(*required)) {
        "Pass"
    } else {
        "Missing"
    }
}

fn classify(statuses: &[&str]) -> &'static str {
    if statuses.contains(&"Fail") {
        "NotReady"
    } else if statuses.contains(&"Missing") {
        "Conditional"
    } else {
        "ReadyForInternalUse"
    }
}

fn corpus_evidence(path: &Path) -> Result<CorpusEvidence, String> {
    let mut files = Vec::new();
    collect_files(path, &mut files)?;
    files.sort();
    let mut hash = Sha256::new();
    let mut bytes = 0_u64;
    for file in &files {
        let contents =
            fs::read(file).map_err(|error| format!("read {}: {error}", file.display()))?;
        let relative = file.strip_prefix(path).unwrap_or(file);
        hash.update(relative.to_string_lossy().as_bytes());
        hash.update([0]);
        hash.update((contents.len() as u64).to_le_bytes());
        hash.update(&contents);
        bytes = bytes
            .checked_add(contents.len() as u64)
            .ok_or("RC.10 corpus byte count overflow")?;
    }
    Ok(CorpusEvidence {
        path: path.display().to_string(),
        files: files.len(),
        bytes,
        sha256: hex(&hash.finalize()),
    })
}

fn collect_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let metadata =
        fs::metadata(path).map_err(|error| format!("stat {}: {error}", path.display()))?;
    if metadata.is_file() {
        files.push(path.to_owned());
        return Ok(());
    }
    if !metadata.is_dir() {
        return Err(format!("unsupported RC.10 corpus path {}", path.display()));
    }
    let mut entries = fs::read_dir(path)
        .map_err(|error| format!("read directory {}: {error}", path.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("read directory {}: {error}", path.display()))?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        collect_files(&entry.path(), files)?;
    }
    Ok(())
}

fn repository_evidence() -> Result<RepositoryEvidence, String> {
    let commit = command_output("git", &["rev-parse", "HEAD"])?;
    let status = command_output(
        "git",
        &["status", "--porcelain", "--untracked-files=normal"],
    )?;
    Ok(RepositoryEvidence {
        commit,
        dirty: !status.is_empty(),
    })
}

fn command_output(program: &str, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(arguments)
        .output()
        .map_err(|error| format!("run {program}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{program} {:?} failed: {}",
            arguments,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn read_json(path: &Path) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("parse {}: {error}", path.display()))
}

fn string_field<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value[field]
        .as_str()
        .ok_or_else(|| format!("missing string field {field}"))
}

fn string_array(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn gate(id: &str, status: &'static str, evidence: String) -> DecisionGate {
    DecisionGate {
        id: id.into(),
        status,
        evidence,
    }
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision_precedence_is_fail_then_missing_then_ready() {
        assert_eq!(classify(&["Pass", "Fail", "Missing"]), "NotReady");
        assert_eq!(classify(&["Pass", "Missing"]), "Conditional");
        assert_eq!(classify(&["Pass", "Pass"]), "ReadyForInternalUse");
    }

    #[test]
    fn architecture_gate_requires_every_declared_runner() {
        let required = ["aarch64", "x86_64"].into_iter().collect();
        let one = ["x86_64".to_owned()].into_iter().collect();
        let both = ["aarch64".to_owned(), "x86_64".to_owned()]
            .into_iter()
            .collect();
        assert_eq!(evidence_status(&required, &one, false), "Missing");
        assert_eq!(evidence_status(&required, &both, false), "Pass");
        assert_eq!(evidence_status(&required, &both, true), "Fail");
    }
}
