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

fn demo_parts() -> ParticipationSet {
    use crate::types::{Participation, Role};
    let mut ps = ParticipationSet::default();
    ps.insert(
        ValidatorId("A".into()),
        Participation {
            role: Role::Proposer,
            blocks: 10,
        },
    );
    ps.insert(
        ValidatorId("B".into()),
        Participation {
            role: Role::Verifier,
            blocks: 10,
        },
    );
    ps
}
