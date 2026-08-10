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

Its C1/C2 design is specified in
`docs/microfield/pre-rc-comprehensive-campaign-plan.md`. The substantially
broader normative C3 plan, including the full coverage ledger, factorial
rules, multi-host replication and systems scenarios, is
`docs/microfield/c3-extensive-campaign-plan.md`.
The machine-readable capability ownership and volume targets are frozen in
`c3-coverage-ledger-v1.json`; `tests/c3_campaign_plan.rs` fails if the admitted
RC surface and the C3 ledger drift apart.

C3-P0 adds the deterministic factor expander and the first generated F1/F2
shards:

```text
cargo run -p microfield-validation-lab --locked -- c3-expand \
  --manifest validation/benchmarks/c3-p0-factor-plan-v1.json \
  --out-dir validation/benchmarks/manifests/c3-p0
```

The checked-in output contains 498 publication cells and 30 preflight cells
covering every registered static-field primitive. Regeneration is byte-tested.
The implementation inventory is `c3-operation-inventory-v1.json`; missing
workloads remain explicit until their suite is implemented. Initial Smoke
results and cost bounds are documented in
`docs/microfield/c3-p0-implementation-and-preflight-report.md`.

F3–S3 extend the deterministic inventory through batch/packed algorithms,
runtime fields and tooling, base and multi-evaluation signatures, tracked
state, snapshots, deltas and journals:

```text
cargo run -p microfield-validation-lab --locked -- c3-expand \
  --manifest validation/benchmarks/c3-f3-s3-factor-plan-v1.json \
  --out-dir validation/benchmarks/manifests/c3-f3-s3
```

These seven shards add 2,112 publication cells and 56 preflight cells. Together
with F1/F2 the generated C3-Scaling inventory is 2,610 cells. All 56 F3–S3
Smoke cells completed precisely under the exploratory threshold with stable
checksums; this is harness evidence, not a publication claim. See
`docs/microfield/c3-f3-s3-implementation-and-preflight-report.md`.

T1/R1/D1 add 374 cells for file trees, bounded reconciliation and the
in-memory database:

```text
cargo run -p microfield-validation-lab --locked -- c3-expand \
  --manifest validation/benchmarks/c3-t1-r1-d1-factor-plan-v1.json \
  --out-dir validation/benchmarks/manifests/c3-t1-r1-d1
```

The 16 unique preflight variants completed with stable checksums and were all
precise after the focused T1 restore calibration. See
`docs/microfield/c3-t1-r1-d1-implementation-and-preflight-report.md`.

G1/G2 add 126 cells for the graph pipeline, incremental batches, exact budget
and structural-family matrices, plus persistent/incremental DAG state:

```text
cargo run -p microfield-validation-lab --locked -- c3-expand \
  --manifest validation/benchmarks/c3-g1-g2-factor-plan-v1.json \
  --out-dir validation/benchmarks/manifests/c3-g1-g2
```

All 17 preflight cells were precise with stable checksums. The generated C3
inventory now contains 3,110 publication cells. See
`docs/microfield/c3-g1-g2-implementation-and-preflight-report.md`.

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

The post-C2 exact-graph telemetry campaign makes bounded-search semantics
first-class data instead of encoding them only in checksums:

```text
cargo run --release -p microfield-validation-lab --locked -- \
  publication-campaign \
  --manifest validation/benchmarks/manifests/graph-exact-telemetry-pilot-v4.json \
  --run-dir validation/benchmarks/runs/pre-rc-graph-exact-telemetry-pilot-v4
```

Its six cells and 30 independent workers were all precise. Five returned
`exact`; a deliberately bounded cycle returned `inconclusive` with
`search-nodes` as the exhausted limit. JSON, CSV and Markdown retain the node
budget and exact search counters. The run is `Informative`, with
`claims_allowed=false`.

This campaign introduces the backward-compatible
`microfield-publication-worker-v2` and `aggregate-v2` envelopes. The analyser
continues to accept homogeneous historical `v1` worker sets; only `v2` exact
graph workers are required to carry the structured telemetry.

Two further post-C2 pilots close the selected signature/graph axes:

- `comprehensive-fragmentation-pilot-v3.json` / corresponding run: five
  alternating K=4 fragmentation points, 100 workers, 10/10 precise cells and
  estimable slopes;
- `graph-topology-density-pilot-v1.json` / corresponding run: mesh, regular
  degree 8/32 and paired topology edits, 50 workers and 10/10 precise cells.

The pinned external Graph Atlas, MUTAG, Email-EU and Diseasome corpus remains
an opt-in correctness gate; public timing waits for the controlled C3 host.

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

`publication-controlled-v1.json` is now a historical seed/regression manifest,
not the complete C3 campaign. The extensive C3 plan requires generated shards
and additional workloads before controlled execution.

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
