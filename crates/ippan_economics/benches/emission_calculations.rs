//! Benchmarks for emission calculation performance

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ippan_economics::prelude::*;
use rust_decimal::Decimal;
use std::collections::HashMap;

fn bench_round_reward_calculation(c: &mut Criterion) {
    let emission_engine = EmissionEngine::new();
    
    c.bench_function("calculate_round_reward", |b| {
        b.iter(|| {
            emission_engine.calculate_round_reward(black_box(1000))
        })
    });
}

fn bench_emission_curve_generation(c: &mut Criterion) {
    let emission_engine = EmissionEngine::new();
    
    c.bench_function("generate_emission_curve", |b| {
        b.iter(|| {
            emission_engine.generate_emission_curve(black_box(1), black_box(1000), black_box(10))
        })
    });
}

fn bench_reward_distribution(c: &mut Criterion) {
    let params = EmissionParams::default();
    let round_rewards = RoundRewards::new(params);
    
    let participations = vec![
        ValidatorParticipation {
            validator_id: "validator1".to_string(),
            role: ValidatorRole::Proposer,
            blocks_contributed: 15,
            uptime_score: Decimal::new(95, 2),
        },
        ValidatorParticipation {
            validator_id: "validator2".to_string(),
            role: ValidatorRole::Verifier,
            blocks_contributed: 12,
            uptime_score: Decimal::new(98, 2),
        },
        ValidatorParticipation {
            validator_id: "validator3".to_string(),
            role: ValidatorRole::Verifier,
            blocks_contributed: 8,
            uptime_score: Decimal::new(92, 2),
        },
    ];
    
    c.bench_function("distribute_round_rewards", |b| {
        b.iter(|| {
            round_rewards.distribute_round_rewards(
                black_box(1000),
                black_box(10_000),
                black_box(participations.clone()),
                black_box(1_000),
            )
        })
    });
}

fn bench_supply_tracking(c: &mut Criterion) {
    let mut supply_tracker = SupplyTracker::new(2_100_000_000_000);
    
    c.bench_function("record_emission", |b| {
        b.iter(|| {
            supply_tracker.record_emission(black_box(1000), black_box(10_000))
        })
    });
}

fn bench_governance_voting(c: &mut Criterion) {
    let mut governance = GovernanceParams::new(EmissionParams::default());
    governance.set_validator_power("validator1".to_string(), 100);
    governance.set_validator_power("validator2".to_string(), 80);
    governance.set_validator_power("validator3".to_string(), 60);
    
    let proposal_id = governance.create_proposal(
        "validator1".to_string(),
        EmissionParams::default(),
        100,
        "Test proposal".to_string(),
        1000,
    ).unwrap();
    
    c.bench_function("vote_on_proposal", |b| {
        b.iter(|| {
            governance.vote_on_proposal(
                black_box(proposal_id),
                black_box("validator1".to_string()),
                black_box(Vote::Approve),
                black_box(1001),
            )
        })
    });
}

fn bench_supply_audit(c: &mut Criterion) {
    let mut supply_tracker = SupplyTracker::new(2_100_000_000_000);
    
    // Pre-populate with some data
    for round in 1..=1000 {
        supply_tracker.record_emission(round, 10_000).unwrap();
        if round % 100 == 0 {
            supply_tracker.record_burn(round, 1_000).unwrap();
        }
    }
    
    c.bench_function("audit_supply", |b| {
        b.iter(|| {
            supply_tracker.audit_supply()
        })
    });
}

fn bench_cumulative_supply_calculation(c: &mut Criterion) {
    let emission_engine = EmissionEngine::new();
    
    c.bench_function("calculate_cumulative_supply", |b| {
        b.iter(|| {
            emission_engine.calculate_cumulative_supply(black_box(1000))
        })
    });
}

fn bench_emission_parameters_validation(c: &mut Criterion) {
    let governance = GovernanceParams::new(EmissionParams::default());
    
    let mut params = EmissionParams::default();
    params.initial_round_reward = 15_000;
    
    c.bench_function("validate_emission_params", |b| {
        b.iter(|| {
            // This would be an internal method, so we'll simulate it
            let _ = params.initial_round_reward > 0;
            let _ = params.halving_interval > 0;
            let _ = params.total_supply_cap > 0;
            let _ = params.fee_cap_fraction >= Decimal::ZERO;
            let _ = params.fee_cap_fraction <= Decimal::ONE;
        })
    });
}

criterion_group!(
    benches,
    bench_round_reward_calculation,
    bench_emission_curve_generation,
    bench_reward_distribution,
    bench_supply_tracking,
    bench_governance_voting,
    bench_supply_audit,
    bench_cumulative_supply_calculation,
    bench_emission_parameters_validation
);

criterion_main!(benches);