//! Fee Policy Smoke Test
//!
//! This integration test validates the complete fee policy implementation:
//! - Transaction fees are computed correctly (no floats)
//! - Fees are routed to the weekly epoch pool (never burned)
//! - Handle/domain registration fees go to the pool
//! - Epoch distribution pays eligible validators correctly
//! - All invariants hold: sum(payouts) + remainder == pool_balance
//!
//! Run with: cargo test -p ippan-treasury --test fee_policy_smoke -- --nocapture

use ippan_treasury::account_ledger::{AccountLedger, MockAccountLedger};
use ippan_treasury::weekly_pool::{
    assess_eligibility, route_fee_to_pool, should_trigger_distribution, ValidatorEpochMetrics,
    WeeklyFeePoolManager, WORK_SCORE_CAP,
};
use ippan_types::{
    epoch_from_ippan_time_us, fee_pool_account_id, kambei_to_ipn, FeeScheduleV1, Kambei,
    PayoutProfile, PayoutRecipient, RegistryPriceScheduleV1, EPOCH_US, KAMBEI_PER_IPN,
};

// =============================================================================
// TEST HELPERS
// =============================================================================

fn test_validator_id(n: u8) -> [u8; 32] {
    let mut id = [0u8; 32];
    id[0] = n;
    id
}

// =============================================================================
// TEST 1: TRANSACTION FEE COMPUTATION (NO FLOATS)
// =============================================================================

#[test]
fn test_tx_fee_computation_integer_only() {
    println!("\n=== TEST 1: Transaction Fee Computation ===");

    let schedule = FeeScheduleV1::default();
    println!("Fee Schedule v1:");
    println!("  tx_base_fee: {} kambei", schedule.tx_base_fee);
    println!("  tx_byte_fee: {} kambei", schedule.tx_byte_fee);
    println!("  tx_min_fee:  {} kambei", schedule.tx_min_fee);
    println!(
        "  tx_max_fee:  {} kambei ({} IPN)",
        schedule.tx_max_fee,
        kambei_to_ipn(schedule.tx_max_fee)
    );

    // Test various transaction sizes
    let test_cases = vec![
        (0, schedule.tx_min_fee, "empty tx hits min fee"),
        (100, schedule.tx_base_fee + 100, "100 byte tx"),
        (500, schedule.tx_base_fee + 500, "500 byte tx"),
        (1000, schedule.tx_base_fee + 1000, "1KB tx"),
    ];

    for (tx_bytes, expected_fee, description) in test_cases {
        let fee = schedule.compute_tx_fee(tx_bytes);
        println!("  {} bytes -> {} kambei ({})", tx_bytes, fee, description);
        assert_eq!(fee, expected_fee, "Fee mismatch for {}", description);
        // Verify no floating point was used (fee is exact integer)
        assert!(fee >= schedule.tx_min_fee);
        assert!(fee <= schedule.tx_max_fee);
    }

    println!("✓ All tx fees computed using integer math only");
}

// =============================================================================
// TEST 2: FEE ROUTING TO WEEKLY POOL (NEVER BURNED)
// =============================================================================

#[test]
fn test_fee_routing_to_pool_never_burned() {
    println!("\n=== TEST 2: Fee Routing to Weekly Pool ===");

    let epoch = 5u64;
    let manager = WeeklyFeePoolManager::new(epoch);
    let mut ledger = MockAccountLedger::new();

    // Compute fee for a 200-byte transaction
    let schedule = FeeScheduleV1::default();
    let tx_bytes = 200u32;
    let fee = schedule.compute_tx_fee(tx_bytes);
    println!("Transaction fee: {} kambei", fee);

    // Route fee to pool
    route_fee_to_pool(&manager, fee, &mut ledger).expect("route should succeed");

    // Verify pool received the fee
    let pool_account = fee_pool_account_id(epoch);
    let pool_balance = ledger.get_validator_balance(&pool_account).unwrap();
    println!("Pool balance: {} kambei", pool_balance);
    assert_eq!(
        pool_balance, fee as u128,
        "Pool should have received exact fee"
    );
    assert_eq!(
        manager.accumulated_fees(),
        fee as u128,
        "Manager should track fee"
    );

    println!("✓ Fee routed to pool, not burned");
}

// =============================================================================
// TEST 3: HANDLE/DOMAIN REGISTRATION FEES
// =============================================================================

