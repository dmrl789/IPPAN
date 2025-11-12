#![cfg(feature = "deterministic_math")]

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
    GBDTTree, ValidatorFeatures,
};
use ippan_ai_core::Fixed;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn fp(value: &str) -> Fixed {
    Fixed::from_decimal_str(value).expect("valid decimal string")
}

type TelemetryMap = HashMap<String, (i64, Fixed, Fixed, Fixed)>;

fn telemetry_entry(
    time_us: i64,
    latency_ms: &str,
    uptime_pct: &str,
    entropy: &str,
) -> (i64, Fixed, Fixed, Fixed) {
    (time_us, fp(latency_ms), fp(uptime_pct), fp(entropy))
}

const EXPECTED_CANONICAL_TEST_MODEL_JSON: &str = r#"{
  "learning_rate": 100000,
  "trees": [
    {
      "nodes": [
        {
          "feature": 0,
          "left": 1,
          "right": 2,
          "threshold": 0,
          "value": null
        },
        {
          "feature": 0,
          "left": null,
          "right": null,
          "threshold": 0,
          "value": 100000
        },
        {
          "feature": 0,
          "left": null,
          "right": null,
          "threshold": 0,
          "value": -50000
        }
      ]
    }
  ]
}"#;

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
    let mut telemetry: TelemetryMap = HashMap::new();
    telemetry.insert(
        "node1".to_string(),
        telemetry_entry(100_000_i64, "1.2", "99.9", "0.42"),
    );
    telemetry.insert(
        "node2".to_string(),
        telemetry_entry(100_080_i64, "0.9", "99.8", "0.38"),
    );
    telemetry.insert(
        "node3".to_string(),
        telemetry_entry(99_950_i64, "2.1", "98.9", "0.45"),
    );

    let ippan_time_median = 100_050;
    let features = normalize_features(&telemetry, ippan_time_median);

    assert_eq!(features.len(), 3);

    let by_id: HashMap<_, _> = features
        .into_iter()
        .map(|f| (f.node_id.clone(), f))
        .collect();

    assert_eq!(by_id["node1"].delta_time_us, -50);
    assert_eq!(by_id["node2"].delta_time_us, 30);
    assert_eq!(by_id["node3"].delta_time_us, -100);

    assert_eq!(by_id["node1"].latency_ms, fp("1.2"));
    assert_eq!(by_id["node1"].uptime_pct, fp("99.9"));
    assert_eq!(by_id["node1"].peer_entropy, fp("0.42"));
}

/// Normalization should depend on relative IPPAN time only
#[test]
fn test_normalize_features_clock_offset_invariance() {
    let telemetry_a: TelemetryMap = HashMap::from([
        (
            "nodeA".into(),
            telemetry_entry(100_000_i64, "1.2", "99.9", "0.42"),
        ),
        (
            "nodeB".into(),
            telemetry_entry(100_080_i64, "0.9", "99.8", "0.38"),
        ),
        (
            "nodeC".into(),
            telemetry_entry(100_030_i64, "2.1", "98.9", "0.45"),
        ),
    ]);
    let telemetry_b: TelemetryMap = HashMap::from([
        (
            "nodeA".into(),
            telemetry_entry(105_000_i64, "1.2", "99.9", "0.42"),
        ),
        (
            "nodeB".into(),
            telemetry_entry(105_080_i64, "0.9", "99.8", "0.38"),
        ),
        (
            "nodeC".into(),
            telemetry_entry(105_030_i64, "2.1", "98.9", "0.45"),
        ),
    ]);

    let features_a = normalize_features(&telemetry_a, 100_050);
    let features_b = normalize_features(&telemetry_b, 105_050);

    let map = |features: Vec<ValidatorFeatures>| -> TelemetryMap {
        features
            .into_iter()
            .map(|f| {
                (
                    f.node_id,
                    (f.delta_time_us, f.latency_ms, f.uptime_pct, f.peer_entropy),
                )
            })
            .collect()
    };

    assert_eq!(map(features_a), map(features_b));
}

