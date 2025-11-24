use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use ippan_consensus::{create_participation_set, RoundExecutor};
use ippan_economics::EmissionParams;
use ippan_treasury::InMemoryAccountLedger;
use ippan_types::ChainState;
use rand::{rngs::StdRng, Rng, SeedableRng};

const VALIDATOR_COUNT: usize = 64;

fn generate_validators(count: usize) -> Vec<(u64, [u8; 32], u64, i64)> {
    let mut rng = StdRng::seed_from_u64(9_812_345);
    (0..count)
        .map(|idx| {
            let mut id = [0u8; 32];
            rng.fill(&mut id);
            let stake = 1_000 + idx as u64 * 10;
            let blocks = 1 + (rng.gen::<u32>() % 5) as u64;
            let reputation = 10_000 + (rng.gen::<u32>() % 5_000) as i64;
            (stake, id, blocks, reputation)
        })
        .collect()
}

fn benchmark_round_processing(c: &mut Criterion) {
    let validators = generate_validators(VALIDATOR_COUNT);
    let proposer = validators[0].1;
    let participants_template = create_participation_set(&validators, proposer);
    let fees_micro: u128 = 250_000;

    let mut group = c.benchmark_group("consensus_round_processing");
    group.throughput(Throughput::Elements(VALIDATOR_COUNT as u64));
    group.bench_function("execute_round_64", |b| {
        b.iter(|| {
            let mut executor = RoundExecutor::new(
                EmissionParams::default(),
                Box::new(InMemoryAccountLedger::new()),
            );
            let mut chain_state = ChainState::new();
            let participants = participants_template.clone();
            let result = executor
                .execute_round(42, &mut chain_state, participants, fees_micro)
                .expect("round executes");
            criterion::black_box(result);
        });
    });
    group.finish();
}

criterion_group!(benches, benchmark_round_processing);
criterion_main!(benches);
