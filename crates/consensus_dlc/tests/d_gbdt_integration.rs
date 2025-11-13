//! Integration tests for D-GBDT scoring in consensus
//!
//! Tests the full integration of deterministic GBDT scoring
//! into the validator selection mechanism.

#![cfg(feature = "d_gbdt")]
#![allow(deprecated)] // Allow use of from_floats() for backward compatibility

use ippan_consensus_dlc::{
    dgbdt::{FairnessModel, ValidatorMetrics},
    scoring::d_gbdt::{score_validator, score_validators, ValidatorSnapshot, SCALE},
    verifier::{ValidatorSetManager, VerifierSet},
};
use ippan_types::Amount;
use std::collections::HashMap;

/// Load test model from JSON
fn load_test_model() -> ippan_ai_core::gbdt::GBDTModel {
    let json_path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/resources/test_model.json");
    let json_content = std::fs::read_to_string(json_path)
        .expect("Failed to read test model JSON");
    serde_json::from_str(&json_content).expect("Failed to parse test model JSON")
}

/// Create test validator snapshots
fn create_test_snapshots() -> Vec<ValidatorSnapshot> {
    vec![
        ValidatorSnapshot {
            validator_id: "validator_excellent".to_string(),
            uptime_ms: 86_400_000,        // 24h
            missed_rounds: 0,
            response_ms_p50: 100,         // 100ms
            stake_i64_scaled: 1_000_000_000,
            slash_count: 0,
            last_24h_blocks: 500,
            age_rounds: 100_000,
        },
        ValidatorSnapshot {
            validator_id: "validator_good".to_string(),
            uptime_ms: 75_600_000,        // 21h
            missed_rounds: 10,
            response_ms_p50: 200,         // 200ms
            stake_i64_scaled: 500_000_000,
            slash_count: 0,
            last_24h_blocks: 400,
            age_rounds: 50_000,
        },
        ValidatorSnapshot {
            validator_id: "validator_medium".to_string(),
            uptime_ms: 43_200_000,        // 12h
            missed_rounds: 50,
            response_ms_p50: 500,         // 500ms
            stake_i64_scaled: 250_000_000,
            slash_count: 1,
            last_24h_blocks: 200,
            age_rounds: 25_000,
        },
        ValidatorSnapshot {
            validator_id: "validator_poor".to_string(),
            uptime_ms: 21_600_000,        // 6h
            missed_rounds: 200,
            response_ms_p50: 2000,        // 2s
            stake_i64_scaled: 50_000_000,
            slash_count: 3,
            last_24h_blocks: 50,
            age_rounds: 5_000,
        },
    ]
}

#[test]
fn test_score_validator_with_model() {
    let model = load_test_model();
    let snapshot = ValidatorSnapshot {
        validator_id: "test".to_string(),
        uptime_ms: 86_400_000,
        missed_rounds: 0,
        response_ms_p50: 100,
        stake_i64_scaled: 1_000_000_000,
        slash_count: 0,
        last_24h_blocks: 500,
        age_rounds: 100_000,
    };

    let score = score_validator(&snapshot, Some(&model)).expect("Scoring should succeed");
    assert!(score > 0, "Score should be positive for excellent validator");
    assert!(score <= SCALE * 10, "Score should not exceed maximum");
}

#[test]
fn test_score_validator_without_model() {
    let snapshot = ValidatorSnapshot {
        validator_id: "test".to_string(),
        uptime_ms: 86_400_000,
        missed_rounds: 0,
        response_ms_p50: 100,
        stake_i64_scaled: 1_000_000_000,
        slash_count: 0,
        last_24h_blocks: 500,
        age_rounds: 100_000,
    };

    // Should fall back to default PoA scoring
    let score = score_validator(&snapshot, None).expect("Scoring should succeed");
    assert!(score > 0, "Score should be positive even without model");
}

#[test]
fn test_score_validators_ranking() {
    let model = load_test_model();
    let snapshots = create_test_snapshots();

    let results = score_validators(&snapshots, Some(&model))
        .expect("Scoring multiple validators should succeed");

    assert_eq!(results.len(), 4, "Should score all validators");

    // Verify ordering: excellent > good > medium > poor
    assert_eq!(results[0].0, "validator_excellent", "Best validator should rank first");
    assert_eq!(results[3].0, "validator_poor", "Worst validator should rank last");

    // Verify scores are descending
    for i in 0..results.len() - 1 {
        assert!(
            results[i].1 >= results[i + 1].1,
            "Scores should be in descending order"
        );
    }
}

