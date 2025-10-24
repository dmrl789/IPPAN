//! DAG-Fair Emission Integration Tests
//!
//! Simulates multi-round consensus with deterministic emission,
//! halving schedule, and fair reward distribution among validators.
//! Ensures the emission model respects hard supply caps and fairness.

use ippan_economics::*;
use ippan_treasury::RewardSink;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::collections::HashMap;

/// Create a validator ID
fn validator_id(id: u8) -> ValidatorId {
    ValidatorId(format!("@validator{}.ipn", id))
}

/// Generate a random participation set
fn generate_participants(round: u64, validators: &[ValidatorId], rng: &mut StdRng) -> ParticipationSet {
    let mut set = ParticipationSet::default();
    for vid in validators {
        let role = if rng.gen_bool(0.25) { Role::Proposer } else { Role::Verifier };
        let blocks = rng.gen_range(1..=5);
        set.insert(vid.clone(), Participation { role, blocks });
    }
    set
}

/// Simulate 1000 rounds of DAG-Fair emission and distribution
#[test]
fn test_dag_fair_emission_integration() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut params = EconomicsParams::default();
    params.halving_interval_rounds = 250;
    params.base_emission_micro = 10_000;
    params.hard_cap_micro = 21_000_000 * MICRO_PER_IPN;

    let validators: Vec<ValidatorId> = (0..10).map(|i| validator_id(i)).collect();
    let mut reward_sink = RewardSink::new();

    let mut total_issued: MicroIPN = 0;
    let mut total_burned: MicroIPN = 0;

    for round in 0..1000 {
        let emission_micro = match emission_for_round_capped(round, total_issued, &params) {
            Some(e) => e,
            None => break,
        };
        let fees_micro = rng.gen_range(0..=10);
        let parts = generate_participants(round, &validators, &mut rng);

        let (payouts, emission_paid, _fees_paid) =
            distribute_round(emission_micro, fees_micro, &parts, &params).unwrap();

        reward_sink.credit_round_payouts(round, &payouts).unwrap();
        total_issued = total_issued.saturating_add(emission_paid);

        if round % 250 == 0 && round > 0 {
            let expected = sum_emission_over_rounds(round - 249, round, |r| emission_for_round(r, &params));
            let actual = total_issued.min(expected);
            total_burned = total_burned.saturating_add(epoch_auto_burn(expected, actual));
        }
    }

    // --- Assertions ---
    let stats = reward_sink.get_statistics();
    assert!(stats.total_rounds > 0);
    assert!(stats.total_validators > 0);
    assert!(total_issued <= params.hard_cap_micro);
    assert!(emission_for_round(1, &params) > emission_for_round(500, &params));

    println!(
        "✅ DAG-Fair integration test completed:
         Rounds: {}
         Validators: {}
         Issued: {} μIPN (~{:.3} IPN)
         Burned: {} μIPN
         Avg per round: {} μIPN",
        stats.total_rounds,
        stats.total_validators,
        total_issued,
        total_issued as f64 / MICRO_PER_IPN as f64,
        total_burned,
        stats.average_per_round
    );
}

/// Verify halving schedule
#[test]
fn test_halving_schedule_behavior() {
    let mut params = EconomicsParams::default();
    params.base_emission_micro = 1_000_000;
    params.halving_interval_rounds = 100;

    let e0 = emission_for_round(0, &params);
    let e100 = emission_for_round(100, &params);
    let e200 = emission_for_round(200, &params);
    let e300 = emission_for_round(300, &params);

    assert_eq!(e100, e0 / 2);
    assert_eq!(e200, e0 / 4);
    assert_eq!(e300, e0 / 8);

    println!(
        "✅ Halving verified: r0={} r100={} r200={} r300={}",
        e0, e100, e200, e300
    );
}

/// Verify emission stops when supply cap is reached
#[test]
fn test_supply_cap_enforcement() {
    let mut params = EconomicsParams::default();
    params.base_emission_micro = 10_000;
    params.hard_cap_micro = 50_000;
    params.halving_interval_rounds = 100;

    let mut total_issued = 0;
    for round in 0..10 {
        match emission_for_round_capped(round, total_issued, &params) {
            Some(emit) => total_issued += emit,
            None => break,
        }
    }

    assert!(total_issued <= params.hard_cap_micro);
    println!("✅ Supply cap enforcement: total issued = {} μIPN", total_issued);
}

/// Verify fairness between proposer and verifiers
#[test]
fn test_fair_distribution_ratios() {
    let mut params = EconomicsParams::default();
    params.base_emission_micro = 100_000;
    params.proposer_bps = 2000; // 20%
    params.verifier_bps = 8000; // 80%

    let mut parts = ParticipationSet::default();
    parts.insert(ValidatorId("@alice.ipn".into()), Participation { role: Role::Proposer, blocks: 5 });
    parts.insert(ValidatorId("@bob.ipn".into()), Participation { role: Role::Verifier, blocks: 5 });
    parts.insert(ValidatorId("@carol.ipn".into()), Participation { role: Role::Verifier, blocks: 5 });

    let (payouts, emission, _) = distribute_round(100_000, 0, &parts, &params).unwrap();
    let total_paid: MicroIPN = payouts.iter().map(|(_, v)| *v).sum();

    assert_eq!(total_paid, emission);
    let alice = payouts.iter().find(|(v, _)| v.0 == "@alice.ipn").unwrap().1;
    let bob = payouts.iter().find(|(v, _)| v.0 == "@bob.ipn").unwrap().1;

    println!(
        "✅ Fairness check:
         proposer={} μIPN, verifier={} μIPN, total={}",
        alice, bob, total_paid
    );

    assert!(bob > alice);
}

/// Verify zero emission once cap is fully reached
#[test]
fn test_zero_emission_post_cap() {
    let mut params = EconomicsParams::default();
    params.base_emission_micro = 5000;
    params.hard_cap_micro = 10_000;

    assert_eq!(emission_for_round_capped(1, 0, &params), Some(5000));
    assert_eq!(emission_for_round_capped(2, 5000, &params), Some(5000));
    assert_eq!(emission_for_round_capped(3, 10_000, &params), None);

    println!("✅ Zero emission verified post-cap reached");
}
