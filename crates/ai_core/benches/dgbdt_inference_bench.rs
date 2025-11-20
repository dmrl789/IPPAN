use std::path::PathBuf;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ippan_ai_core::{load_model_from_path, DeterministicModel};

fn load_reputation_model() -> DeterministicModel {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let model_path = manifest_dir.join("../../models/reputation_v1.json");
    load_model_from_path(&model_path).expect("load reputation model")
}

fn bench_reputation_inference(c: &mut Criterion) {
    let model = load_reputation_model();
    let features = vec![black_box(5_000_i64)];

    c.bench_function("dgbdt_reputation_score", |b| {
        b.iter(|| {
            let score = model.score(black_box(&features));
            black_box(score);
        });
    });
}

criterion_group!(ai_benches, bench_reputation_inference);
criterion_main!(ai_benches);
