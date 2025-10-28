//! End-to-end simulation of DAG-Fair Emission across 1 000 rounds
//!
//! This test validates deterministic supply growth, halving,
//! and proportional fairness among validators with mixed roles.

use ippan_consensus::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;

#[test]
fn simulate_emission_across_many_rounds() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut params = EconomicsParams::default();

    // reduce halving interval for test speed (every 250 rounds)
    params.halving_interval_rounds = 250;

    // Simulated blockchain state
    let mut total_issued: MicroIPN = 0;
    let mut total_burned: MicroIPN = 0;

    // synthetic ledger
    let mut balances: HashMap<ValidatorId, MicroIPN> = HashMap::new();

    // Validators
    let validators = vec![
        ValidatorId("alice.ipn".into()),
        ValidatorId("bob.ipn".into()),
        ValidatorId("carol.ipn".into()),
        ValidatorId("dave.ipn".into()),
    ];

    // simulate 1000 rounds
    let rounds: u64 = 1_000;
    for round in 0..rounds {
        let emission_micro =
            emission_for_round_capped(round, total_issued, &params).expect("hard cap not exceeded");

        // random 0–5 μIPN of fees (small relative to emission)
        let fees_micro = rng.gen_range(0..=5);

        // participation: each validator 1–5 blocks, random role
        let mut parts = ParticipationSet::default();
        for vid in &validators {
            let role = if rng.gen_bool(0.25) {
                Role::Proposer
            } else {
                Role::Verifier
            };
            let blocks = rng.gen_range(1..=5);
            parts.insert(vid.clone(), Participation { role, blocks });
        }

        let (payouts, emission_paid, _fees_paid) =
            distribute_round(emission_micro, fees_micro, &parts, &params)
                .expect("distribution succeeds");

        // update total supply and balances
        total_issued = total_issued.saturating_add(emission_paid);
        for (vid, amt) in payouts {
            *balances.entry(vid).or_default() += amt;
        }

        // sanity: emission ≤ remaining cap
        assert!(total_issued <= params.hard_cap_micro);

        // occasionally simulate epoch reconciliation every 250 rounds
        if round > 0 && round % 250 == 0 {
            let expected =
                sum_emission_over_rounds(round - 249, round, |r| emission_for_round(r, &params));
            let actual = total_issued.min(expected);
            let burn = epoch_auto_burn(expected, actual);
            total_burned = total_burned.saturating_add(burn);
        }
    }

    // === Post-conditions ===

    // total minted ≤ 21 M IPN
    assert!(total_issued <= params.hard_cap_micro);

    // at least one halving occurred (compare round 1 to round 500)
    assert!(emission_for_round(1, &params) > emission_for_round(500, &params));

    // all validators earned something
    for vid in &validators {
        let bal = balances.get(vid).copied().unwrap_or(0);
        assert!(bal > 0, "validator {:?} earned zero", vid);
    }

    // fairness check: total ratio spread < 2× (no dominance)
    let min = balances.values().min().copied().unwrap_or(0);
    let max = balances.values().max().copied().unwrap_or(0);
    assert!(
        max <= min.saturating_mul(2),
        "reward imbalance too large: min={} max={}",
        min,
        max
    );

    println!(
        "✅ DAG-Fair simulation: issued={} μIPN (≈ {:.3} IPN), burned={} μIPN",
        total_issued,
        total_issued as f64 / MICRO_PER_IPN as f64,
        total_burned
    );
}

#[test]
fn test_emission_halving_schedule() {
    let params = EconomicsParams {
        r0: 10_000,
        halving_interval_rounds: 1000,
        ..Default::default()
    };

    // Test multiple halvings
    assert_eq!(emission_for_round(1, &params), 10_000);
    assert_eq!(emission_for_round(999, &params), 10_000);
    assert_eq!(emission_for_round(1000, &params), 5_000);
    assert_eq!(emission_for_round(1999, &params), 5_000);
    assert_eq!(emission_for_round(2000, &params), 2_500);
    assert_eq!(emission_for_round(3000, &params), 1_250);
}

