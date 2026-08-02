#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$repo_root"

inventory="crates/microfield/unsafe/unsafe-inventory-v2.sha256"
sha256sum --check --strict "$inventory"

mapfile -t unsafe_files < <(
  find crates/microfield/src -type f -name '*.rs' -exec \
    grep -El 'unsafe fn|unsafe \{|unsafe impl|unsafe trait|unsafe extern|#\[unsafe\(' {} + | sort
)
expected=(
  crates/microfield/src/backend/aarch64_pmull.rs
  crates/microfield/src/backend/x86_pclmul.rs
  crates/microfield/src/backend/x86_prime.rs
  crates/microfield/src/backend/x86_vpclmul.rs
  crates/microfield/src/engine/packed/storage.rs
)

if [[ "${unsafe_files[*]}" != "${expected[*]}" ]]; then
  echo "el conjunto de fronteras unsafe difiere del inventario revisado" >&2
  printf 'observado: %s\n' "${unsafe_files[@]}" >&2
  exit 1
fi

allow_count="$(grep -R -n -E --include='*.rs' '#\[allow\(unsafe_code\)\]' crates/microfield/src | wc -l)"
if [[ "$allow_count" -ne 5 ]]; then
  echo "se esperaban exactamente cinco excepciones allow(unsafe_code), hay $allow_count" >&2
  exit 1
fi

for source in "${expected[@]}"; do
  if ! grep -q 'SAFETY:' "$source"; then
    echo "la frontera $source carece de invariantes SAFETY documentadas" >&2
    exit 1
  fi
done

echo "inventario unsafe v2 correcto: cinco fronteras, hashes revisados e invariantes SAFETY"
