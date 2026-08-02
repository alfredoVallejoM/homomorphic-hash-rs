#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$repo_root"

calibration="${MICROFIELD_CALIBRATION_DIR:-crates/microfield/calibration}"
table="$calibration/selection-table-v1.csv"
profiles=("$calibration"/profiles/*.json)

if [[ "$(head -n 1 "$table")" != \
  "selection_table_version,field,backend,minimum_batch,automatic_selection,evidence_profile,conservative_improvement_percent,reason" ]]; then
  echo "cabecera inesperada en la tabla de selección" >&2
  exit 1
fi

row_count="$(tail -n +2 "$table" | sed '/^[[:space:]]*$/d' | wc -l)"
if [[ "$row_count" -ne 9 ]]; then
  echo "la tabla debe contener exactamente nueve decisiones, contiene $row_count" >&2
  exit 1
fi

duplicates="$(tail -n +2 "$table" | cut -d, -f2,3 | sort | uniq -d)"
if [[ -n "$duplicates" ]]; then
  echo "hay decisiones de campo/backend duplicadas: $duplicates" >&2
  exit 1
fi

for profile in "${profiles[@]}"; do
  jq -e '
    .schema == 1 and
    (.profile_id | type == "string" and length > 0) and
    (.source_commit | test("^[0-9a-f]{40}$")) and
    (.environment.architecture == "x86_64" or .environment.architecture == "aarch64") and
    (.environment.cpu_family | type == "string" and length > 0) and
    (.environment.cpu_model | type == "string" and length > 0) and
    (.environment.microcode | type == "string" and length > 0) and
    (.environment.profile == "bench") and
    (.criterion.minimum_samples >= 10) and
    (.criterion.warm_up_seconds > 0) and
    (.criterion.measurement_seconds > 0) and
    (.measurements | length > 0) and
    all(.measurements[];
      (.batch_len >= 1) and
      (.lower_ns > 0) and
      (.upper_ns >= .lower_ns)
    )
  ' "$profile" >/dev/null

  profile_id="$(jq -r '.profile_id' "$profile")"
  if [[ "$(basename "$profile" .json)" != "$profile_id" ]]; then
    echo "el nombre de $profile no coincide con profile_id=$profile_id" >&2
    exit 1
  fi

  duplicate_measurements="$(
    jq -r '.measurements[] | [.field,.operation,.region,.batch_len,.backend] | @csv' "$profile" \
      | sort | uniq -d
  )"
  if [[ -n "$duplicate_measurements" ]]; then
    echo "mediciones duplicadas en $profile: $duplicate_measurements" >&2
    exit 1
  fi
done

while IFS=, read -r version field backend minimum automatic evidence improvement reason; do
  [[ "$version" == "1" ]] || { echo "versión de tabla no soportada: $version" >&2; exit 1; }
  case "$field" in
    gf2_128_v1|gf2_256_hh_v1|gf2_256_alt_v1) ;;
    *) echo "campo desconocido en calibración: $field" >&2; exit 1 ;;
  esac
  case "$backend" in
    x86_pclmul|x86_vpclmul|aarch64_pmull) ;;
    *) echo "backend desconocido en calibración: $backend" >&2; exit 1 ;;
  esac
  [[ "$automatic" == "true" || "$automatic" == "false" ]] || {
    echo "automatic_selection inválido para $field/$backend" >&2
    exit 1
  }
  [[ "$minimum" == "none" || "$minimum" =~ ^[0-9]+$ ]] || {
    echo "minimum_batch inválido para $field/$backend" >&2
    exit 1
  }
  [[ -n "$reason" ]] || { echo "falta razón para $field/$backend" >&2; exit 1; }

  if [[ "$evidence" != arm64-native-functional-* ]]; then
    [[ -f "$calibration/profiles/$evidence.json" ]] || {
      echo "perfil de evidencia inexistente: $evidence" >&2
      exit 1
    }
  fi
  if [[ "$automatic" == "true" ]]; then
    case "$backend" in
      x86_pclmul|aarch64_pmull) baseline="portable" ;;
      x86_vpclmul) baseline="x86_pclmul" ;;
    esac
    computed_gain="$(
      jq -r \
        --arg field "$field" \
        --arg backend "$backend" \
        --arg baseline "$baseline" \
        --argjson batch "$minimum" '
          [
            .measurements[] as $base |
            select(
              $base.field == $field and
              $base.backend == $baseline and
              $base.batch_len == $batch and
              $base.region == "engine" and
              ($base.operation == "mul" or $base.operation == "square")
            ) |
            .measurements[] as $candidate |
            select(
              $candidate.field == $field and
              $candidate.backend == $backend and
              $candidate.batch_len == $base.batch_len and
              $candidate.region == $base.region and
              $candidate.operation == $base.operation
            ) |
            {
              operation: $candidate.operation,
              gain: (100 * ($base.lower_ns - $candidate.upper_ns) / $base.lower_ns)
            }
          ] |
          select((map(.operation) | unique | sort) == ["mul", "square"]) |
          map(.gain) | min
        ' "$calibration/profiles/$evidence.json"
    )"
    [[ -n "$computed_gain" ]] || {
      echo "faltan pares mul/square de evidencia para $field/$backend" >&2
      exit 1
    }
    awk -v declared="$improvement" -v computed="$computed_gain" '
      BEGIN { exit !(declared + 0 >= 20 && computed + 0 >= 20 && declared <= computed + 0.1) }
    ' || {
      echo "selección automática sin mejora conservadora del 20 %: $field/$backend" >&2
      exit 1
    }

    if [[ "$backend" == "x86_vpclmul" ]]; then
      pipeline_gain="$(
        jq -r \
          --arg field "$field" \
          --arg backend "$backend" \
          --arg baseline "$baseline" \
          --argjson batch "$minimum" '
            .measurements as $all |
            [
              $all[] as $base |
              select(
                $base.field == $field and
                $base.backend == $baseline and
                $base.batch_len == $batch and
                $base.operation == "mul" and
                $base.region == "pipeline_reused"
              ) |
              $all[] as $candidate |
              select(
                $candidate.field == $field and
                $candidate.backend == $backend and
                $candidate.batch_len == $base.batch_len and
                $candidate.operation == $base.operation and
                $candidate.region == $base.region
              ) |
              100 * ($base.lower_ns - $candidate.upper_ns) / $base.lower_ns
            ] | select(length > 0) | min
          ' "$calibration/profiles/$evidence.json"
      )"
      awk -v gain="$pipeline_gain" 'BEGIN { exit !(gain + 0 >= 20) }' || {
        echo "VPCLMUL automático carece de mejora del 20 % con packing" >&2
        exit 1
      }
    fi
  fi
done < <(tail -n +2 "$table")

if rg -q ',x86_vpclmul,[^,]+,true,' "$table"; then
  x86_families="$(
    jq -r 'select(.environment.architecture == "x86_64") |
      select(any(.measurements[]; .backend == "x86_vpclmul")) |
      .environment.cpu_family' "${profiles[@]}" | sort -u | wc -l
  )"
  [[ "$x86_families" -ge 2 ]] || {
    echo "VPCLMUL automático requiere dos familias x86-64; hay $x86_families" >&2
    exit 1
  }
fi

if rg -q ',aarch64_pmull,[^,]+,true,' "$table"; then
  arm_families="$(
    jq -r 'select(.environment.architecture == "aarch64") |
      select(any(.measurements[]; .backend == "aarch64_pmull")) |
      .environment.cpu_family' "${profiles[@]}" | sort -u | wc -l
  )"
  [[ "$arm_families" -ge 2 ]] || {
    echo "PMULL automático requiere dos familias AArch64; hay $arm_families" >&2
    exit 1
  }
fi

echo "calibración v1 correcta: nueve decisiones, perfiles válidos y promoción conservadora"
