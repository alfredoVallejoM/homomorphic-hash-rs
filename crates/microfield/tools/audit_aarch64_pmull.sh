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

if grep -Eq '\bblr\b|\bbr[[:space:]]+x[0-9]+\b|__rust_alloc|alloc::' "$assembly"; then
  echo "el backend PMULL contiene dispatch indirecto o una referencia al asignador" >&2
  exit 1
fi

echo "auditoría PMULL correcta: instrucciones y especializaciones presentes; sin dispatch indirecto ni asignador"
