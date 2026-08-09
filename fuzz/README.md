# RC.7 fuzzing

These targets exercise only bounded, externally reachable parsing and
normalization paths. They do not interpret algebraic homomorphic signatures as
cryptographic authenticators.

Run a deterministic smoke campaign with nightly Rust and `cargo-fuzz`:

```text
cargo install cargo-fuzz --locked --version 0.13.2
cargo run --manifest-path fuzz/Cargo.toml --bin generate_seed_corpus --locked
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run microfield_manifests -- -runs=5000 -max_len=65536
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run structural_wires -- -runs=5000 -max_len=4096
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run graph_wires -- -runs=5000 -max_len=4096
```

Leak detection is disabled because these campaigns run under a supervised
process environment where LeakSanitizer cannot attach reliably; AddressSanitizer
memory safety checks remain enabled.

The checked-in corpus is the reproducible baseline. Any minimized crashing or
divergent input must be copied from `artifacts/<target>/` into the matching
`corpus/<target>/` directory and accompanied by a regression test and issue
classification before RC.7 can close.
