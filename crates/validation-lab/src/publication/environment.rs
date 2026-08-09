use std::{collections::BTreeMap, env, fs, path::Path, process::Command};

use sha2::{Digest, Sha256};

use super::model::{
    BenchmarkManifest, CampaignProfile, EnvironmentClassification, EnvironmentReport, HostMode,
};

pub fn capture(manifest: &BenchmarkManifest) -> EnvironmentReport {
    let affinity = proc_status_value("Cpus_allowed_list").unwrap_or_else(|| "unknown".into());
    let tree_clean = command_allow_empty("git", &["status", "--porcelain"])
        .is_some_and(|output| output.is_empty());
    let release_profile = !cfg!(debug_assertions);
    let mut unknowns = Vec::new();
    let cpu = cpu_information();
    let memory = key_value_file(
        Path::new("/proc/meminfo"),
        &["MemTotal", "MemAvailable", "SwapTotal", "HugePages_Total"],
        ':',
    );
    let frequency = frequency_information();
    for (name, known) in [
        (
            "cpu-model",
            cpu.contains_key("model name") || cpu.contains_key("Processor"),
        ),
        ("microcode", cpu.contains_key("microcode")),
        ("cpu-affinity", affinity != "unknown"),
        (
            "frequency-governor",
            frequency.contains_key("scaling_governor"),
        ),
        ("turbo", frequency.contains_key("turbo")),
    ] {
        if !known {
            unknowns.push(name.into());
        }
    }
    let classification = classify(
        manifest,
        &affinity,
        &frequency,
        tree_clean,
        release_profile,
        &mut unknowns,
    );
    EnvironmentReport {
        schema: "microfield-publication-environment-v1".into(),
        classification,
        host_mode: manifest.host_mode,
        commit: command("git", &["rev-parse", "HEAD"]),
        tree_clean,
        binary_sha256: binary_sha256(),
        profile: if cfg!(debug_assertions) {
            "debug".into()
        } else {
            "release".into()
        },
        architecture: env::consts::ARCH.into(),
        operating_system: os_release(),
        kernel: command("uname", &["-srvo"]),
        rustc: command("rustc", &["-vV"]),
        cargo: command("cargo", &["-V"]),
        rustflags: env::var("RUSTFLAGS").unwrap_or_default(),
        cpu,
        memory,
        frequency,
        temperature: temperature_information(),
        filesystem: filesystem_information(),
        affinity,
        container: container_kind(),
        utc_started: command("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"]),
        disclosed_unknowns: unknowns,
    }
}

fn classify(
    manifest: &BenchmarkManifest,
    affinity: &str,
    frequency: &BTreeMap<String, String>,
    tree_clean: bool,
    release_profile: bool,
    unknowns: &mut Vec<String>,
) -> EnvironmentClassification {
    if manifest.profile == CampaignProfile::Smoke || manifest.host_mode == HostMode::CiShared {
        return EnvironmentClassification::Smoke;
    }
    let dedicated_attested = env::var("MICROFIELD_BENCH_DEDICATED").as_deref() == Ok("1");
    let affinity_attested = env::var("MICROFIELD_BENCH_AFFINITY_FIXED").as_deref() == Ok("1");
    if manifest.host_mode == HostMode::Dedicated
        && dedicated_attested
        && affinity_attested
        && affinity != "unknown"
        && frequency.contains_key("scaling_governor")
        && tree_clean
        && release_profile
    {
        EnvironmentClassification::Controlled
    } else {
        if manifest.host_mode == HostMode::Dedicated && !dedicated_attested {
            unknowns.push("dedicated-host-attestation".into());
        }
        if manifest.host_mode == HostMode::Dedicated && !affinity_attested {
            unknowns.push("fixed-affinity-attestation".into());
        }
        if !tree_clean {
            unknowns.push("dirty-source-tree".into());
        }
        if !release_profile {
            unknowns.push("non-release-binary".into());
        }
        EnvironmentClassification::Informative
    }
}