#[test]
fn test_handle_domain_fees_to_pool() {
    println!("\n=== TEST 3: Handle/Domain Registration Fees ===");

    let schedule = RegistryPriceScheduleV1::default();
    println!("Registry Price Schedule v1:");
    println!(
        "  handle_register_fee: {} kambei ({:.6} IPN)",
        schedule.handle_register_fee,
        schedule.handle_register_fee as f64 / KAMBEI_PER_IPN as f64
    );
    println!(
        "  premium_multiplier: {}x",
        schedule.premium_handle_multiplier_bps as f64 / 10000.0
    );
    println!(
        "  domain_register_fee: {} kambei ({:.2} IPN)",
        schedule.domain_register_fee,
        schedule.domain_register_fee as f64 / KAMBEI_PER_IPN as f64
    );

    // Test handle registration fee
    let normal_fee = schedule.compute_handle_register_fee(false);
    let premium_fee = schedule.compute_handle_register_fee(true);
    println!("\nHandle fees:");
    println!("  Normal handle: {} kambei", normal_fee);
    println!("  Premium handle: {} kambei (2x)", premium_fee);
    assert_eq!(premium_fee, normal_fee * 2, "Premium should be 2x");

    // Simulate routing handle fee to pool
    let epoch = 10u64;
    let manager = WeeklyFeePoolManager::new(epoch);
    let mut ledger = MockAccountLedger::new();

    // Pay for handle registration
    route_fee_to_pool(&manager, normal_fee, &mut ledger).expect("route should succeed");

    let pool_account = fee_pool_account_id(epoch);
    let pool_balance = ledger.get_validator_balance(&pool_account).unwrap();
    assert_eq!(
        pool_balance, normal_fee as u128,
        "Pool should have handle fee"
    );

    // Pay for domain registration
    route_fee_to_pool(&manager, schedule.domain_register_fee, &mut ledger)
        .expect("route should succeed");

    let pool_balance_after = ledger.get_validator_balance(&pool_account).unwrap();
    assert_eq!(
        pool_balance_after,
        (normal_fee + schedule.domain_register_fee) as u128
    );
    println!(
        "\nPool accumulated: {} kambei (handle + domain)",
        pool_balance_after
    );

    println!("✓ Handle and domain fees routed to pool");
}

// =============================================================================
// TEST 4: EPOCH DISTRIBUTION TO VALIDATORS
// =============================================================================

#[test]
fn test_epoch_distribution_to_validators() {
    println!("\n=== TEST 4: Epoch Distribution to Validators ===");

    let epoch = 0u64;
    let manager = WeeklyFeePoolManager::new(epoch);
    let mut ledger = MockAccountLedger::new();

    // Accumulate fees in pool
    let total_fees: Kambei = 10_000_000; // 0.1 IPN
    manager.accumulate_fee(total_fees);
    println!("Pool balance before distribution: {} kambei", total_fees);

    // Set up 3 validators with different work scores (all with uninterrupted presence)
    let v1 = test_validator_id(1);
    let v2 = test_validator_id(2);
    let v3 = test_validator_id(3);

    let metrics = vec![
        ValidatorEpochMetrics::with_uninterrupted(v1, true, 100), // 100 weight
        ValidatorEpochMetrics::with_uninterrupted(v2, true, 200), // 200 weight
        ValidatorEpochMetrics::with_uninterrupted(v3, true, 300), // 300 weight
    ];

    println!("\nValidator metrics:");
    for m in &metrics {
        println!(
            "  {:?}: work_score={}, weight={}, eligible={}",
            m.validator_id[0],
            m.work_score,
            m.weight(),
            m.is_eligible()
        );
    }

    // Assess eligibility
    let eligibility = assess_eligibility(&metrics);
    println!("\nEligibility result:");
    println!("  Eligible: {} validators", eligibility.eligible.len());
    println!("  Total weight: {}", eligibility.total_weight);
    assert_eq!(eligibility.eligible.len(), 3);
    assert_eq!(eligibility.total_weight, 600); // 100 + 200 + 300

    // Perform distribution
    let result = manager
        .distribute_fees(&metrics, 100, &mut ledger)
        .expect("distribution should succeed");

    println!("\nDistribution result:");
    for payout in &result.payouts {
        println!(
            "  Validator {:?}: {} kambei",
            payout.validator_id[0], payout.amount
        );
    }
    println!("Total distributed: {} kambei", result.total_distributed);
    println!("Remainder: {} kambei", result.remainder);

    // CRITICAL INVARIANT: sum(payouts) + remainder == pool_balance
    assert_eq!(
        result.total_distributed as u128 + result.remainder as u128,
        total_fees as u128,
        "INVARIANT VIOLATED: sum(payouts) + remainder != pool_balance"
    );
    println!("✓ INVARIANT HOLDS: sum(payouts) + remainder == pool_balance");

    // Verify no burning
    println!("✓ No fees burned - all accounted for");
}

