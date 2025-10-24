//! End-to-end simulation of DAG-Fair Emission across 1 000 rounds
//!
//! This test validates deterministic supply growth, halving,
//! and proportional fairness among validators with mixed roles.

use ippan_economics::*;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::collections::HashMap;

#[test]
fn simulate_emission_across_many_rounds() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut params = EconomicsParams::default();

    // Reduce halving interval for test speed (every 250 rounds)
    params.halving_interval_rounds = 250;

    // Simulated blockchain state
    let mut total_issued: MicroIPN = 0;
    let mut total_burned: MicroIPN = 0;

    // Synthetic ledger
    let mut balances: HashMap<ValidatorId, MicroIPN> = HashMap::new();

    // Validators
    let validators = vec![
        ValidatorId("alice.ipn".into()),
        ValidatorId("bob.ipn".into()),
        ValidatorId("carol.ipn".into()),
        ValidatorId("dave.ipn".into()),
    ];

    // Simulate 1 000 rounds
    let rounds: u64 = 1_000;
    for round in 0..rounds {
        let emission_micro = emission_for_round_capped(round, total_issued, &params)
            .expect("hard cap not exceeded");

        // Random 0–5 μIPN of fees (small relative to emission)
        let fees_micro = rng.gen_range(0..=5);

        // Participation: each validator 1–5 blocks, random role
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

        // Update total supply and balances
        total_issued = total_issued.saturating_add(emission_paid);
        for (vid, amt) in payouts {
            *balances.entry(vid).or_default() += amt;
        }

        // Sanity: emission ≤ remaining cap
        assert!(total_issued <= params.hard_cap_micro);

        // Occasionally simulate epoch reconciliation every 250 rounds
        if round > 0 && round % 250 == 0 {
            let expected = sum_emission_over_rounds(
                round - 249,
                round,
                |r| emission_for_round(r, &params),
            );
            let actual = total_issued.min(expected);
            let burn = epoch_auto_burn(expected, actual);
            total_burned = total_burned.saturating_add(burn);
        }
    }

    // === Post-conditions ===

    // Total minted ≤ 21 M IPN
    assert!(total_issued <= params.hard_cap_micro);

    // At least one halving occurred
    assert!(emission_for_round(1, &params) > emission_for_round(500, &params));

    // All validators earned something
    for vid in &validators {
        let bal = balances.get(vid).copied().unwrap_or(0);
        assert!(bal > 0, "validator {:?} earned zero", vid);
    }

    // Fairness check: total ratio spread < 2× (no dominance)
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
