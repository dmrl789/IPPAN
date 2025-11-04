use ippan_consensus::emission_tracker::ValidatorContribution;
use ippan_consensus::EmissionTracker;
use ippan_economics::{
    projected_supply, rounds_until_supply_cap, scheduled_round_reward, EmissionEngine,
    EmissionParams, RoundRewards, ValidatorId, ValidatorParticipation, ValidatorRole,
};
use rust_decimal::Decimal;

fn make_participation(
    id: &str,
    role: ValidatorRole,
    blocks: u32,
    uptime_percent: u32,
) -> ValidatorParticipation {
    ValidatorParticipation {
        validator_id: ValidatorId::new(id),
        role,
        blocks_contributed: blocks,
        uptime_score: Decimal::new(uptime_percent as i64, 2),
    }
}

fn make_tracker_contribution(
    id_byte: u8,
    blocks_proposed: u32,
    blocks_verified: u32,
    reputation_score: f64,
) -> ValidatorContribution {
    ValidatorContribution {
        validator_id: [id_byte; 32],
        blocks_proposed,
        blocks_verified,
        reputation_score,
    }
}

#[test]
fn test_scheduled_rewards_follow_halving() {
    let params = EmissionParams::default();

    assert_eq!(scheduled_round_reward(1, &params), params.initial_round_reward_micro);

    let halving_round = params.halving_interval_rounds + 1;
    let reward_before = scheduled_round_reward(params.halving_interval_rounds, &params);
    let reward_after = scheduled_round_reward(halving_round, &params);

    assert_eq!(reward_before, params.initial_round_reward_micro);
    assert_eq!(reward_after, params.initial_round_reward_micro / 2);
}

#[test]
fn test_projected_supply_monotonic_and_capped() {
    let params = EmissionParams::default();
    let year_rounds = 315_360_000; // ~1 year at 10 rounds/sec

    let supply_year1 = projected_supply(year_rounds, &params);
    let supply_year2 = projected_supply(year_rounds * 2, &params);
    let supply_year10 = projected_supply(year_rounds * 10, &params);

    assert!(supply_year1 <= supply_year2);
    assert!(supply_year2 <= supply_year10);
    assert!(supply_year10 <= params.max_supply_micro as u128);

    let theoretical_year1 = params.initial_round_reward_micro as u128 * year_rounds as u128;
    let expected_year1 = theoretical_year1.min(params.max_supply_micro as u128);
    let tolerance = expected_year1 / 100; // 1% tolerance
    assert!(
        supply_year1 >= expected_year1.saturating_sub(tolerance)
            && supply_year1 <= expected_year1.saturating_add(tolerance),
        "Year 1 emission out of range: expected ~{}, got {}",
        expected_year1,
        supply_year1
    );

    assert_eq!(supply_year10, params.max_supply_micro as u128);
}

#[test]
fn test_rounds_until_supply_cap_behavior() {
    let default_params = EmissionParams::default();
    let default_rounds = rounds_until_supply_cap(&default_params)
        .expect("default parameters should eventually reach the cap");
    assert!(default_rounds > 0);
    assert_eq!(
        projected_supply(default_rounds, &default_params),
        default_params.max_supply_micro as u128
    );

    let custom_params = EmissionParams {
        initial_round_reward_micro: 100_000,
        halving_interval_rounds: 10_000,
        max_supply_micro: 1_500_000_000,
        ..EmissionParams::default()
    };

    let rounds = rounds_until_supply_cap(&custom_params);
    assert!(rounds.is_some());
    let projected_at_round = projected_supply(rounds.unwrap(), &custom_params);
    assert!(projected_at_round >= custom_params.max_supply_micro as u128);
}

#[test]
fn test_round_rewards_weighting_and_bonus() {
    let rewards = RoundRewards::new(EmissionParams::default());

    let participations = vec![
        make_participation("proposer", ValidatorRole::Proposer, 10, 100),
        make_participation("verifier", ValidatorRole::Verifier, 10, 100),
        make_participation("low-uptime", ValidatorRole::Verifier, 10, 70),
    ];

    let distribution = rewards
        .distribute_round_rewards(1, 10_000, participations, 1_000)
        .unwrap();

    let proposer = distribution
        .validator_rewards
        .get(&ValidatorId::new("proposer"))
        .unwrap()
        .total_reward;
    let verifier = distribution
        .validator_rewards
        .get(&ValidatorId::new("verifier"))
        .unwrap()
        .total_reward;
    let low = distribution
        .validator_rewards
        .get(&ValidatorId::new("low-uptime"))
        .unwrap()
        .total_reward;

    assert!(proposer > verifier);
    assert!(verifier > low);

    let ratio = proposer as f64 / verifier as f64;
    assert!(
        (ratio - 1.2).abs() < 0.15,
        "Proposer bonus ratio too far from 1.2: {}",
        ratio
    );
}