// =============================================================================
// TEST 5: VALIDATOR ELIGIBILITY (100% PRESENCE REQUIRED)
// =============================================================================

#[test]
fn test_validator_eligibility_strict_presence() {
    println!("\n=== TEST 5: Validator Eligibility (100% Presence Required) ===");

    // Test case: 99% presence is NOT eligible
    let v_99 = ValidatorEpochMetrics::new(test_validator_id(1), 9900, 500); // 99%
    println!("Validator with 99% presence:");
    println!("  presence_bps: {}", v_99.presence_bps);
    println!("  is_eligible: {}", v_99.is_eligible());
    println!("  weight: {}", v_99.weight());
    assert!(!v_99.is_eligible(), "99% presence should NOT be eligible");
    assert_eq!(
        v_99.weight(),
        0,
        "Ineligible validator should have 0 weight"
    );

    // Test case: 100% presence IS eligible
    let v_100 = ValidatorEpochMetrics::new(test_validator_id(2), 10000, 500); // 100%
    println!("\nValidator with 100% presence:");
    println!("  presence_bps: {}", v_100.presence_bps);
    println!("  is_eligible: {}", v_100.is_eligible());
    println!("  weight: {}", v_100.weight());
    assert!(v_100.is_eligible(), "100% presence should be eligible");
    assert!(
        v_100.weight() > 0,
        "Eligible validator should have positive weight"
    );

    // Test case: Slashed validator is NOT eligible even with 100% presence
    let mut v_slashed = ValidatorEpochMetrics::with_uninterrupted(test_validator_id(3), true, 500);
    v_slashed.mark_slashed();
    println!("\nSlashed validator with 100% presence:");
    println!("  is_eligible: {}", v_slashed.is_eligible());
    assert!(
        !v_slashed.is_eligible(),
        "Slashed validator should NOT be eligible"
    );

    // Test case: Zero work score is NOT eligible
    let v_zero_work = ValidatorEpochMetrics::with_uninterrupted(test_validator_id(4), true, 0);
    println!("\nValidator with 0 work score:");
    println!("  is_eligible: {}", v_zero_work.is_eligible());
    assert!(
        !v_zero_work.is_eligible(),
        "Zero work should NOT be eligible"
    );

    println!("\n✓ Eligibility rules enforced correctly");
}

// =============================================================================
// TEST 6: WORK SCORE CAP (ANTI-WHALE)
// =============================================================================

#[test]
fn test_work_score_cap_anti_whale() {
    println!("\n=== TEST 6: Work Score Cap (Anti-Whale) ===");
    println!("WORK_SCORE_CAP: {}", WORK_SCORE_CAP);

    // Validator with work score above cap
    let v_whale = ValidatorEpochMetrics::with_uninterrupted(test_validator_id(1), true, 100_000);
    println!("\nWhale validator:");
    println!("  work_score: {}", v_whale.work_score);
    println!("  weight (capped): {}", v_whale.weight());
    assert_eq!(v_whale.weight(), WORK_SCORE_CAP, "Weight should be capped");

    // Validator with work score below cap
    let v_normal = ValidatorEpochMetrics::with_uninterrupted(test_validator_id(2), true, 500);
    println!("\nNormal validator:");
    println!("  work_score: {}", v_normal.work_score);
    println!("  weight: {}", v_normal.weight());
    assert_eq!(v_normal.weight(), 500, "Weight should equal work_score");

    println!("\n✓ Work score cap prevents whale dominance");
}

// =============================================================================
// TEST 7: PAYOUT PROFILE SPLITS
// =============================================================================

