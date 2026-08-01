#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$repo_root"

cargo bench -p microfield --bench portable_batch --no-run

binary="$(find target/release/deps -maxdepth 1 -type f -name 'portable_batch-*' -executable -printf '%T@ %p\n' \
  | sort -nr | head -n 1 | cut -d' ' -f2-)"
if [[ -z "$binary" ]]; then
  echo "no se encontró el ejecutable de auditoría portable_batch" >&2
  exit 1
fi

audit_dir="$(mktemp -d)"
trap 'rm -rf "$audit_dir"' EXIT
objdump -Cd "$binary" > "$audit_dir/full.asm"

awk '
  /^[[:xdigit:]]+ <.*microfield::backend::x86_pclmul::/ { in_backend = 1 }
  in_backend && /^$/ { in_backend = 0 }
  in_backend { print }
' "$audit_dir/full.asm" > "$audit_dir/backend.asm"

awk '
  /^[[:xdigit:]]+ <.*microfield::backend::x86_pclmul::builtins::(multiply|square)/ {
    in_kernel = 1
  }
  in_kernel && /^$/ { in_kernel = 0 }
  in_kernel { print }
' "$audit_dir/full.asm" > "$audit_dir/kernels.asm"

test -s "$audit_dir/backend.asm"
test -s "$audit_dir/kernels.asm"
grep -Eq '\bpclmul[a-z0-9]*\b' "$audit_dir/backend.asm"

if grep -Eq 'call[q]?[[:space:]].*\*|__rust_alloc|<.*alloc::' "$audit_dir/kernels.asm"; then
  echo "el kernel PCLMUL contiene dispatch interno o una referencia al asignador" >&2
  exit 1
fi

echo "auditoría PCLMUL correcta: instrucciones presentes; sin dispatch interno ni asignador"
