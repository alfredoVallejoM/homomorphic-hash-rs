#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$repo_root"

cargo bench -p microfield --bench prime_fields --no-run
cargo test -p microfield --all-features --test external_prime_avx2_bridge --release --no-run

binary="$(find target/release/deps -maxdepth 1 -type f -name 'prime_fields-*' -executable -printf '%T@ %p\n' \
  | sort -nr | head -n 1 | cut -d' ' -f2-)"
if [[ -z "$binary" ]]; then
  echo "no se encontró el ejecutable de auditoría prime_fields" >&2
  exit 1
fi

audit_dir="$(mktemp -d)"
trap 'rm -rf "$audit_dir"' EXIT
objdump -Cd "$binary" > "$audit_dir/full.asm"

bridge_binary="$(find target/release/deps -maxdepth 1 -type f \
  -name 'external_prime_avx2_bridge-*' -executable -printf '%T@ %p\n' \
  | sort -nr | head -n 1 | cut -d' ' -f2-)"
if [[ -z "$bridge_binary" ]]; then
  echo "no se encontró el ejecutable del bridge primo AVX2" >&2
  exit 1
fi
objdump -Cd "$bridge_binary" > "$audit_dir/bridge-full.asm"

awk '
  /^[[:xdigit:]]+ <.*microfield::backend::x86_prime::/ { in_backend = 1 }
  in_backend && /^$/ { in_backend = 0 }
  in_backend { print }
' "$audit_dir/full.asm" > "$audit_dir/backend.asm"

awk '
  /^[[:xdigit:]]+ <.*microfield::backend::x86_prime::/ { in_backend = 1 }
  in_backend && /^$/ { in_backend = 0 }
  in_backend { print }
' "$audit_dir/bridge-full.asm" > "$audit_dir/bridge-backend.asm"

awk '
  /^[[:xdigit:]]+ <.*microfield::backend::x86_prime::packed(8|16|32)_(binary|assign)_impl::/ { in_packed = 1 }
  in_packed && /^$/ { in_packed = 0 }
  in_packed { print }
' "$audit_dir/bridge-full.asm" > "$audit_dir/bridge-packed-hot.asm"

awk '
  /^[[:xdigit:]]+ <.*microfield::backend::x86_prime::bmi2_mul::<.*>>:$/ { in_element = 1 }
  in_element && /^$/ { in_element = 0 }
  in_element { print }
' "$audit_dir/full.asm" > "$audit_dir/bmi2-element.asm"

awk '
  /^[[:xdigit:]]+ <.*microfield::prime::montgomery::montgomery_reduce_wide::<4, 8>>:$/ { in_redc = 1 }
  in_redc && /^$/ { in_redc = 0 }
  in_redc { print }
' "$audit_dir/full.asm" > "$audit_dir/montgomery-redc.asm"

awk '
  /^[[:xdigit:]]+ <.*microfield::backend::x86_prime::add_radix64::<.*>>:$/ { in_add = 1 }
  in_add && /^$/ { in_add = 0 }
  in_add { print }
' "$audit_dir/full.asm" > "$audit_dir/radix64-add.asm"

test -s "$audit_dir/backend.asm"
test -s "$audit_dir/bmi2-element.asm"
test -s "$audit_dir/montgomery-redc.asm"
test -s "$audit_dir/radix64-add.asm"
test -s "$audit_dir/bridge-backend.asm"
test -s "$audit_dir/bridge-packed-hot.asm"
grep -Eq '\bvpmullw\b' "$audit_dir/backend.asm"
grep -Eq '\bvpmulhuw\b' "$audit_dir/backend.asm"
grep -Eq '\bvpackuswb\b' "$audit_dir/backend.asm"
grep -Eq '\bvpmuludq\b' "$audit_dir/backend.asm"
grep -Eq '\bvpcmpgtq\b' "$audit_dir/backend.asm"
grep -Eq '\bvpsllq\b' "$audit_dir/backend.asm"
grep -Eq '\bvpsrlq\b' "$audit_dir/backend.asm"
grep -Eq '\bvzeroupper\b' "$audit_dir/backend.asm"
grep -Eq '\bmulx\b' "$audit_dir/backend.asm"
grep -Eq '\bvpmulld\b' "$audit_dir/bridge-backend.asm"
grep -Eq '\bvpackusdw\b' "$audit_dir/bridge-backend.asm"
grep -Eq '\bvpmuludq\b' "$audit_dir/bridge-backend.asm"
grep -Eq '\bvpblendd\b' "$audit_dir/bridge-backend.asm"
grep -Eq '\bvzeroupper\b' "$audit_dir/bridge-backend.asm"

# The four-limb product contributes N² MULX operations. Conditional jumps
# inside this single-element boundary would reintroduce value-dependent carry
# propagation or correction.
mulx_count="$(grep -Ec '\bmulx\b' "$audit_dir/bmi2-element.asm")"
if [[ "$mulx_count" -lt 16 ]]; then
  echo "el elemento BMI2 no usa MULX en el producto ancho completo" >&2
  exit 1
fi
if grep -Eq '\bj(a|ae|b|be|c|cxz|e|ecxz|g|ge|l|le|na|nae|nb|nbe|nc|ne|ng|nge|nl|nle|no|np|ns|nz|o|p|pe|po|rcxz|s|z)\b' \
  "$audit_dir/bmi2-element.asm" "$audit_dir/montgomery-redc.asm"; then
  echo "el elemento BMI2 contiene un salto condicional dependiente del valor" >&2
  exit 1
fi
add_jump_count="$(grep -Ec '\bj(a|ae|b|be|c|cxz|e|ecxz|g|ge|l|le|na|nae|nb|nbe|nc|ne|ng|nge|nl|nle|no|np|ns|nz|o|p|pe|po|rcxz|s|z)\b' "$audit_dir/radix64-add.asm")"
if [[ "$add_jump_count" -gt 2 ]]; then
  echo "la suma radix-64 contiene más saltos que entrada vacía y bucle batch" >&2
  exit 1
fi

forbidden_hot="$({
  grep -Eh 'call[q]?[[:space:]].*\*|__rust_alloc|<.*alloc::|\b(idiv|div)[bwlq]?\b' \
    "$audit_dir/backend.asm" "$audit_dir/bridge-packed-hot.asm" || true
} | grep -Ev '<memset@' || true)"
if [[ -n "$forbidden_hot" ]]; then
  echo "los kernels primos contienen dispatch interno, división o una referencia al asignador" >&2
  exit 1
fi

instruction_count="$(grep -Ec '^[[:space:]]*[[:xdigit:]]+:' "$audit_dir/backend.asm")"
if [[ "$instruction_count" -gt 8000 ]]; then
  echo "los kernels primos superan el presupuesto estructural de 8000 instrucciones" >&2
  exit 1
fi

echo "auditoría prima x86 correcta: AVX2 packed u8/u16/u32, Goldilocks y BMI2 sin dispatch interno"