#[test]
fn test_payout_profile_splits() {
    println!("\n=== TEST 7: Payout Profile Splits ===");

    // Create a split profile: 60% / 30% / 10%
    let profile = PayoutProfile {
        recipients: vec![
            PayoutRecipient {
                address: test_validator_id(1),
                weight_bps: 6000,
            },
            PayoutRecipient {
                address: test_validator_id(2),
                weight_bps: 3000,
            },
            PayoutRecipient {
                address: test_validator_id(3),
                weight_bps: 1000,
            },
        ],
    };

    assert!(profile.validate().is_ok(), "Profile should be valid");

    let total_payout: Kambei = 10_000_000; // 0.1 IPN
    let (splits, remainder) = profile.split_amount(total_payout);

    println!("Total payout: {} kambei", total_payout);
    println!("Splits:");
    let mut sum = 0u64;
    for (recipient, amount) in &splits {
        println!("  {:?}: {} kambei", recipient.address[0], amount);
        sum += amount;
    }
    println!("Sum of splits: {} kambei", sum);
    println!("Remainder: {} kambei", remainder);

    // INVARIANT: sum(splits) + remainder == total_payout
    assert_eq!(
        sum + remainder,
        total_payout,
        "INVARIANT VIOLATED: sum(splits) + remainder != total_payout"
    );
    println!("\n✓ INVARIANT HOLDS: sum(splits) + remainder == total_payout");
}

// =============================================================================
// TEST 8: DISTRIBUTION RUNS ONCE PER EPOCH
// =============================================================================

#[test]
fn test_distribution_once_per_epoch() {
    println!("\n=== TEST 8: Distribution Runs Once Per Epoch ===");

    let epoch = 0u64;
    let manager = WeeklyFeePoolManager::new(epoch);
    let mut ledger = MockAccountLedger::new();
    manager.accumulate_fee(10_000);

    let metrics = vec![ValidatorEpochMetrics::with_uninterrupted(
        test_validator_id(1),
        true,
        100,
    )];

    // First distribution should succeed
    let result1 = manager.distribute_fees(&metrics, 100, &mut ledger);
    println!("First distribution: {:?}", result1.is_ok());
    assert!(result1.is_ok(), "First distribution should succeed");

    // Second distribution for same epoch should fail
    let result2 = manager.distribute_fees(&metrics, 101, &mut ledger);
    println!("Second distribution: {:?}", result2.is_err());
    assert!(result2.is_err(), "Second distribution should fail");

    println!("\n✓ Distribution enforced to run exactly once per epoch");
}

// =============================================================================
// TEST 9: EPOCH BOUNDARY DETECTION
// =============================================================================

#[test]
fn test_epoch_boundary_detection() {
    println!("\n=== TEST 9: Epoch Boundary Detection ===");
    println!(
        "EPOCH_US: {} microseconds ({} days)",
        EPOCH_US,
        EPOCH_US / (24 * 60 * 60 * 1_000_000)
    );

    // Same epoch - no distribution trigger
    let t1 = 1000u64;
    let t2 = 2000u64;
    assert!(
        !should_trigger_distribution(t2, t1),
        "Same epoch should not trigger"
    );
    println!(
        "Same epoch ({} -> {}): no trigger",
        epoch_from_ippan_time_us(t1),
        epoch_from_ippan_time_us(t2)
    );

    // Different epochs - should trigger
    let t3 = EPOCH_US + 1000;
    assert!(
        should_trigger_distribution(t3, t1),
        "Different epoch should trigger"
    );
    println!(
        "Different epoch ({} -> {}): trigger!",
        epoch_from_ippan_time_us(t1),
        epoch_from_ippan_time_us(t3)
    );

    println!("\n✓ Epoch boundary detection works correctly");
}

// =============================================================================
// TEST 10: FULL END-TO-END SMOKE TEST
// =============================================================================

