use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use ippan_ai_core::{
    compute_scores, DecisionNode, DeterministicGBDT, Fixed, GBDTTree, ValidatorFeatures,
};

const VALIDATOR_COUNT: usize = 256;
const ROUNDS_PER_SAMPLE: usize = 8;

fn deterministic_model() -> DeterministicGBDT {
    DeterministicGBDT {
        trees: vec![
            GBDTTree {
                nodes: vec![
                    DecisionNode {
                        feature: 0,
                        threshold: Fixed::ZERO,
                        left: Some(1),
                        right: Some(2),
                        value: None,
                    },
                    DecisionNode {
                        feature: 1,
                        threshold: Fixed::from_micro(750),
                        left: None,
                        right: None,
                        value: Some(Fixed::from_ratio(3, 10)),
                    },
                    DecisionNode {
                        feature: 2,
                        threshold: Fixed::from_micro(950_000),
                        left: None,
                        right: None,
                        value: Some(Fixed::from_ratio(-1, 20)),
                    },
                ],
            },
            GBDTTree {
                nodes: vec![
                    DecisionNode {
                        feature: 3,
                        threshold: Fixed::from_micro(500_000),
                        left: Some(1),
                        right: Some(2),
                        value: None,
                    },
                    DecisionNode {
                        feature: 0,
                        threshold: Fixed::ZERO,
                        left: None,
                        right: None,
                        value: Some(Fixed::from_ratio(1, 5)),
                    },
                    DecisionNode {
                        feature: 3,
                        threshold: Fixed::from_micro(750_000),
                        left: None,
                        right: None,
                        value: Some(Fixed::from_ratio(-1, 10)),
                    },
                ],
            },
        ],
        learning_rate: Fixed::from_ratio(1, 10),
    }
}

fn generate_features(count: usize) -> Vec<ValidatorFeatures> {
    (0..count)
        .map(|idx| {
            let delta = (idx as i64 - 128) * 10;
            let latency = Fixed::from_micro(25_000 + (idx as i64 * 137 % 25_000));
            let uptime = Fixed::from_micro(950_000 + (idx as i64 % 50) * 500);
            let entropy = Fixed::from_micro(400_000 + (idx as i64 * 123 % 200_000));

            ValidatorFeatures {
                node_id: format!("validator-{idx:04}"),
                delta_time_us: delta,
                latency_ms: latency,
                uptime_pct: uptime,
                peer_entropy: entropy,
                cpu_usage: Some(Fixed::from_micro(450_000 + (idx as i64 % 100) * 1_000)),
                memory_usage: None,
                network_reliability: Some(Fixed::from_micro(800_000)),
            }
        })
        .collect()
}

fn benchmark_dgbdt_scoring(c: &mut Criterion) {
    let model = deterministic_model();
    let features = generate_features(VALIDATOR_COUNT);

    let mut group = c.benchmark_group("dgbdt_scoring");
    group.throughput(Throughput::Elements(
        (VALIDATOR_COUNT * ROUNDS_PER_SAMPLE) as u64,
    ));
    group.bench_function("score_256_validators_over_8_rounds", |b| {
        b.iter(|| {
            for round in 0..ROUNDS_PER_SAMPLE {
                let hash = format!("round-hash-{round:04}");
                let scores = compute_scores(&model, &features, &hash);
                criterion::black_box(scores);
            }
        });
    });
    group.finish();
}

criterion_group!(benches, benchmark_dgbdt_scoring);
criterion_main!(benches);
