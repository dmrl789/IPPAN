use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use ippan_time::{HashTimer, IppanTimeMicros};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::cmp::Ordering;

const SAMPLE_SIZE: usize = 256;

fn generate_hash_timers(count: usize) -> Vec<HashTimer> {
    let mut rng = StdRng::seed_from_u64(42_101_337);
    (0..count)
        .map(|idx| {
            let mut node_id = [0u8; 32];
            rng.fill(&mut node_id);
            let payload = format!("tx-payload-{idx:04}").into_bytes();
            let nonce = (idx as u64 + 1).to_be_bytes();
            HashTimer::derive(
                "hashtimer-bench",
                IppanTimeMicros(1_700_000_000 + idx as u64),
                b"ordering",
                &payload,
                &nonce,
                &node_id,
            )
        })
        .collect()
}

fn compare_timers(a: &HashTimer, b: &HashTimer) -> Ordering {
    a.timestamp_us
        .cmp(&b.timestamp_us)
        .then_with(|| a.digest().cmp(&b.digest()))
}

fn benchmark_hashtimer_ordering(c: &mut Criterion) {
    let timers = generate_hash_timers(SAMPLE_SIZE);

    let mut group = c.benchmark_group("hashtimer_ordering");
    group.throughput(Throughput::Elements(SAMPLE_SIZE as u64));

    group.bench_function("sort_batch_256", |b| {
        b.iter(|| {
            let mut batch = timers.clone();
            batch.sort_by(compare_timers);
            black_box(batch);
        });
    });

    group.bench_function("stable_priority_queue_256", |b| {
        b.iter(|| {
            let mut items: Vec<_> = timers
                .iter()
                .enumerate()
                .map(|(idx, timer)| (timer.clone(), idx as u64))
                .collect();
            items.sort_by(|(ta, _), (tb, _)| compare_timers(ta, tb));
            black_box(items);
        });
    });

    group.finish();
}

criterion_group!(benches, benchmark_hashtimer_ordering);
criterion_main!(benches);
