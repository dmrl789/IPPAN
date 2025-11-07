//! Comprehensive unit tests for deterministic GBDT module
//!
//! Tests cover:
//! - Model loading and validation
//! - Deterministic prediction consistency
//! - IPPAN Time normalization
//! - Hash certificate generation
//! - Cross-platform determinism

use ippan_ai_core::deterministic_gbdt::{
    compute_scores, create_test_model, normalize_features, DecisionNode, DeterministicGBDT,
    DeterministicGBDTError, GBDTTree, ValidatorFeatures,
};
use ippan_ai_core::fixed::Fixed;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn fp(value: f64) -> Fixed {
    Fixed::from_f64(value)
}

/// Test model loading from JSON file
#[test]
fn test_model_loading_from_json() {
    let temp_dir = TempDir::new().unwrap();
    let model_path = temp_dir.path().join("test_model.json");

    let model = create_test_model();
    model.save_json(&model_path).unwrap();

    let loaded_model = DeterministicGBDT::from_json_file(&model_path).unwrap();
    assert_eq!(model.trees.len(), loaded_model.trees.len());
    assert_eq!(model.learning_rate, loaded_model.learning_rate);
}

/// Test model loading from binary file
#[test]
fn test_model_loading_from_binary() {
    let temp_dir = TempDir::new().unwrap();
    let model_path = temp_dir.path().join("test_model.bin");

    let model = create_test_model();
    model.save_binary(&model_path).unwrap();

    let loaded_model = DeterministicGBDT::from_binary_file(&model_path).unwrap();
    assert_eq!(model.trees.len(), loaded_model.trees.len());
    assert_eq!(model.learning_rate, loaded_model.learning_rate);
}

/// Test deterministic prediction consistency
#[test]
fn test_deterministic_prediction_consistency() {
    let model = create_test_model();
    let features = vec![
        Fixed::from_int(1),
        Fixed::from_int(2),
        Fixed::from_int(3),
        Fixed::from_int(4),
    ];

    let predictions: Vec<Fixed> = (0..100).map(|_| model.predict(&features)).collect();
    for i in 1..predictions.len() {
        assert_eq!(
            predictions[0], predictions[i],
            "Prediction {} differs from first",
            i
        );
    }
}

/// Test IPPAN Time normalization
#[test]
fn test_ippan_time_normalization() {
    let mut telemetry = HashMap::new();
    telemetry.insert("node1".to_string(), (100_000, 1.2, 99.9, 0.42));
    telemetry.insert("node2".to_string(), (100_080, 0.9, 99.8, 0.38));
    telemetry.insert("node3".to_string(), (99_950, 2.1, 98.9, 0.45));

    let ippan_time_median = 100_050;
    let features = normalize_features(&telemetry, ippan_time_median);

    assert_eq!(features.len(), 3);

    let by_id: HashMap<_, _> = features.into_iter().map(|f| (f.node_id.clone(), f)).collect();

    assert_eq!(by_id["node1"].delta_time_us, -50);
    assert_eq!(by_id["node2"].delta_time_us, 30);
    assert_eq!(by_id["node3"].delta_time_us, -100);

    assert_eq!(by_id["node1"].latency_ms, fp(1.2));
    assert_eq!(by_id["node1"].uptime_pct, fp(99.9));
    assert_eq!(by_id["node1"].peer_entropy, fp(0.42));
}

/// Normalization should depend on relative IPPAN time only
#[test]
fn test_normalize_features_clock_offset_invariance() {
    let telemetry_a = HashMap::from([
        ("nodeA".into(), (100_000, 1.2, 99.9, 0.42)),
        ("nodeB".into(), (100_080, 0.9, 99.8, 0.38)),
        ("nodeC".into(), (100_030, 2.1, 98.9, 0.45)),
    ]);
    let telemetry_b = HashMap::from([
        ("nodeA".into(), (105_000, 1.2, 99.9, 0.42)),
        ("nodeB".into(), (105_080, 0.9, 99.8, 0.38)),
        ("nodeC".into(), (105_030, 2.1, 98.9, 0.45)),
    ]);

    let features_a = normalize_features(&telemetry_a, 100_050);
    let features_b = normalize_features(&telemetry_b, 105_050);

    let map = |features: Vec<ValidatorFeatures>| -> HashMap<String, (i64, Fixed, Fixed, Fixed)> {
        features
            .into_iter()
            .map(|f| (f.node_id, (f.delta_time_us, f.latency_ms, f.uptime_pct, f.peer_entropy)))
            .collect()
    };

    assert_eq!(map(features_a), map(features_b));
}