#[test]
fn test_round_rewards_equal_participation_fairness() {
    let rewards = RoundRewards::new(EmissionParams::default());

    let participations: Vec<_> = (0..4)
        .map(|i| make_participation(&format!("validator-{i}"), ValidatorRole::Verifier, 8, 100))
        .collect();

    let distribution = rewards
        .distribute_round_rewards(2, 10_000, participations, 2_000)
        .unwrap();

    let rewards: Vec<u64> = distribution
        .validator_rewards
        .values()
        .map(|r| r.total_reward)
        .collect();

    let avg: u64 = rewards.iter().sum::<u64>() / rewards.len() as u64;
    let tolerance = avg / 100; // 1%
    let max_diff = rewards
        .iter()
        .map(|r| (*r as i128 - avg as i128).abs() as u64)
        .max()
        .unwrap();

    assert!(
        max_diff <= tolerance.max(1),
        "Equal participants deviated beyond tolerance: max diff {}, tolerance {}",
        max_diff,
        tolerance
    );
}

#[test]
fn test_round_rewards_deterministic() {
    let rewards = RoundRewards::new(EmissionParams::default());
    let participations = vec![
        make_participation("alpha", ValidatorRole::Proposer, 6, 100),
        make_participation("beta", ValidatorRole::Verifier, 6, 95),
    ];

    let dist1 = rewards
        .distribute_round_rewards(5, 8_000, participations.clone(), 500)
        .unwrap();
    let dist2 = rewards
        .distribute_round_rewards(5, 8_000, participations, 500)
        .unwrap();

    assert_eq!(dist1.total_reward, dist2.total_reward);
    assert_eq!(dist1.validator_rewards.len(), dist2.validator_rewards.len());
    for (validator, reward1) in &dist1.validator_rewards {
        let reward2 = dist2
            .validator_rewards
            .get(validator)
            .expect("missing validator in second distribution");
        assert_eq!(reward1.total_reward, reward2.total_reward);
        assert_eq!(reward1.round_emission, reward2.round_emission);
        assert_eq!(reward1.transaction_fees, reward2.transaction_fees);
        assert_eq!(reward1.ai_commissions, reward2.ai_commissions);
        assert_eq!(reward1.network_dividend, reward2.network_dividend);
    }
}

#[test]
fn test_emission_tracker_processes_rounds_and_stays_consistent() {
    let params = EmissionParams::default();
    let mut tracker = EmissionTracker::new(params.clone(), 50);

    let contributions = vec![
        make_tracker_contribution(1, 5, 12, 10_000.0),
        make_tracker_contribution(2, 3, 8, 9_500.0),
    ];

    for round in 1..=100 {
        tracker
            .process_round(round, &contributions, 0, 0)
            .expect("round processing should succeed");
    }

    assert_eq!(tracker.last_round, 100);
    assert!(tracker.cumulative_supply > 0);
    assert!(tracker.verify_consistency().is_ok());
    assert!(!tracker.audit_history.is_empty());
}

#[test]
fn test_emission_tracker_handles_empty_rounds() {
    let params = EmissionParams::default();
    let mut tracker = EmissionTracker::new(params, 10);

    tracker.process_round(1, &[], 0, 0).unwrap();
    tracker.process_round(2, &[], 0, 0).unwrap();

    assert_eq!(tracker.empty_rounds, 2);
    assert!(tracker.verify_consistency().is_ok());
}

#[test]
fn test_emission_tracker_rejects_non_sequential_rounds() {
    let params = EmissionParams::default();
    let mut tracker = EmissionTracker::new(params, 100);
    let contributions = vec![make_tracker_contribution(3, 1, 1, 8_000.0)];

    tracker.process_round(1, &contributions, 100, 50).unwrap();
    let err = tracker.process_round(3, &contributions, 100, 50).unwrap_err();
    assert!(err.contains("Non-sequential"));
}

#[test]
fn test_emission_engine_enforces_supply_cap() {
    let params = EmissionParams {
        initial_round_reward_micro: 1_000,
        halving_interval_rounds: 2,
        max_supply_micro: 5_000,
        ..EmissionParams::default()
    };

    let mut engine = EmissionEngine::with_params(params.clone());
    let mut total: u128 = 0;

    for round in 1..=10 {
        total = total.saturating_add(engine.advance_round(round).unwrap() as u128);
    }

    assert!(total <= params.max_supply_micro as u128);
}