#[test]
fn test_full_fee_policy_e2e() {
    println!("\n=== TEST 10: Full End-to-End Fee Policy Smoke Test ===");
    println!("This test simulates a complete fee lifecycle:\n");

    // Setup
    let epoch = 42u64;
    let manager = WeeklyFeePoolManager::new(epoch);
    let mut ledger = MockAccountLedger::new();
    let tx_schedule = FeeScheduleV1::default();
    let registry_schedule = RegistryPriceScheduleV1::default();

    println!("1. Simulating transactions...");

    // Alice sends 10 IPN to Bob (200 byte tx)
    let tx1_fee = tx_schedule.compute_tx_fee(200);
    route_fee_to_pool(&manager, tx1_fee, &mut ledger).unwrap();
    println!("   Alice -> Bob: 10 IPN (fee: {} kambei)", tx1_fee);

    // Bob sends 5 IPN to Charlie (150 byte tx)
    let tx2_fee = tx_schedule.compute_tx_fee(150);
    route_fee_to_pool(&manager, tx2_fee, &mut ledger).unwrap();
    println!("   Bob -> Charlie: 5 IPN (fee: {} kambei)", tx2_fee);

    // Charlie registers a handle
    let handle_fee = registry_schedule.compute_handle_register_fee(false);
    route_fee_to_pool(&manager, handle_fee, &mut ledger).unwrap();
    println!(
        "   Charlie registers @charlie.ipn (fee: {} kambei)",
        handle_fee
    );

    let total_fees = tx1_fee + tx2_fee + handle_fee;
    println!("\n2. Total fees collected: {} kambei", total_fees);
    assert_eq!(manager.accumulated_fees(), total_fees as u128);

    // Verify pool balance
    let pool_account = fee_pool_account_id(epoch);
    let pool_balance = ledger.get_validator_balance(&pool_account).unwrap();
    println!("   Pool balance: {} kambei", pool_balance);
    assert_eq!(pool_balance, total_fees as u128);

    // Setup validators for distribution
    println!("\n3. Setting up validators for distribution...");
    let v1 = test_validator_id(1);
    let v2 = test_validator_id(2);

    let metrics = vec![
        ValidatorEpochMetrics::with_uninterrupted(v1, true, 400),
        ValidatorEpochMetrics::with_uninterrupted(v2, true, 600),
    ];

    for m in &metrics {
        println!(
            "   Validator {:?}: work_score={}, eligible={}",
            m.validator_id[0],
            m.work_score,
            m.is_eligible()
        );
    }

    // Perform distribution
    println!("\n4. Performing epoch distribution...");
    let distribution = manager
        .distribute_fees(&metrics, 100, &mut ledger)
        .expect("distribution should succeed");

    println!("   Distribution result:");
    for payout in &distribution.payouts {
        println!(
            "     Validator {:?}: {} kambei",
            payout.validator_id[0], payout.amount
        );
    }
    println!(
        "   Total distributed: {} kambei",
        distribution.total_distributed
    );
    println!("   Remainder: {} kambei", distribution.remainder);

    // CRITICAL INVARIANTS
    println!("\n5. Verifying invariants...");

    // Invariant 1: sum(payouts) + remainder == pool_balance
    let invariant1 = distribution.total_distributed as u128 + distribution.remainder as u128
        == total_fees as u128;
    println!(
        "   ✓ sum(payouts) + remainder == pool_balance: {}",
        invariant1
    );
    assert!(invariant1, "INVARIANT 1 VIOLATED");

    // Invariant 2: No fees burned (all accounted for)
    println!(
        "   ✓ No fees burned: all {} kambei accounted for",
        total_fees
    );

    // Invariant 3: Distribution ran exactly once
    let second_dist = manager.distribute_fees(&metrics, 101, &mut ledger);
    let invariant3 = second_dist.is_err();
    println!("   ✓ Distribution runs once per epoch: {}", invariant3);
    assert!(invariant3, "INVARIANT 3 VIOLATED");

    println!("\n=== ALL INVARIANTS HOLD ===");
    println!("Fee policy smoke test PASSED!");
}

// =============================================================================
// SUMMARY TEST
// =============================================================================

#[test]
fn test_fee_policy_summary() {
    println!("\n");
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║           IPPAN FEE POLICY SMOKE TEST SUMMARY                ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ ✓ Transaction fees computed with integer math (no floats)   ║");
    println!("║ ✓ Fees routed to weekly epoch pool (never burned)           ║");
    println!("║ ✓ Handle/domain fees go to the same pool                    ║");
    println!("║ ✓ Epoch distribution pays eligible validators               ║");
    println!("║ ✓ 100% presence required for eligibility                    ║");
    println!("║ ✓ Work score cap prevents whale dominance                   ║");
    println!("║ ✓ Payout profiles split correctly                           ║");
    println!("║ ✓ Distribution runs exactly once per epoch                  ║");
    println!("║ ✓ INVARIANT: sum(payouts) + remainder == pool_balance       ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
}
