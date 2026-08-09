#!/usr/bin/env bash
set -euo pipefail

repository_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
inventory=$(mktemp /tmp/hhrs-package-inventory.XXXXXX)
trap 'rm -f "$inventory"' EXIT

cd "$repository_root"
cargo package -p homomorphic-hash-rs --list --allow-dirty > "$inventory"

grep -qx 'Cargo.toml' "$inventory"
grep -qx 'README.md' "$inventory"
grep -qx 'src/lib.rs' "$inventory"

if grep -qE '^(data/|fuzz/|target/|test-fixtures/)' "$inventory"; then
  echo 'RC package inventory contains datasets, fuzz state, build output or fixtures' >&2
  exit 1
fi

file_count=$(wc -l < "$inventory")
if (( file_count > 150 )); then
  echo "RC package inventory unexpectedly contains $file_count files" >&2
  exit 1
fi

cargo package -p microfield --allow-dirty --no-verify --locked --offline
cargo metadata --locked --offline --no-deps --format-version 1 > /dev/null

echo "RC package inventory: $file_count root files; microfield archive dry-run passed"
echo 'The root archive remains intentionally unstaged until microfield has a registry/staging source.'