#[test]
fn test_hard_cap_enforcement() {
    let mut params = EconomicsParams::default();
    params.hard_cap_micro = 1_000_000; // 0.01 IPN cap

    let mut total_issued = 0u128;

    // Issue until we hit the cap
    for round in 0..10_000 {
        match emission_for_round_capped(round, total_issued, &params) {
            Ok(emission) => {
                total_issued = total_issued.saturating_add(emission);
            }
            Err(_) => {
                // Hit the cap
                assert_eq!(total_issued, params.hard_cap_micro);
                return;
            }
        }
    }

    // Should never exceed cap
    assert!(total_issued <= params.hard_cap_micro);
}

#[test]
fn test_fair_distribution_proportionality() {
    let params = EconomicsParams::default();
    let emission = 10_000u128;
    let fees = 1_000u128;

    // Two validators: one with 10 blocks, one with 5 blocks
    let mut parts = ParticipationSet::default();
    parts.insert(
        ValidatorId("alice.ipn".into()),
        Participation {
            role: Role::Verifier,
            blocks: 10,
        },
    );
    parts.insert(
        ValidatorId("bob.ipn".into()),
        Participation {
            role: Role::Verifier,
            blocks: 5,
        },
    );

    let (payouts, _, _) =
        distribute_round(emission, fees, &parts, &params).expect("distribution succeeds");

    let alice_payout = payouts
        .get(&ValidatorId("alice.ipn".into()))
        .copied()
        .unwrap_or(0);
    let bob_payout = payouts
        .get(&ValidatorId("bob.ipn".into()))
        .copied()
        .unwrap_or(0);

    // Alice should get ~2× Bob's reward (10 blocks vs 5)
    let ratio = alice_payout as f64 / bob_payout as f64;
    assert!(
        (ratio - 2.0).abs() < 0.1,
        "expected ratio ~2.0, got {}",
        ratio
    );
}

#[test]
fn test_proposer_verifier_split() {
    let params = EconomicsParams {
        proposer_bps: 2000, // 20%
        verifier_bps: 8000, // 80%
        ..Default::default()
    };

    let emission = 10_000u128;
    let fees = 0u128;

    let mut parts = ParticipationSet::default();
    parts.insert(
        ValidatorId("proposer.ipn".into()),
        Participation {
            role: Role::Proposer,
            blocks: 1,
        },
    );
    parts.insert(
        ValidatorId("verifier.ipn".into()),
        Participation {
            role: Role::Verifier,
            blocks: 1,
        },
    );

    let (payouts, _, _) =
        distribute_round(emission, fees, &parts, &params).expect("distribution succeeds");

    let proposer_payout = payouts
        .get(&ValidatorId("proposer.ipn".into()))
        .copied()
        .unwrap_or(0);
    let verifier_payout = payouts
        .get(&ValidatorId("verifier.ipn".into()))
        .copied()
        .unwrap_or(0);

    // Proposer should get ~20%, verifier ~80%
    let proposer_pct = (proposer_payout as f64 / emission as f64) * 100.0;
    let verifier_pct = (verifier_payout as f64 / emission as f64) * 100.0;

    assert!(
        (proposer_pct - 20.0).abs() < 1.0,
        "expected proposer ~20%, got {:.1}%",
        proposer_pct
    );
    assert!(
        (verifier_pct - 80.0).abs() < 1.0,
        "expected verifier ~80%, got {:.1}%",
        verifier_pct
    );
}

#[test]
fn test_zero_emission_after_many_halvings() {
    let params = EconomicsParams {
        r0: 1,
        halving_interval_rounds: 1,
        ..Default::default()
    };

    // After 64 halvings, emission should be 0
    let round_after_64_halvings = 64;
    assert_eq!(emission_for_round(round_after_64_halvings, &params), 0);
}

#[test]
fn test_emission_sum_helper() {
    let params = EconomicsParams {
        r0: 1000,
        halving_interval_rounds: 10,
        ..Default::default()
    };

    let sum = sum_emission_over_rounds(1, 20, |r| emission_for_round(r, &params));

    // Rounds 1-9: 1000 each = 9,000 (halvings = 0)
    // Rounds 10-19: 500 each = 5,000 (halvings = 1)
    // Round 20: 250 (halvings = 2)
    // Total = 14,250
    assert_eq!(sum, 14_250);
}

#[test]
fn test_epoch_auto_burn() {
    let expected = 100_000u128;
    let actual = 99_950u128;

    let burned = epoch_auto_burn(expected, actual);
    assert_eq!(burned, 50);

    // No burn if actual >= expected
    let burned_zero = epoch_auto_burn(1000, 1000);
    assert_eq!(burned_zero, 0);
}
