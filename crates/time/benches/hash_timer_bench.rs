use criterion::{criterion_group, criterion_main, Criterion};
use ippan_time::now_us;

fn bench_now_us(c: &mut Criterion) {
    c.bench_function("ippan_time_now_us", |b| {
        b.iter(|| {
            let _ = now_us();
        });
    });
}

criterion_group!(time_benches, bench_now_us);
criterion_main!(time_benches);