#[test]
fn test_deterministic_scoring() {
    let model = load_test_model();
    let snapshots = create_test_snapshots();

    let results1 = score_validators(&snapshots, Some(&model))
        .expect("First scoring should succeed");
    let results2 = score_validators(&snapshots, Some(&model))
        .expect("Second scoring should succeed");

    assert_eq!(results1.len(), results2.len());
    for (r1, r2) in results1.iter().zip(results2.iter()) {
        assert_eq!(r1.0, r2.0, "Validator IDs should match");
        assert_eq!(r1.1, r2.1, "Scores should be deterministic");
        assert_eq!(r1.2, r2.2, "Weights should be deterministic");
    }
}

#[test]
fn test_verifier_set_selection_with_d_gbdt() {
    let model = load_test_model();
    let mut validators = HashMap::new();

    // Create validators with metrics
    validators.insert(
        "validator_excellent".to_string(),
        
            ValidatorMetrics::from_floats(
            0.99,
            0.1,
            1.0,
            500,
            500,
            Amount::from_micro_ipn(1_000_000_000),
            100_000,
        ),
    );
    validators.insert(
        "validator_good".to_string(),
        
            ValidatorMetrics::from_floats(
            0.95,
            0.2,
            0.98,
            400,
            400,
            Amount::from_micro_ipn(500_000_000),
            50_000,
        ),
    );
    validators.insert(
        "validator_medium".to_string(),
        
            ValidatorMetrics::from_floats(
            0.85,
            0.5,
            0.95,
            200,
            200,
            Amount::from_micro_ipn(250_000_000),
            25_000,
        ),
    );

    let verifier_set = VerifierSet::select_with_d_gbdt(
        Some(&model),
        &validators,
        "test_seed",
        1,
        3,
    )
    .expect("Verifier set selection should succeed");

    assert_eq!(verifier_set.round, 1);
    assert!(!verifier_set.primary.is_empty(), "Should select a primary");
    assert!(verifier_set.size() <= 3, "Should not exceed max set size");
    assert!(verifier_set.size() >= 1, "Should have at least the primary");
}

#[test]
fn test_verifier_set_selection_deterministic() {
    let model = load_test_model();
    let mut validators = HashMap::new();

    validators.insert(
        "val1".to_string(),
        
            ValidatorMetrics::from_floats(
            0.99,
            0.1,
            1.0,
            100,
            500,
            Amount::from_micro_ipn(10_000_000),
            1000,
        ),
    );
    validators.insert(
        "val2".to_string(),
        
            ValidatorMetrics::from_floats(
            0.95,
            0.2,
            0.98,
            80,
            400,
            Amount::from_micro_ipn(5_000_000),
            800,
        ),
    );

    let set1 = VerifierSet::select_with_d_gbdt(
        Some(&model),
        &validators,
        "seed123",
        1,
        2,
    )
    .expect("First selection should succeed");

    let set2 = VerifierSet::select_with_d_gbdt(
        Some(&model),
        &validators,
        "seed123",
        1,
        2,
    )
    .expect("Second selection should succeed");

    assert_eq!(set1.primary, set2.primary, "Primary should be deterministic");
    assert_eq!(set1.shadows, set2.shadows, "Shadows should be deterministic");
}

#[test]
fn test_validator_set_manager_with_gbdt_model() {
    let model = load_test_model();
    let fairness_model = FairnessModel::new_production();
    let mut manager = ValidatorSetManager::new(fairness_model, 3);

    // Register validators
    manager
        .register_validator(
            "val1".to_string(),
            
            ValidatorMetrics::from_floats(
                0.99,
                0.1,
                1.0,
                100,
                500,
                Amount::from_micro_ipn(10_000_000),
                1000,
            ),
        )
        .expect("Registration should succeed");

    manager
        .register_validator(
            "val2".to_string(),
            
            ValidatorMetrics::from_floats(
                0.95,
                0.2,
                0.98,
                80,
                400,
                Amount::from_micro_ipn(5_000_000),
                800,
            ),
        )
        .expect("Registration should succeed");

    // Set GBDT model
    manager.set_gbdt_model(model);

    // Select verifiers
    let verifier_set = manager
        .select_for_round("test_seed".to_string(), 1)
        .expect("Selection should succeed");

    assert!(!verifier_set.primary.is_empty());
    assert_eq!(verifier_set.round, 1);
}