/// Test validator scoring
#[test]
fn test_validator_scoring_scenarios() {
    let model = create_test_model();
    let mut telemetry_good: TelemetryMap = HashMap::new();
    telemetry_good.insert(
        "good_node".to_string(),
        telemetry_entry(100_000_i64, "0.5", "99.9", "0.8"),
    );
    let mut telemetry_poor: TelemetryMap = HashMap::new();
    telemetry_poor.insert(
        "poor_node".to_string(),
        telemetry_entry(100_000_i64, "5.0", "85.0", "0.2"),
    );

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
        learning_rate: fp("0.1"),
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
                value: Some(fp("0.1")),
            },
        ],
    };
    let invalid_model = DeterministicGBDT {
        trees: vec![invalid_tree],
        learning_rate: fp("0.1"),
    };
    assert!(invalid_model.validate().is_err());
}

/// Test cross-platform determinism
#[test]
fn test_cross_platform_determinism_simulation() {
    let model = create_test_model();
    let features = vec![fp("1.5"), fp("2.5"), fp("3.5"), fp("4.5")];
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

/// Canonical JSON must be stable bit-for-bit for the test model
#[test]
fn test_canonical_json_for_test_model_is_bit_stable() {
    let model = create_test_model();
    let canonical = model.to_canonical_json().unwrap();
    assert_eq!(canonical, EXPECTED_CANONICAL_TEST_MODEL_JSON);

    // Regenerating the JSON should not change any bytes
    let canonical_repeat = model.to_canonical_json().unwrap();
    assert_eq!(canonical, canonical_repeat);
}

/// Saved JSON artifact must match canonical serialization and remain stable across saves
#[test]
fn test_save_json_matches_canonical_bytes() {
    let model = create_test_model();
    let temp_dir = TempDir::new().unwrap();

    let first_path = temp_dir.path().join("model.json");
    model.save_json(&first_path).unwrap();
    let first_bytes = fs::read_to_string(&first_path).unwrap();
    assert_eq!(first_bytes, EXPECTED_CANONICAL_TEST_MODEL_JSON);

    let second_path = temp_dir.path().join("model_again.json");
    model.save_json(&second_path).unwrap();
    let second_bytes = fs::read_to_string(&second_path).unwrap();
    assert_eq!(first_bytes, second_bytes);
}

/// Binary encoding must remain identical across runs and match bincode reference bytes
#[test]
fn test_binary_encoding_is_bit_stable() {
    let model = create_test_model();
    let expected_bytes = bincode::serialize(&model).unwrap();

    let temp_dir = TempDir::new().unwrap();

    let first_path = temp_dir.path().join("model.bin");
    model.save_binary(&first_path).unwrap();
    let first_bytes = fs::read(&first_path).unwrap();
    assert_eq!(first_bytes, expected_bytes);

    let second_path = temp_dir.path().join("model_again.bin");
    model.save_binary(&second_path).unwrap();
    let second_bytes = fs::read(&second_path).unwrap();
    assert_eq!(first_bytes, second_bytes);
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
    let golden_path_aarch64 =
        manifest_dir.join("../../models/deterministic_gbdt_model.aarch64.sha256");

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

    let golden_hash_aarch64 = fs::read_to_string(&golden_path_aarch64)
        .expect("Missing aarch64 golden hash file")
        .trim()
        .to_string();

    println!("Computed model hash : {}", computed_hash);
    println!("Golden hash (x86_64): {}", golden_hash);
    println!("Golden hash (aarch64): {}", golden_hash_aarch64);

    let update_requested = std::env::var("IPPAN_UPDATE_GOLDEN_HASH").as_deref() == Ok("1");
    for (arch, expected, path) in [
        ("x86_64", &golden_hash, &golden_path),
        ("aarch64", &golden_hash_aarch64, &golden_path_aarch64),
    ] {
        if &computed_hash != expected {
            if update_requested {
                fs::write(path, format!("{}\n", computed_hash))
                    .unwrap_or_else(|e| panic!("Failed to update {arch} golden hash: {e}"));
                println!("Updated {arch} golden hash file.");
            } else {
                panic!(
                    "Deterministic model hash mismatch for {arch}. Expected {}, got {}. \
                     Re-run with IPPAN_UPDATE_GOLDEN_HASH=1 to refresh the reference.",
                    expected, computed_hash
                );
            }
        }
    }
}

/// Golden hash verification on aarch64
#[cfg(target_arch = "aarch64")]
#[test]
fn test_deterministic_golden_model_hash_matches_reference_on_aarch64() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let model_path = manifest_dir.join("../../models/deterministic_gbdt_model.json");
    let golden_path = manifest_dir.join("../../models/deterministic_gbdt_model.aarch64.sha256");

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

    assert_eq!(computed_hash, golden_hash);
}
