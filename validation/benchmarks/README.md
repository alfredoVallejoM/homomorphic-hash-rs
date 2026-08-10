# Pre-RC publication benchmarks

This directory contains the versioned protocol, manifests and JSON schemas for
the statistically rigorous campaign. It is separate from the fast RC.8
capacity/regression gate.

The measured homomorphic signatures are non-cryptographic algebraic
fingerprints. Performance results do not add security properties.

The integral C1 campaign covers every maintained family and gives extra weight
to composition laws and graph scaling:

```text
cargo run --release -p microfield-validation-lab --locked -- \
  publication-campaign \
  --manifest validation/benchmarks/manifests/comprehensive-smoke-v1.json \
  --run-dir validation/benchmarks/runs/pre-rc-comprehensive-smoke-v1
```

Its design and C2/C3 expansion are specified in
`docs/microfield/pre-rc-comprehensive-campaign-plan.md`.

C2 is also versioned and remains informative:

```text
cargo run --release -p microfield-validation-lab --locked -- \
  publication-campaign \
  --manifest validation/benchmarks/manifests/comprehensive-pilot-v1.json \
  --run-dir validation/benchmarks/runs/pre-rc-comprehensive-pilot-v1

cargo run --release -p microfield-validation-lab --locked -- \
  publication-campaign \
  --manifest validation/benchmarks/manifests/comprehensive-fragmentation-pilot-v1.json \
  --run-dir validation/benchmarks/runs/pre-rc-comprehensive-fragmentation-pilot-v1
```

Together these runs contain 47 cells, 470 independent processes and 7,050
observations; 44/47 cells met the 10% pilot precision target. PostgreSQL adds
33 exactly verified samples over one million rows. Interpretation and the C3
blockers are in `docs/microfield/pre-rc-comprehensive-pilot-results.md`.

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

The bulk-scaling extension isolates coalesced summary-tree replacements and
partitioned database transactions across size, edit density and scattered or
clustered distributions:

- `pilot-bulk-scaling-v1.json` and
  `runs/pre-rc-bulk-scaling-pilot-v1`: baseline, 57 cells;
- `pilot-bulk-scaling-v2.json` and
  `runs/pre-rc-bulk-scaling-pilot-v2`: post-clone optimization, 57 cells;
- `pilot-bulk-density-frontier-v1.json` and
  `runs/pre-rc-bulk-density-frontier-pilot-v1`: 75/100% frontier, 18 cells;
- `pilot-db-streaming-v1.json` and
  `runs/pre-rc-database-streaming-pilot-v1`: post-streaming confirmation, six
  cells.

Together they retain 6,210 observations from 690 independent worker processes;
131/138 cells met the configured precision target. These runs are also
`Informative` with `claims_allowed=false`. Their interpretation, including why
256 edits are not an absolute limit, lives in
`docs/microfield/pre-rc-bulk-scaling-results.md`.

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