#[test]
fn test_validator_set_manager_falls_back_without_model() {
    let fairness_model = FairnessModel::new_production();
    let mut manager = ValidatorSetManager::new(fairness_model, 3);

    // Register validators
    manager
        .register_validator(
            "val1".to_string(),
            ValidatorMetrics::default(),
        )
        .expect("Registration should succeed");

    // Select verifiers without GBDT model (should use legacy FairnessModel)
    let verifier_set = manager
        .select_for_round("test_seed".to_string(), 1)
        .expect("Selection should succeed with fallback");

    assert!(!verifier_set.primary.is_empty());
}

#[test]
fn test_mini_round_with_d_gbdt() {
    // This simulates a mini consensus round with D-GBDT scoring
    let model = load_test_model();
    let fairness_model = FairnessModel::new_production();
    let mut manager = ValidatorSetManager::new(fairness_model, 5);

    // Register 5 validators with varying quality
    let validators = vec![
        ("excellent", 0.99, 0.05, 1.0, 500, 100_000),
        ("good", 0.95, 0.10, 0.98, 400, 80_000),
        ("medium", 0.90, 0.20, 0.95, 300, 50_000),
        ("poor", 0.80, 0.50, 0.90, 200, 20_000),
        ("worst", 0.70, 1.00, 0.85, 100, 10_000),
    ];

    for (name, uptime, latency, honesty, blocks, stake) in validators {
        manager
            .register_validator(
                name.to_string(),
                
            ValidatorMetrics::from_floats(
                    uptime,
                    latency,
                    honesty,
                    blocks,
                    blocks,
                    Amount::from_micro_ipn(stake),
                    1000,
                ),
            )
            .expect("Registration should succeed");
    }

    // Set GBDT model
    manager.set_gbdt_model(model);

    // Run multiple rounds to test stability
    let mut primary_selections = std::collections::HashMap::new();
    for round in 1..=5 {
        let verifier_set = manager
            .select_for_round(format!("seed_{}", round), round)
            .expect("Selection should succeed");

        *primary_selections.entry(verifier_set.primary.clone()).or_insert(0) += 1;

        // Should select the configured number of verifiers
        assert!(verifier_set.size() <= 5);
        assert!(verifier_set.size() >= 1);
        
        // Verify worst validators are not primary
        assert!(
            verifier_set.primary != "worst",
            "Worst validator should never be primary in round {}",
            round
        );
    }
    
    // Over 5 rounds, the best validators should be selected more often
    let best_count = *primary_selections.get("excellent").unwrap_or(&0) 
        + *primary_selections.get("good").unwrap_or(&0);
    let worst_count = *primary_selections.get("poor").unwrap_or(&0);
    
    assert!(
        best_count >= worst_count,
        "Best validators should be selected more often than poor validators"
    );
}

#[test]
fn test_slash_penalty_affects_scoring() {
    // Test with no model to verify the default PoA scoring penalizes slashes
    let snapshot_no_slash = ValidatorSnapshot {
        validator_id: "clean".to_string(),
        uptime_ms: 86_400_000,
        missed_rounds: 0,
        response_ms_p50: 100,
        stake_i64_scaled: 1_000_000_000,
        slash_count: 0,
        last_24h_blocks: 500,
        age_rounds: 100_000,
    };

    let snapshot_with_slash = ValidatorSnapshot {
        validator_id: "slashed".to_string(),
        uptime_ms: 86_400_000,
        missed_rounds: 0,
        response_ms_p50: 100,
        stake_i64_scaled: 1_000_000_000,
        slash_count: 5,
        last_24h_blocks: 500,
        age_rounds: 100_000,
    };

    // Test with default PoA scoring (no model)
    let score_clean = score_validator(&snapshot_no_slash, None)
        .expect("Scoring should succeed");
    let score_slashed = score_validator(&snapshot_with_slash, None)
        .expect("Scoring should succeed");

    assert!(
        score_clean > score_slashed,
        "Validator without slashes should score higher (clean: {}, slashed: {})",
        score_clean, score_slashed
    );
    
    // Verify the feature extraction reflects the slash penalty
    use ippan_consensus_dlc::scoring::d_gbdt::extract_features;
    let features_clean = extract_features(&snapshot_no_slash);
    let features_slashed = extract_features(&snapshot_with_slash);
    
    // Feature 4 is slash score - should be lower for slashed validator
    assert!(
        features_clean[4] > features_slashed[4],
        "Slash feature should reflect penalty"
    );
}
