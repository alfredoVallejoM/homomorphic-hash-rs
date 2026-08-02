#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$repo_root"

if [[ "${MICROFIELD_AARCH64_BUILD_STD:-0}" == "1" ]]; then
  cargo +nightly rustc -p microfield --lib --release \
    --target aarch64-unknown-linux-gnu -Z build-std=core,alloc \
    --no-default-features --features portable,builtin-fields -- --emit=asm
else
  cargo rustc -p microfield --lib --release \
    --target aarch64-unknown-linux-gnu \
    --no-default-features --features portable,builtin-fields -- --emit=asm
fi

assembly="$(find target/aarch64-unknown-linux-gnu/release/deps -maxdepth 1 \
  -type f -name 'microfield-*.s' -printf '%T@ %p\n' \
  | sort -nr | head -n 1 | cut -d' ' -f2-)"
if [[ -z "$assembly" ]]; then
  echo "no se encontró el ensamblado AArch64 de microfield" >&2
  exit 1
fi

grep -Eq '^[[:space:]]*pmull[[:space:]]' "$assembly"
grep -Eq 'aarch64_pmull.*wide_product_128_karatsuba' "$assembly"
grep -Eq 'aarch64_pmull.*wide_product_256_karatsuba' "$assembly"
grep -Eq 'aarch64_pmull.*wide_square_256' "$assembly"

audit_dir="$(mktemp -d)"
trap 'rm -rf "$audit_dir"' EXIT
awk '
  /aarch64_pmull/ { in_backend = 1 }
  in_backend { print }
  in_backend && /^\.Lfunc_end/ { in_backend = 0 }
' "$assembly" > "$audit_dir/backend.s"
test -s "$audit_dir/backend.s"

if grep -Eq '\bblr\b|\bbr[[:space:]]+x[0-9]+\b|__rust_alloc|alloc::|\b(udiv|sdiv)\b' \
  "$audit_dir/backend.s"; then
  echo "el backend PMULL contiene dispatch indirecto, división o una referencia al asignador" >&2
  exit 1
fi

instruction_count="$(grep -Ec '^[[:space:]]+[a-z][a-z0-9.]*[[:space:]]' "$audit_dir/backend.s" || true)"
if [[ "$instruction_count" -gt 12000 ]]; then
  echo "el backend PMULL supera el presupuesto estructural de 12000 instrucciones" >&2
  exit 1
fi

echo "auditoría PMULL correcta: $instruction_count instrucciones; sin dispatch, división ni asignador"
