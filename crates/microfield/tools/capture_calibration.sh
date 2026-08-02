#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$repo_root"

if [[ "$#" -lt 1 || "$#" -gt 2 ]]; then
  echo "uso: $0 OUTPUT_DIRECTORY [CRITERION_FILTER]" >&2
  exit 2
fi

output="$1"
filter="${2:-}"
sample_size="${MICROFIELD_CALIBRATION_SAMPLES:-20}"
warm_up="${MICROFIELD_CALIBRATION_WARMUP_SECONDS:-1}"
measurement="${MICROFIELD_CALIBRATION_MEASUREMENT_SECONDS:-2}"
target_dir="${MICROFIELD_CALIBRATION_TARGET_DIR:-$repo_root/target/calibration}"

if [[ -e "$output" ]] && find "$output" -mindepth 1 -print -quit | grep -q .; then
  echo "el directorio de captura debe estar vacío: $output" >&2
  exit 1
fi
mkdir -p "$output/environment" "$output/criterion"

command=(
  cargo +stable bench -p microfield --bench portable_batch --
)
if [[ -n "$filter" ]]; then
  command+=("$filter")
fi
command+=(
  --noplot
  --sample-size "$sample_size"
  --warm-up-time "$warm_up"
  --measurement-time "$measurement"
)

printf '%q ' "${command[@]}" > "$output/command.txt"
printf '\n' >> "$output/command.txt"

CARGO_TARGET_DIR="$target_dir" "${command[@]}"

criterion_root="$target_dir/criterion"
if [[ ! -d "$criterion_root" ]]; then
  echo "Criterion no produjo resultados en $criterion_root" >&2
  exit 1
fi

while IFS= read -r -d '' estimate; do
  relative="${estimate#"$criterion_root"/}"
  destination="$output/criterion/$relative"
  mkdir -p "$(dirname "$destination")"
  cp "$estimate" "$destination"
done < <(find "$criterion_root" -type f -path '*/new/estimates.json' -print0 | sort -z)

if ! find "$output/criterion" -type f -name estimates.json -print -quit | grep -q .; then
  echo "no se capturó ningún estimates.json" >&2
  exit 1
fi

rustc_vv="$(rustc +stable -vV)"
printf '%s\n' "$rustc_vv" > "$output/environment/rustc-vV.txt"
uname -a > "$output/environment/uname.txt"
if command -v lscpu >/dev/null 2>&1; then
  lscpu --json > "$output/environment/lscpu.json"
else
  printf '{"lscpu":null}\n' > "$output/environment/lscpu.json"
fi

architecture="$(uname -m)"
cpu_model="$(awk -F: '/model name|Model/ { sub(/^[[:space:]]+/, "", $2); print $2; exit }' /proc/cpuinfo 2>/dev/null || true)"
cpu_model="${cpu_model:-unknown}"
cpu_family="$(awk -F: '/cpu family/ { gsub(/[[:space:]]/, "", $2); print $2; exit }' /proc/cpuinfo 2>/dev/null || true)"
cpu_part="$(awk -F: '/CPU part/ { gsub(/[[:space:]]/, "", $2); print $2; exit }' /proc/cpuinfo 2>/dev/null || true)"
cpu_family="${cpu_family:-${cpu_part:-unknown}}"
microcode="$(awk -F: '/microcode/ { gsub(/[[:space:]]/, "", $2); print $2; exit }' /proc/cpuinfo 2>/dev/null || true)"
microcode="${microcode:-unknown}"
rustc_release="$(printf '%s\n' "$rustc_vv" | awk -F': ' '/^release:/ { print $2 }')"
llvm_version="$(printf '%s\n' "$rustc_vv" | awk -F': ' '/^LLVM version:/ { print $2 }')"
source_commit="$(git rev-parse HEAD)"
captured_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
kernel="$(uname -r)"
os="$(awk -F= '/^PRETTY_NAME=/ { value=$2; gsub(/^"|"$/, "", value); print value }' /etc/os-release 2>/dev/null || true)"
os="${os:-unknown}"

jq -n \
  --arg captured_at_utc "$captured_at" \
  --arg source_commit "$source_commit" \
  --arg architecture "$architecture" \
  --arg cpu_family "$cpu_family" \
  --arg cpu_model "$cpu_model" \
  --arg microcode "$microcode" \
  --arg os "$os" \
  --arg kernel "$kernel" \
  --arg rustc "$rustc_release" \
  --arg llvm "$llvm_version" \
  --arg filter "$filter" \
  --argjson sample_size "$sample_size" \
  --argjson warm_up_seconds "$warm_up" \
  --argjson measurement_seconds "$measurement" \
  '{
    schema: 1,
    captured_at_utc: $captured_at_utc,
    source_commit: $source_commit,
    environment: {
      architecture: $architecture,
      cpu_family: $cpu_family,
      cpu_model: $cpu_model,
      microcode: $microcode,
      os: $os,
      kernel: $kernel,
      rustc: $rustc,
      llvm: $llvm,
      profile: "bench"
    },
    criterion: {
      filter: $filter,
      sample_size: $sample_size,
      warm_up_seconds: $warm_up_seconds,
      measurement_seconds: $measurement_seconds
    }
  }' > "$output/capture.json"

(
  cd "$output"
  find . -type f ! -name SHA256SUMS -print0 | sort -z | xargs -0 sha256sum
) > "$output/SHA256SUMS"

echo "captura reproducible escrita en $output"
