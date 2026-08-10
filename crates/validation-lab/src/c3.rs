//! Deterministic expansion of the extensive C3 factorial plan.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    publication::{is_supported_operation, BenchmarkCell, BenchmarkManifest},
    write_json,
};

use crate::publication::{CampaignProfile, HostMode};

#[derive(Clone, Debug, Deserialize)]
pub struct FactorPlan {
    schema: String,
    campaign_id: String,
    seed: u64,
    shards: Vec<FactorShard>,
}

#[derive(Clone, Debug, Deserialize)]
struct FactorShard {
    id: String,
    family: String,
    scale_unit: String,
    operations: Vec<FactorOperation>,
    #[serde(default)]
    preflight_max_scale: Option<usize>,
    primary_factors: BTreeMap<String, Vec<FactorValue>>,
    #[serde(default)]
    secondary_factors: BTreeMap<String, Vec<FactorValue>>,
}

#[derive(Clone, Debug, Deserialize)]
struct FactorOperation {
    id: String,
    operation: String,
    #[serde(default)]
    strategy: Option<String>,
    #[serde(default)]
    baseline: Option<String>,
    #[serde(default)]
    maximum_scale: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(untagged)]
enum FactorValue {
    Integer(usize),
    Text(String),
}

impl FactorValue {
    fn id_fragment(&self) -> String {
        match self {
            Self::Integer(value) => value.to_string(),
            Self::Text(value) => value
                .chars()
                .map(|character| {
                    if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                        character
                    } else {
                        '-'
                    }
                })
                .collect(),
        }
    }

    fn as_usize(&self, factor: &str) -> Result<usize, String> {
        match self {
            Self::Integer(value) => Ok(*value),
            Self::Text(_) => Err(format!("factor {factor:?} must contain integers")),
        }
    }

    fn as_text(&self, factor: &str) -> Result<&str, String> {
        match self {
            Self::Text(value) => Ok(value),
            Self::Integer(_) => Err(format!("factor {factor:?} must contain strings")),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ExpansionReport {
    schema: String,
    campaign_id: String,
    source: String,
    total_cells: usize,
    total_preflight_cells: usize,
    shards: Vec<ShardExpansion>,
}

impl ExpansionReport {
    pub const fn total_cells(&self) -> usize {
        self.total_cells
    }

    pub const fn total_preflight_cells(&self) -> usize {
        self.total_preflight_cells
    }
}

#[derive(Clone, Debug, Serialize)]
struct ShardExpansion {
    id: String,
    operations: usize,
    primary_combinations: usize,
    secondary_combinations: usize,
    publication_cells: usize,
    smoke_cells: usize,
    preflight_cells: usize,
    requested_preflight_fraction: f64,
    actual_preflight_fraction: f64,
    publication_manifest: String,
    smoke_manifest: String,
    preflight_manifest: String,
}

pub fn expand_plan(plan_path: &Path, output_directory: &Path) -> Result<ExpansionReport, String> {
    let bytes = fs::read(plan_path)
        .map_err(|error| format!("read C3 factor plan {}: {error}", plan_path.display()))?;
    let plan: FactorPlan = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse C3 factor plan {}: {error}", plan_path.display()))?;
    validate_plan(&plan)?;
    fs::create_dir_all(output_directory)
        .map_err(|error| format!("create {}: {error}", output_directory.display()))?;

    let mut shard_reports = Vec::with_capacity(plan.shards.len());
    let mut total_cells = 0;
    let mut total_preflight_cells = 0;
    for shard in &plan.shards {
        let primary = cartesian(&shard.primary_factors)?;
        let secondary = pairwise_cover(&shard.secondary_factors)?;
        let cells = expand_shard(shard, &primary, &secondary)?;
        let smoke_cells = sample_cells(&cells, &shard.operations, 0.0, Some(1));
        let preflight_cells =
            sample_cells(&cells, &shard.operations, 0.01, shard.preflight_max_scale);

        let publication_name = format!("c3-{}-publication-v1.json", shard.id);
        let smoke_name = format!("c3-{}-smoke-v1.json", shard.id);
        let preflight_name = format!("c3-{}-preflight-v1.json", shard.id);
        write_json(
            &output_directory.join(&publication_name),
            &manifest(
                &plan,
                shard,
                CampaignProfile::Publication,
                HostMode::Dedicated,
                30,
                100,
                10,
                50,
                1_000_000,
                10_000,
                0.03,
                cells.clone(),
            ),
        )?;
        write_json(
            &output_directory.join(&smoke_name),
            &manifest(
                &plan,
                shard,
                CampaignProfile::Smoke,
                HostMode::CiShared,
                2,
                2,
                2,
                5,
                100_000,
                1_024,
                0.5,
                smoke_cells.clone(),
            ),
        )?;
        write_json(
            &output_directory.join(&preflight_name),
            &manifest(
                &plan,
                shard,
                CampaignProfile::Smoke,
                HostMode::Interactive,
                2,
                2,
                2,
                5,
                250_000,
                1_024,
                0.25,
                preflight_cells.clone(),
            ),
        )?;
        total_cells += cells.len();
        total_preflight_cells += preflight_cells.len();
        shard_reports.push(ShardExpansion {
            id: shard.id.clone(),
            operations: shard.operations.len(),
            primary_combinations: primary.len(),
            secondary_combinations: secondary.len(),
            publication_cells: cells.len(),
            smoke_cells: smoke_cells.len(),
            preflight_cells: preflight_cells.len(),
            requested_preflight_fraction: 0.01,
            actual_preflight_fraction: preflight_cells.len() as f64 / cells.len() as f64,
            publication_manifest: publication_name,
            smoke_manifest: smoke_name,
            preflight_manifest: preflight_name,
        });
    }
    let report = ExpansionReport {
        schema: "algesum-c3-expansion-report-v1".into(),
        campaign_id: plan.campaign_id,
        source: plan_path
            .file_name()
            .ok_or("C3 factor plan path has no filename")?
            .to_string_lossy()
            .into_owned(),
        total_cells,
        total_preflight_cells,
        shards: shard_reports,
    };
    write_json(
        &output_directory.join("c3-expansion-report-v1.json"),
        &report,
    )?;
    Ok(report)
}

#[allow(clippy::too_many_arguments)]
fn manifest(
    plan: &FactorPlan,
    shard: &FactorShard,
    profile: CampaignProfile,
    host_mode: HostMode,
    minimum_processes: usize,
    maximum_processes: usize,
    warmup_observations: usize,
    measured_observations: usize,
    target_observation_ns: u64,
    maximum_batch_iterations: u64,
    maximum_relative_ci_half_width: f64,
    cells: Vec<BenchmarkCell>,
) -> BenchmarkManifest {
    let suffix = match profile {
        CampaignProfile::Publication => "publication",
        CampaignProfile::Smoke if host_mode == HostMode::Interactive => "preflight",
        CampaignProfile::Smoke => "smoke",
        CampaignProfile::Pilot => "pilot",
    };
    BenchmarkManifest {
        schema: "microfield-publication-benchmark-manifest-v1".into(),
        campaign_id: format!("{}-{}-{suffix}", plan.campaign_id, shard.id),
        profile,
        seed: plan.seed,
        host_mode,
        minimum_processes,
        maximum_processes,
        precision_check_every: 5.min(maximum_processes - minimum_processes + 1),
        warmup_observations,
        measured_observations,
        target_observation_ns,
        maximum_batch_iterations,
        bootstrap_resamples: if profile == CampaignProfile::Publication {
            10_000
        } else {
            200
        },
        maximum_relative_ci_half_width,
        cells_from: None,
        cells,
    }
}

fn validate_plan(plan: &FactorPlan) -> Result<(), String> {
    if plan.schema != "algesum-c3-factor-plan-v1"
        || plan.campaign_id.is_empty()
        || plan.shards.is_empty()
    {
        return Err("invalid C3 factor plan envelope".into());
    }
    let mut shard_ids = BTreeSet::new();
    for shard in &plan.shards {
        if !valid_id(&shard.id)
            || !shard_ids.insert(shard.id.as_str())
            || shard.family.is_empty()
            || shard.scale_unit.is_empty()
            || shard.operations.is_empty()
            || !shard.primary_factors.contains_key("scale")
        {
            return Err(format!("invalid C3 shard {}", shard.id));
        }
        validate_factors(&shard.primary_factors)?;
        validate_factors(&shard.secondary_factors)?;
        let primary_names = shard.primary_factors.keys().collect::<BTreeSet<_>>();
        if shard
            .secondary_factors
            .keys()
            .any(|name| primary_names.contains(name))
        {
            return Err(format!(
                "duplicated primary/secondary factor in {}",
                shard.id
            ));
        }
        let mut operation_ids = BTreeSet::new();
        for operation in &shard.operations {
            if !valid_id(&operation.id)
                || !operation_ids.insert(operation.id.as_str())
                || !is_supported_operation(&operation.operation)
            {
                return Err(format!(
                    "invalid or unsupported C3 operation {} in {}",
                    operation.id, shard.id
                ));
            }
        }
        for operation in &shard.operations {
            if operation
                .baseline
                .as_deref()
                .is_some_and(|baseline| !operation_ids.contains(baseline))
            {
                return Err(format!(
                    "unknown baseline for operation {} in {}",
                    operation.id, shard.id
                ));
            }
        }
    }
    Ok(())
}

fn validate_factors(factors: &BTreeMap<String, Vec<FactorValue>>) -> Result<(), String> {
    for (name, values) in factors {
        if !matches!(
            name.as_str(),
            "scale" | "payload_bytes" | "dataset_size" | "strategy"
        ) || values.is_empty()
            || values.iter().collect::<BTreeSet<_>>().len() != values.len()
        {
            return Err(format!("invalid C3 factor {name:?}"));
        }
    }
    Ok(())
}

fn cartesian(
    factors: &BTreeMap<String, Vec<FactorValue>>,
) -> Result<Vec<BTreeMap<String, FactorValue>>, String> {
    let mut combinations = vec![BTreeMap::new()];
    for (name, values) in factors {
        let mut expanded = Vec::with_capacity(combinations.len() * values.len());
        for combination in &combinations {
            for value in values {
                let mut candidate = combination.clone();
                candidate.insert(name.clone(), value.clone());
                expanded.push(candidate);
            }
        }
        combinations = expanded;
    }
    Ok(combinations)
}

fn pairwise_cover(
    factors: &BTreeMap<String, Vec<FactorValue>>,
) -> Result<Vec<BTreeMap<String, FactorValue>>, String> {
    if factors.len() <= 1 {
        return cartesian(factors);
    }
    let candidates = cartesian(factors)?;
    let factor_names = factors.keys().cloned().collect::<Vec<_>>();
    let mut uncovered = BTreeSet::new();
    for left in 0..factor_names.len() {
        for right in left + 1..factor_names.len() {
            for left_value in &factors[&factor_names[left]] {
                for right_value in &factors[&factor_names[right]] {
                    uncovered.insert((
                        factor_names[left].clone(),
                        left_value.clone(),
                        factor_names[right].clone(),
                        right_value.clone(),
                    ));
                }
            }
        }
    }
    let mut selected = Vec::new();
    while !uncovered.is_empty() {
        let candidate = candidates
            .iter()
            .max_by_key(|candidate| covered_pairs(candidate, &factor_names, &uncovered))
            .ok_or("pairwise expansion has no candidates")?;
        let score = covered_pairs(candidate, &factor_names, &uncovered);
        if score == 0 {
            return Err("pairwise expansion could not cover every factor pair".into());
        }
        selected.push(candidate.clone());
        remove_covered_pairs(candidate, &factor_names, &mut uncovered);
    }
    Ok(selected)
}

type FactorPair = (String, FactorValue, String, FactorValue);

fn covered_pairs(
    candidate: &BTreeMap<String, FactorValue>,
    names: &[String],
    uncovered: &BTreeSet<FactorPair>,
) -> usize {
    let mut count = 0;
    for left in 0..names.len() {
        for right in left + 1..names.len() {
            let pair = (
                names[left].clone(),
                candidate[&names[left]].clone(),
                names[right].clone(),
                candidate[&names[right]].clone(),
            );
            count += usize::from(uncovered.contains(&pair));
        }
    }
    count
}

fn remove_covered_pairs(
    candidate: &BTreeMap<String, FactorValue>,
    names: &[String],
    uncovered: &mut BTreeSet<FactorPair>,
) {
    for left in 0..names.len() {
        for right in left + 1..names.len() {
            uncovered.remove(&(
                names[left].clone(),
                candidate[&names[left]].clone(),
                names[right].clone(),
                candidate[&names[right]].clone(),
            ));
        }
    }
}

fn expand_shard(
    shard: &FactorShard,
    primary: &[BTreeMap<String, FactorValue>],
    secondary: &[BTreeMap<String, FactorValue>],
) -> Result<Vec<BenchmarkCell>, String> {
    let secondary = if secondary.is_empty() {
        vec![BTreeMap::new()]
    } else {
        secondary.to_vec()
    };
    let mut cells = Vec::new();
    let mut identities = BTreeMap::new();
    for operation in &shard.operations {
        for primary_case in primary {
            for secondary_case in &secondary {
                let factors = primary_case
                    .iter()
                    .chain(secondary_case)
                    .map(|(name, value)| (name.clone(), value.clone()))
                    .collect::<BTreeMap<_, _>>();
                let suffix = factor_suffix(&factors);
                let id = format!("{}-{}-{suffix}", shard.id, operation.id);
                if id.len() > 160 {
                    return Err(format!("generated C3 cell id exceeds 160 bytes: {id}"));
                }
                let scale = factor_usize(&factors, "scale")?;
                if operation
                    .maximum_scale
                    .is_some_and(|maximum| scale > maximum)
                {
                    continue;
                }
                let payload_bytes = optional_usize(&factors, "payload_bytes")?.unwrap_or(16);
                let dataset_size = optional_usize(&factors, "dataset_size")?;
                let factor_strategy = factors
                    .get("strategy")
                    .map(|value| value.as_text("strategy"))
                    .transpose()?;
                let strategy = factor_strategy
                    .map(str::to_owned)
                    .or_else(|| operation.strategy.clone())
                    .unwrap_or_else(|| operation.id.clone());
                identities.insert((operation.id.as_str(), suffix.clone()), id.clone());
                cells.push(BenchmarkCell {
                    id,
                    family: shard.family.clone(),
                    operation: operation.operation.clone(),
                    scale,
                    scale_unit: shard.scale_unit.clone(),
                    payload_bytes,
                    dataset_size,
                    baseline_cell: None,
                    curve: Some(format!("{}-{}", shard.id, operation.id)),
                    strategy: Some(strategy),
                });
            }
        }
    }
    for cell in &mut cells {
        let operation_id = cell
            .id
            .strip_prefix(&format!("{}-", shard.id))
            .and_then(|tail| {
                shard
                    .operations
                    .iter()
                    .find(|operation| tail.starts_with(&format!("{}-", operation.id)))
            })
            .expect("generated cell has a generated operation");
        if let Some(baseline) = &operation_id.baseline {
            let prefix = format!("{}-{}-", shard.id, operation_id.id);
            let suffix = cell.id.strip_prefix(&prefix).expect("generated prefix");
            cell.baseline_cell = identities
                .get(&(baseline.as_str(), suffix.to_owned()))
                .cloned();
        }
    }
    Ok(cells)
}

fn sample_cells(
    cells: &[BenchmarkCell],
    operations: &[FactorOperation],
    fraction: f64,
    maximum_scale: Option<usize>,
) -> Vec<BenchmarkCell> {
    let target = if fraction == 0.0 {
        operations.len()
    } else {
        ((cells.len() as f64 * fraction).ceil() as usize).max(operations.len())
    };
    let mut selected = BTreeSet::new();
    for (operation_index, operation) in operations.iter().enumerate() {
        let mut eligible = cells
            .iter()
            .enumerate()
            .filter(|(_, cell)| {
                cell.operation == operation.operation
                    && operation
                        .strategy
                        .as_deref()
                        .is_none_or(|strategy| cell.strategy.as_deref() == Some(strategy))
                    && maximum_scale.is_none_or(|maximum| cell.scale <= maximum)
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if eligible.is_empty() {
            eligible = cells
                .iter()
                .enumerate()
                .filter(|(_, cell)| {
                    cell.operation == operation.operation
                        && operation
                            .strategy
                            .as_deref()
                            .is_none_or(|strategy| cell.strategy.as_deref() == Some(strategy))
                })
                .map(|(index, _)| index)
                .collect();
        }
        if !eligible.is_empty() {
            let position = if fraction == 0.0 || operations.len() == 1 {
                0
            } else {
                operation_index * (eligible.len() - 1) / (operations.len() - 1)
            };
            selected.insert(eligible[position]);
        }
    }
    if target > selected.len() {
        let mut eligible = cells
            .iter()
            .enumerate()
            .filter(|(_, cell)| maximum_scale.is_none_or(|maximum| cell.scale <= maximum))
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if eligible.is_empty() {
            eligible.extend(0..cells.len());
        }
        if eligible.is_empty() {
            return Vec::new();
        }
        let stride = eligible.len() as f64 / target as f64;
        for sample in 0..target {
            selected.insert(
                eligible[((sample as f64 + 0.5) * stride).floor() as usize % eligible.len()],
            );
        }
    }
    let mut changed = true;
    while changed {
        changed = false;
        for index in selected.clone() {
            if let Some(baseline) = &cells[index].baseline_cell {
                if let Some(baseline_index) = cells.iter().position(|cell| &cell.id == baseline) {
                    changed |= selected.insert(baseline_index);
                }
            }
        }
    }
    selected
        .into_iter()
        .map(|index| cells[index].clone())
        .collect()
}

fn factor_suffix(factors: &BTreeMap<String, FactorValue>) -> String {
    factors
        .iter()
        .map(|(name, value)| {
            let short = match name.as_str() {
                "scale" => "n",
                "payload_bytes" => "p",
                "dataset_size" => "d",
                "strategy" => "s",
                other => other,
            };
            format!("{short}{}", value.id_fragment())
        })
        .collect::<Vec<_>>()
        .join("-")
}

fn factor_usize(factors: &BTreeMap<String, FactorValue>, name: &str) -> Result<usize, String> {
    factors
        .get(name)
        .ok_or_else(|| format!("missing required C3 factor {name:?}"))?
        .as_usize(name)
}

fn optional_usize(
    factors: &BTreeMap<String, FactorValue>,
    name: &str,
) -> Result<Option<usize>, String> {
    factors
        .get(name)
        .map(|value| value.as_usize(name))
        .transpose()
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

pub fn generated_paths(output_directory: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths = fs::read_dir(output_directory)
        .map_err(|error| format!("read {}: {error}", output_directory.display()))?
        .map(|entry| {
            entry
                .map(|value| value.path())
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairwise_cover_contains_every_secondary_pair() {
        let factors = BTreeMap::from([
            (
                "payload_bytes".into(),
                vec![FactorValue::Integer(1), FactorValue::Integer(16)],
            ),
            (
                "dataset_size".into(),
                vec![FactorValue::Integer(64), FactorValue::Integer(128)],
            ),
            (
                "strategy".into(),
                vec![FactorValue::Text("a".into()), FactorValue::Text("b".into())],
            ),
        ]);
        let selected = pairwise_cover(&factors).unwrap();
        assert!(selected.len() < 8);
        let names = factors.keys().cloned().collect::<Vec<_>>();
        let mut uncovered = pairwise_pairs(&factors, &names);
        for candidate in &selected {
            remove_covered_pairs(candidate, &names, &mut uncovered);
        }
        assert!(uncovered.is_empty());
    }

    #[test]
    fn checked_in_p0_manifests_are_byte_reproducible() {
        assert_checked_in_manifests(
            Path::new("../../validation/benchmarks/c3-p0-factor-plan-v1.json"),
            Path::new("../../validation/benchmarks/manifests/c3-p0"),
            "p0",
        );
    }

    #[test]
    fn checked_in_f3_s3_manifests_are_byte_reproducible() {
        assert_checked_in_manifests(
            Path::new("../../validation/benchmarks/c3-f3-s3-factor-plan-v1.json"),
            Path::new("../../validation/benchmarks/manifests/c3-f3-s3"),
            "f3-s3",
        );
    }

    #[test]
    fn checked_in_t1_r1_d1_manifests_are_byte_reproducible() {
        assert_checked_in_manifests(
            Path::new("../../validation/benchmarks/c3-t1-r1-d1-factor-plan-v1.json"),
            Path::new("../../validation/benchmarks/manifests/c3-t1-r1-d1"),
            "t1-r1-d1",
        );
    }

    #[test]
    fn checked_in_g1_g2_manifests_are_byte_reproducible() {
        assert_checked_in_manifests(
            Path::new("../../validation/benchmarks/c3-g1-g2-factor-plan-v1.json"),
            Path::new("../../validation/benchmarks/manifests/c3-g1-g2"),
            "g1-g2",
        );
    }

    #[test]
    fn checked_in_x2_manifests_are_byte_reproducible() {
        assert_checked_in_manifests(
            Path::new("../../validation/benchmarks/c3-x2-factor-plan-v1.json"),
            Path::new("../../validation/benchmarks/manifests/c3-x2"),
            "x2",
        );
    }

    #[test]
    fn checked_in_f1_f2_closure_manifests_are_byte_reproducible() {
        assert_checked_in_manifests(
            Path::new("../../validation/benchmarks/c3-f1-f2-closure-factor-plan-v1.json"),
            Path::new("../../validation/benchmarks/manifests/c3-f1-f2-closure"),
            "f1-f2-closure",
        );
    }

    fn assert_checked_in_manifests(plan: &Path, checked_in: &Path, label: &str) {
        let generated = std::env::temp_dir().join(format!(
            "algesum-c3-expand-{label}-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        if generated.exists() {
            fs::remove_dir_all(&generated).unwrap();
        }
        expand_plan(plan, &generated).unwrap();
        for path in generated_paths(&generated).unwrap() {
            let name = path.file_name().unwrap();
            assert_eq!(
                fs::read(&path).unwrap(),
                fs::read(checked_in.join(name)).unwrap(),
                "generated C3 artifact drifted: {}",
                name.to_string_lossy()
            );
            if name != "c3-expansion-report-v1.json" {
                crate::publication::load_manifest(&path).unwrap_or_else(|error| {
                    panic!("generated manifest {} is invalid: {error}", path.display())
                });
            }
        }
        fs::remove_dir_all(generated).unwrap();
    }

    fn pairwise_pairs(
        factors: &BTreeMap<String, Vec<FactorValue>>,
        names: &[String],
    ) -> BTreeSet<FactorPair> {
        let mut pairs = BTreeSet::new();
        for left in 0..names.len() {
            for right in left + 1..names.len() {
                for left_value in &factors[&names[left]] {
                    for right_value in &factors[&names[right]] {
                        pairs.insert((
                            names[left].clone(),
                            left_value.clone(),
                            names[right].clone(),
                            right_value.clone(),
                        ));
                    }
                }
            }
        }
        pairs
    }
}