/// Test validator scoring
#[test]
fn test_validator_scoring_scenarios() {
    let model = create_test_model();
    let mut telemetry_good = HashMap::new();
    telemetry_good.insert("good_node".to_string(), (100_000, 0.5, 99.9, 0.8));
    let mut telemetry_poor = HashMap::new();
    telemetry_poor.insert("poor_node".to_string(), (100_000, 5.0, 85.0, 0.2));

    let ippan_time_median = 100_000;
    let round_hash = "test_round_123";

    let features_good = normalize_features(&telemetry_good, ippan_time_median);
    let features_poor = normalize_features(&telemetry_poor, ippan_time_median);

    let scores_good = compute_scores(&model, &features_good, round_hash);
    let scores_poor = compute_scores(&model, &features_poor, round_hash);

    assert_eq!(scores_good.len(), 1);
    assert_eq!(scores_poor.len(), 1);
    assert!(scores_good.contains_key("good_node"));
    assert!(scores_poor.contains_key("poor_node"));
}

/// Test model hash consistency
#[test]
fn test_model_hash_consistency() {
    let model = create_test_model();
    let round_hash = "consistent_round_hash";
    let hashes: Vec<String> = (0..10)
        .map(|_| model.model_hash(round_hash).unwrap())
        .collect();
    for i in 1..hashes.len() {
        assert_eq!(hashes[0], hashes[i], "Hash {} differs from first", i);
    }

    let different_hash = model.model_hash("different_round_hash").unwrap();
    assert_ne!(hashes[0], different_hash);
}

/// Test model validation with invalid structures
#[test]
fn test_model_validation_invalid_structures() {
    let empty_model = DeterministicGBDT {
        trees: vec![],
        learning_rate: fp(0.1),
    };
    assert!(empty_model.validate().is_err());

    let invalid_tree = GBDTTree {
        nodes: vec![
            DecisionNode {
                feature: 0,
                threshold: Fixed::ZERO,
                left: Some(5),
                right: Some(2),
                value: None,
            },
            DecisionNode {
                feature: 0,
                threshold: Fixed::ZERO,
                left: None,
                right: None,
                value: Some(fp(0.1)),
            },
        ],
    };
    let invalid_model = DeterministicGBDT {
        trees: vec![invalid_tree],
        learning_rate: fp(0.1),
    };
    assert!(invalid_model.validate().is_err());
}

/// Test cross-platform determinism
#[test]
fn test_cross_platform_determinism_simulation() {
    let model = create_test_model();
    let features = vec![fp(1.5), fp(2.5), fp(3.5), fp(4.5)];
    let round_hash = "deterministic_round";

    let r1 = model.predict(&features);
    let r2 = model.predict(&features);
    let r3 = model.predict(&features);
    assert_eq!(r1, r2);
    assert_eq!(r2, r3);

    let h1 = model.model_hash(round_hash).unwrap();
    let h2 = model.model_hash(round_hash).unwrap();
    assert_eq!(h1, h2);
}

/// Test serialization round-trip
#[test]
fn test_model_serialization_round_trip() {
    let model = create_test_model();
    let temp_dir = TempDir::new().unwrap();

    let json_path = temp_dir.path().join("model.json");
    model.save_json(&json_path).unwrap();
    let loaded = DeterministicGBDT::from_json_file(&json_path).unwrap();

    let features = vec![
        Fixed::from_int(1),
        Fixed::from_int(2),
        Fixed::from_int(3),
        Fixed::from_int(4),
    ];
    assert_eq!(model.predict(&features), loaded.predict(&features));

    let bin_path = temp_dir.path().join("model.bin");
    model.save_binary(&bin_path).unwrap();
    let loaded_bin = DeterministicGBDT::from_binary_file(&bin_path).unwrap();
    assert_eq!(model.predict(&features), loaded_bin.predict(&features));
}

/// Golden hash verification on x86_64
#[test]
fn test_deterministic_golden_model_hash_matches_reference_on_x86_64() {
    if std::env::consts::ARCH != "x86_64" {
        println!("Skipping golden hash test on {}", std::env::consts::ARCH);
        return;
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let model_path = manifest_dir.join("../../models/deterministic_gbdt_model.json");
    let golden_path = manifest_dir.join("../../models/deterministic_gbdt_model.x86_64.sha256");

    let model =
        DeterministicGBDT::from_json_file(&model_path).expect("Failed to load deterministic model");
    let canonical_json = model.to_canonical_json().unwrap();

    let mut hasher = Sha256::new();
    hasher.update(canonical_json.as_bytes());
    let computed_hash = format!("{:x}", hasher.finalize());

    let golden_hash = fs::read_to_string(&golden_path)
        .expect("Missing golden hash file")
        .trim()
        .to_string();

    println!("Computed model hash: {}", computed_hash);
    println!("Golden hash        : {}", golden_hash);

    if computed_hash != golden_hash {
        if std::env::var("IPPAN_UPDATE_GOLDEN_HASH").as_deref() == Ok("1") {
            fs::write(&golden_path, format!("{}\n", computed_hash))
                .expect("Failed to update golden hash");
            println!("Updated golden hash file.");
        } else {
            panic!(
                "Deterministic model hash mismatch. Expected {}, got {}. 
                Re-run with IPPAN_UPDATE_GOLDEN_HASH=1 to refresh the reference.",
                golden_hash, computed_hash
            );
        }
    }
}
