use criterion::{criterion_group, criterion_main, Criterion};

fn bench_throughput(c: &mut Criterion) {
    // TODO: Implement Suite 1 - Raw GB/s throughput vs SHA-256
}

criterion_group!(benches, bench_throughput);
criterion_main!(benches);
