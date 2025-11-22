use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ippan_ai_core::gbdt::{Node, Tree, SCALE};
use ippan_ai_core::DeterministicModel;

fn sample_model() -> DeterministicModel {
    // Minimal deterministic model for benchmarking: one tree with a single split.
    // Values are scaled by `SCALE` (1e6) to match the deterministic GBDT format.
    let tree = Tree::new(
        vec![
            Node::internal(0, 0, 50 * SCALE, 1, 2),
            Node::leaf(1, 100 * SCALE),
            Node::leaf(2, 200 * SCALE),
        ],
        SCALE,
    );

    DeterministicModel::new(vec![tree], 0)
}

fn bench_reputation_inference(c: &mut Criterion) {
    let model = sample_model();
    let features = vec![black_box(40 * SCALE)];

    c.bench_function("dgbdt_reputation_score", |b| {
        b.iter(|| {
            let score = model.score(black_box(&features));
            black_box(score);
        });
    });
}

criterion_group!(ai_benches, bench_reputation_inference);
criterion_main!(ai_benches);
