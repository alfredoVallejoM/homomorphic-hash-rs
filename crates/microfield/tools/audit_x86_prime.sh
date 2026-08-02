#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$repo_root"

cargo bench -p microfield --bench prime_fields --no-run

binary="$(find target/release/deps -maxdepth 1 -type f -name 'prime_fields-*' -executable -printf '%T@ %p\n' \
  | sort -nr | head -n 1 | cut -d' ' -f2-)"
if [[ -z "$binary" ]]; then
  echo "no se encontró el ejecutable de auditoría prime_fields" >&2
  exit 1
fi

audit_dir="$(mktemp -d)"
trap 'rm -rf "$audit_dir"' EXIT
objdump -Cd "$binary" > "$audit_dir/full.asm"

awk '
  /^[[:xdigit:]]+ <.*microfield::backend::x86_prime::/ { in_backend = 1 }
  in_backend && /^$/ { in_backend = 0 }
  in_backend { print }
' "$audit_dir/full.asm" > "$audit_dir/backend.asm"

test -s "$audit_dir/backend.asm"
grep -Eq '\bvpmullw\b' "$audit_dir/backend.asm"
grep -Eq '\bvpmulhuw\b' "$audit_dir/backend.asm"
grep -Eq '\bvpackuswb\b' "$audit_dir/backend.asm"
grep -Eq '\bvzeroupper\b' "$audit_dir/backend.asm"
grep -Eq '\bmulx\b' "$audit_dir/backend.asm"

if grep -Eq 'call[q]?[[:space:]].*\*|__rust_alloc|<.*alloc::|\b(idiv|div)[bwlq]?\b' \
  "$audit_dir/backend.asm"; then
  echo "los kernels primos contienen dispatch interno, división o una referencia al asignador" >&2
  exit 1
fi

instruction_count="$(grep -Ec '^[[:space:]]*[[:xdigit:]]+:' "$audit_dir/backend.asm")"
if [[ "$instruction_count" -gt 8000 ]]; then
  echo "los kernels primos superan el presupuesto estructural de 8000 instrucciones" >&2
  exit 1
fi

echo "auditoría prima x86 correcta: AVX2 widen/reduce/pack y BMI2 MULX; sin dispatch, división ni asignador"
