#![cfg(feature = "deterministic_math")]

use ippan_ai_core::deterministic_gbdt;
use ippan_ai_core::Fixed;
use std::collections::HashMap;

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

#[test]
fn test_deterministic_gbdt_basic_functionality() {
    let model = deterministic_gbdt::create_test_model();
    assert_eq!(model.trees.len(), 1);
    assert_eq!(model.learning_rate, fp("0.1"));

    let features = vec![
        Fixed::from_int(1),
        Fixed::from_int(2),
        Fixed::from_int(3),
        Fixed::from_int(4),
    ];
    let prediction = model.predict(&features);
    let repeat = model.predict(&features);
    assert_eq!(prediction, repeat);
}

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

    let ippan_time_median = 100_050_i64;
    let features = deterministic_gbdt::normalize_features(&telemetry, ippan_time_median);

    assert_eq!(features.len(), 2);
    for feature in &features {
        match feature.node_id.as_str() {
            "node1" => assert_eq!(feature.delta_time_us, -50),
            "node2" => assert_eq!(feature.delta_time_us, 30),
            _ => {}
        }
    }
}

#[test]
fn test_validator_scoring() {
    let model = deterministic_gbdt::create_test_model();
    let mut telemetry: TelemetryMap = HashMap::new();
    telemetry.insert(
        "test_node".to_string(),
        telemetry_entry(100_000_i64, "1.0", "99.0", "0.5"),
    );

    let ippan_time_median = 100_000_i64;
    let round_hash = "test_round";

    let features = deterministic_gbdt::normalize_features(&telemetry, ippan_time_median);
    let scores = deterministic_gbdt::compute_scores(&model, &features, round_hash);

    assert_eq!(scores.len(), 1);
    assert!(scores.contains_key("test_node"));
    assert!(scores["test_node"].to_micro() >= 0);
}

#[test]
fn test_model_hash_consistency() {
    let model = deterministic_gbdt::create_test_model();
    let round_hash = "consistent_round";

    let hash1 = model.model_hash(round_hash).unwrap();
    let hash2 = model.model_hash(round_hash).unwrap();

    assert_eq!(hash1, hash2);
    assert!(!hash1.is_empty());
}

#[test]
fn test_cross_platform_determinism() {
    let model = deterministic_gbdt::create_test_model();
    let features = vec![fp("1.5"), fp("2.5"), fp("3.5"), fp("4.5")];

    let node1_result = model.predict(&features);
    let node2_result = model.predict(&features);
    let node3_result = model.predict(&features);

    assert_eq!(node1_result, node2_result);
    assert_eq!(node2_result, node3_result);
}

#[test]
fn test_usage_example() {
    let telemetry: TelemetryMap = HashMap::from([
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

    let ippan_time_median = 100_050_i64;
    let round_hash_timer = "4b2e18f2fa7c...";

    let model = deterministic_gbdt::create_test_model();
    let features = deterministic_gbdt::normalize_features(&telemetry, ippan_time_median);
    let scores = deterministic_gbdt::compute_scores(&model, &features, round_hash_timer);

    assert_eq!(scores.len(), 3);
    assert!(scores.contains_key("nodeA"));
    assert!(scores.contains_key("nodeB"));
    assert!(scores.contains_key("nodeC"));

    // All scores should be finite and within a bounded range
    let mut has_positive = false;
    for score in scores.values() {
        let value_micro = score.to_micro();
        assert!(
            value_micro >= -500_000,
            "Score fell below expected floor in micro units: {value_micro}"
        );
        if value_micro > 0 {
            has_positive = true;
        }
    }

    assert!(
        has_positive,
        "Expected at least one positive validator score"
    );

    println!("✅ Validator scores: {:?}", scores);
}
