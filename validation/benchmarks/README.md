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

Pilot and publication/scaling manifests are delivered by B.3. A run can only
be classified `Controlled` when it uses a release binary from a clean tree,
the manifest declares a dedicated host, frequency metadata is visible and the
operator explicitly attests dedicated execution and fixed affinity.