fn command(program: &str, arguments: &[&str]) -> String {
    Command::new(program)
        .args(arguments)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|output| !output.is_empty())
        .unwrap_or_else(|| "unknown".into())
}

fn command_allow_empty(program: &str, arguments: &[&str]) -> Option<String> {
    Command::new(program)
        .args(arguments)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn os_release() -> String {
    fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|contents| {
            contents
                .lines()
                .find_map(|line| line.strip_prefix("PRETTY_NAME="))
                .map(|value| value.trim_matches('"').to_owned())
        })
        .unwrap_or_else(|| env::consts::OS.into())
}

fn cpu_information() -> BTreeMap<String, String> {
    key_value_file(
        Path::new("/proc/cpuinfo"),
        &[
            "vendor_id",
            "model name",
            "Processor",
            "CPU implementer",
            "CPU part",
            "microcode",
            "cpu cores",
            "siblings",
            "cache size",
            "flags",
            "Features",
        ],
        ':',
    )
}

fn key_value_file(path: &Path, wanted: &[&str], separator: char) -> BTreeMap<String, String> {
    let Ok(contents) = fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    let mut values = BTreeMap::new();
    for line in contents.lines() {
        let Some((key, value)) = line.split_once(separator) else {
            continue;
        };
        let key = key.trim();
        if wanted.contains(&key) && !values.contains_key(key) {
            values.insert(key.to_owned(), value.trim().to_owned());
        }
    }
    values
}

fn frequency_information() -> BTreeMap<String, String> {
    let base = Path::new("/sys/devices/system/cpu/cpu0/cpufreq");
    let mut values = BTreeMap::new();
    for name in [
        "scaling_driver",
        "scaling_governor",
        "scaling_min_freq",
        "scaling_max_freq",
        "scaling_cur_freq",
        "cpuinfo_min_freq",
        "cpuinfo_max_freq",
    ] {
        if let Ok(value) = fs::read_to_string(base.join(name)) {
            values.insert(name.into(), value.trim().into());
        }
    }
    for path in [
        "/sys/devices/system/cpu/intel_pstate/no_turbo",
        "/sys/devices/system/cpu/cpufreq/boost",
    ] {
        if let Ok(value) = fs::read_to_string(path) {
            values.insert("turbo".into(), format!("{}={}", path, value.trim()));
            break;
        }
    }
    values
}

fn temperature_information() -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    let Ok(entries) = fs::read_dir("/sys/class/thermal") else {
        return values;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("thermal_zone") {
            continue;
        }
        let path = entry.path();
        let Ok(temperature) = fs::read_to_string(path.join("temp")) else {
            continue;
        };
        let label = fs::read_to_string(path.join("type"))
            .map(|value| value.trim().to_owned())
            .unwrap_or(name);
        values.insert(label, temperature.trim().into());
    }
    values
}

fn filesystem_information() -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    values.insert("working-directory".into(), command("df", &["-T", "."]));
    values.insert("page-size".into(), command("getconf", &["PAGE_SIZE"]));
    values
}

fn proc_status_value(key: &str) -> Option<String> {
    fs::read_to_string("/proc/self/status")
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix(key))
        .and_then(|line| line.strip_prefix(':'))
        .map(str::trim)
        .map(str::to_owned)
}

fn binary_sha256() -> String {
    env::current_exe()
        .ok()
        .and_then(|path| fs::read(path).ok())
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .unwrap_or_else(|| "unknown".into())
}

fn container_kind() -> String {
    if Path::new("/.dockerenv").exists() {
        return "docker-compatible".into();
    }
    fs::read_to_string("/proc/1/cgroup")
        .ok()
        .filter(|contents| contents.contains("docker") || contents.contains("kubepods"))
        .map_or_else(|| "not-detected".into(), |_| "cgroup-container".into())
}
