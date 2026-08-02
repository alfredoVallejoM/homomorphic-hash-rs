#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$repo_root"

audit="$repo_root/crates/microfield/tools/audit_calibration.sh"
source_calibration="$repo_root/crates/microfield/calibration"
test_root="$(mktemp -d)"
trap 'rm -rf "$test_root"' EXIT

new_case() {
  local name="$1"
  local target="$test_root/$name"
  cp -R "$source_calibration" "$target"
  printf '%s\n' "$target"
}

expect_rejected() {
  local target="$1"
  local description="$2"
  if MICROFIELD_CALIBRATION_DIR="$target" bash "$audit" >/dev/null 2>&1; then
    echo "el gate aceptó una mutación inválida: $description" >&2
    exit 1
  fi
}

valid="$(new_case valid)"
MICROFIELD_CALIBRATION_DIR="$valid" bash "$audit" >/dev/null

bad_interval="$(new_case bad-interval)"
jq '(.measurements[0].lower_ns) = -1' \
  "$bad_interval/profiles/intel-i7-13700hx-2026-08-02.json" \
  > "$bad_interval/profile.tmp"
mv "$bad_interval/profile.tmp" \
  "$bad_interval/profiles/intel-i7-13700hx-2026-08-02.json"
expect_rejected "$bad_interval" "intervalo negativo"

inflated_gain="$(new_case inflated-gain)"
sed -i 's/,35\.7,portable_crossover_certified$/,99.0,portable_crossover_certified/' \
  "$inflated_gain/selection-table-v1.csv"
expect_rejected "$inflated_gain" "mejora declarada superior a la evidencia"

uncalibrated_auto="$(new_case uncalibrated-auto)"
sed -i 's/,x86_vpclmul,64,false,/,x86_vpclmul,64,true,/' \
  "$uncalibrated_auto/selection-table-v1.csv"
expect_rejected "$uncalibrated_auto" "VPCLMUL automático sin cobertura"

duplicate="$(new_case duplicate)"
tail -n 1 "$duplicate/selection-table-v1.csv" >> "$duplicate/selection-table-v1.csv"
expect_rejected "$duplicate" "decisión duplicada"

invalid_goldilocks="$(new_case invalid-goldilocks)"
sed -i 's/,mul,4,/,mul,8,/' \
  "$invalid_goldilocks/phase46-simd-i7-13700hx-2026-08-02.csv"
expect_rejected "$invalid_goldilocks" "umbral Goldilocks distinto del compilado"

invalid_packed="$(new_case invalid-packed)"
sed -i 's/,16384,13426,13832,3573.9,3651.6,/,16384,13426,13832,3573.9,13000,/' \
  "$invalid_packed/phase47-packed-i7-13700hx-2026-08-02.csv"
expect_rejected "$invalid_packed" "ganancia packed inferior al gate declarado"

echo "autopruebas de calibración correctas: seis mutaciones inválidas rechazadas"
