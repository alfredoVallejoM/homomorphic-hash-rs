# Pre-RC publication benchmarks

This directory contains the versioned protocol, manifests and JSON schemas for
the statistically rigorous campaign. It is separate from the fast RC.8
capacity/regression gate.

The measured homomorphic signatures are non-cryptographic algebraic
fingerprints. Performance results do not add security properties.

```text
cargo run --release -p microfield-validation-lab --locked -- \
  publication-campaign \
  --manifest validation/benchmarks/manifests/smoke-v1.json \
  --run-dir /tmp/microfield-publication-smoke

cargo run --release -p microfield-validation-lab --locked -- \
  publication-analyse \
  --manifest validation/benchmarks/manifests/smoke-v1.json \
  --run-dir /tmp/microfield-publication-smoke
```

`publication-campaign` refuses to overwrite a non-empty run. `smoke` only
checks the harness and always has `claims_allowed=false`. Host-specific timing
is never a cross-machine golden test.

B.3 delivers `pilot-scaling-v1.json`, `publication-informative-v1.json` and
the ready-to-run `publication-controlled-v1.json`. The versioned results live
below `runs/`:

- `pre-rc-b3-pilot-scaling-v1`: 6,600 observations, 42/44 precise cells;
- `pre-rc-b3-publication-informative-v1`: 66,500 observations, 44/44 precise
  cells, 22 paired comparisons and eight scaling curves.

Both runs are `Informative` and therefore have `claims_allowed=false`. Their
timings guide internal decisions but are not public performance claims. See
`docs/microfield/pre-rc-b3-benchmark-results.md` for interpretation and limits.

A run can only be classified `Controlled` when it uses a release binary from a
clean tree, the manifest declares a dedicated host, frequency metadata is
visible and the operator explicitly attests dedicated execution and fixed
affinity. A new campaign must use a new directory; existing evidence is never
overwritten.

Only after those conditions are true, replace `<isolated-cpus>` and run:

```text
MICROFIELD_BENCH_DEDICATED=1 MICROFIELD_BENCH_AFFINITY_FIXED=1 \
taskset -c <isolated-cpus> \
cargo run --release -p microfield-validation-lab --locked -- \
  publication-campaign \
  --manifest validation/benchmarks/manifests/publication-controlled-v1.json \
  --run-dir validation/benchmarks/runs/<new-controlled-campaign-id>
```

The environment variables are operator attestations, not switches that make a
shared host controlled. Never set them unless the statements are true.
