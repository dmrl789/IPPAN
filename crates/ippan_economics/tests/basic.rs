use ippan_economics::*;

#[test]
fn test_emission_halving() {
    let p = EconomicsParams::default();
    // Before first halving
    assert_eq!(emission_for_round(0, &p), 100);
    assert_eq!(emission_for_round(p.halving_interval_rounds - 1, &p), 100);

    // At/after first halving
    assert_eq!(emission_for_round(p.halving_interval_rounds, &p), 50);
    assert_eq!(emission_for_round(p.halving_interval_rounds + 1, &p), 50);
}

#[test]
fn test_fee_cap() {
    let p = EconomicsParams::default();
    let emission = 1000u128; // μIPN
    let parts = demo_parts();
    // 10% cap -> fees must be <= 100
    let ok = distribute_round(emission, 100, &parts, &p).is_ok();
    let bad = distribute_round(emission, 101, &parts, &p).is_err();
    assert!(ok && bad);
}

#[test]
fn test_distribution_weights() {
    let p = EconomicsParams::default();
    let emission = 1_000_000u128; // 1 IPN
    let fees = 0u128;

    let parts = demo_parts(); // A proposer 10 blocks; B verifier 10 blocks

    let (payouts, _, _) = distribute_round(emission, fees, &parts, &p).unwrap();
    let a = payouts.get(&ValidatorId("A".into())).copied().unwrap_or(0);
    let b = payouts.get(&ValidatorId("B".into())).copied().unwrap_or(0);

    // A has 1.2x weight vs B
    assert!(a > b);
}

#[test]
fn test_hard_cap_enforcement() {
    let p = EconomicsParams::default();
    let already_issued = p.hard_cap_micro - 50; // Almost at cap
    
    // Should get remaining 50 μIPN
    let emission = emission_for_round_capped(0, already_issued, &p).unwrap();
    assert_eq!(emission, 50);
    
    // Should fail when cap is exceeded
    let already_issued = p.hard_cap_micro;
    let result = emission_for_round_capped(0, already_issued, &p);
    assert!(result.is_err());
}

#[test]
fn test_empty_participation() {
    let p = EconomicsParams::default();
    let parts = ParticipationSet::default();
    let (payouts, emission_paid, fees_paid) = distribute_round(1000, 0, &parts, &p).unwrap();
    
    assert!(payouts.is_empty());
    assert_eq!(emission_paid, 0);
    assert_eq!(fees_paid, 0);
}

#[test]
fn test_epoch_auto_burn() {
    // No burn needed
    let burn = epoch_auto_burn(1000, 1000);
    assert_eq!(burn, 0);
    
    // Need to burn excess
    let burn = epoch_auto_burn(1000, 1200);
    assert_eq!(burn, 200);
}

#[test]
fn test_sum_emission_over_rounds() {
    let p = EconomicsParams::default();
    let sum = sum_emission_over_rounds(0, 2, |r| emission_for_round(r, &p));
    assert_eq!(sum, 300); // 100 + 100 + 100
}

#[test]
fn test_micro_ipn_conversion() {
    assert_eq!(MICRO_PER_IPN, 1_000_000);
    
    // Test conversion examples
    let ipn_amount = 1.5; // 1.5 IPN
    let micro_amount = (ipn_amount * MICRO_PER_IPN as f64) as u128;
    assert_eq!(micro_amount, 1_500_000);
}

#[test]
fn test_economics_params_default() {
    let p = EconomicsParams::default();
    
    // Check hard cap: 21M IPN = 21B μIPN
    assert_eq!(p.hard_cap_micro, 21_000_000 * MICRO_PER_IPN);
    
    // Check initial reward: 0.0001 IPN = 100 μIPN
    assert_eq!(p.initial_round_reward_micro, 100);
    
    // Check halving interval: ~2 years at 10 rounds/s
    assert_eq!(p.halving_interval_rounds, 630_720_000);
    
    // Check fee cap: 10%
    assert_eq!(p.fee_cap_fraction(), (1, 10));
    
    // Check role weights
    assert_eq!(p.role_weight_milli(true), 1200);  // Proposer
    assert_eq!(p.role_weight_milli(false), 1000); // Verifier
}

#[test]
fn test_distribution_proportionality() {
    let p = EconomicsParams::default();
    let emission = 1_000_000u128; // 1 IPN
    let fees = 100_000u128; // 0.1 IPN
    
    let mut parts = ParticipationSet::default();
    // A: proposer with 5 blocks (weight 1200)
    parts.insert(
        ValidatorId("A".into()),
        Participation { role: Role::Proposer, blocks: 5 },
    );
    // B: verifier with 10 blocks (weight 1000)
    parts.insert(
        ValidatorId("B".into()),
        Participation { role: Role::Verifier, blocks: 10 },
    );
    
    let (payouts, _, _) = distribute_round(emission, fees, &parts, &p).unwrap();
    
    let a_payout = payouts.get(&ValidatorId("A".into())).copied().unwrap_or(0);
    let b_payout = payouts.get(&ValidatorId("B".into())).copied().unwrap_or(0);
    
    // A: 5 * 1200 = 6000 weighted blocks
    // B: 10 * 1000 = 10000 weighted blocks
    // Total: 16000 weighted blocks
    // Pool: 1_000_000 + 100_000 = 1_100_000 μIPN
    
    // A should get: 1_100_000 * 6000 / 16000 = 412_500 μIPN
    // B should get: 1_100_000 * 10000 / 16000 = 687_500 μIPN
    assert_eq!(a_payout, 412_500);
    assert_eq!(b_payout, 687_500);
}

fn demo_parts() -> ParticipationSet {
    use crate::types::{Participation, Role};
    let mut ps = ParticipationSet::default();
    ps.insert(
        ValidatorId("A".into()),
        Participation { role: Role::Proposer, blocks: 10 },
    );
    ps.insert(
        ValidatorId("B".into()),
        Participation { role: Role::Verifier, blocks: 10 },
    );
    ps
}