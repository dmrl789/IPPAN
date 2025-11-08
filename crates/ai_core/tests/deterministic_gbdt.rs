#![cfg(feature = "deterministic_math")]

//! Tests for deterministic GBDT + IPPAN Time integration
//!
//! Ensures identical inference results across nodes using
//! the same IPPAN Time median and HashTimer anchor.

use ippan_ai_core::deterministic_gbdt::{
    compute_scores, normalize_features, DecisionNode, DeterministicGBDT, GBDTTree,
};
use ippan_ai_core::Fixed;
use std::collections::HashMap;

fn fp(value: f64) -> Fixed {
    Fixed::from_f64(value)
}

/// Build a simple deterministic GBDT model
fn build_simple_model() -> DeterministicGBDT {
    let tree1 = GBDTTree {
        nodes: vec![
            DecisionNode {
                feature: 0,
                threshold: Fixed::ZERO,
                left: Some(1),
                right: Some(2),
                value: None,
            },
            DecisionNode {
                feature: 0,
                threshold: Fixed::ZERO,
                left: None,
                right: None,
                value: Some(fp(1.5)),
            },
            DecisionNode {
                feature: 0,
                threshold: Fixed::ZERO,
                left: None,
                right: None,
                value: Some(fp(-0.5)),
            },
        ],
    };

    let tree2 = GBDTTree {
        nodes: vec![
            DecisionNode {
                feature: 1,
                threshold: Fixed::from_int(1),
                left: Some(1),
                right: Some(2),
                value: None,
            },
            DecisionNode {
                feature: 1,
                threshold: Fixed::ZERO,
                left: None,
                right: None,
                value: Some(fp(0.25)),
            },
            DecisionNode {
                feature: 1,
                threshold: Fixed::ZERO,
                left: None,
                right: None,
                value: Some(fp(0.75)),
            },
        ],
    };

    DeterministicGBDT {
        trees: vec![tree1, tree2],
        learning_rate: fp(0.1),
    }
}

#[test]
fn deterministic_prediction_same_features() {
    let model = build_simple_model();
    let features = vec![Fixed::ZERO, fp(0.5), fp(99.9), fp(0.42)];

    let y1 = model.predict(&features);
    let y2 = model.predict(&features);

    assert_eq!(y1, y2);
}

#[test]
fn normalize_features_clock_offset_cancels_when_median_also_offset() {
    // Simulate two observers with +5000µs clock offset, both using IPPAN median also +5000µs
    let telemetry_a: HashMap<String, (i64, f64, f64, f64)> = HashMap::from([
        ("nodeA".into(), (100_000, 1.2, 99.9, 0.42)),
        ("nodeB".into(), (100_080, 0.9, 99.8, 0.38)),
        ("nodeC".into(), (100_030, 2.1, 98.9, 0.45)),
    ]);
    let median_a = 100_050_i64;

    let telemetry_b: HashMap<String, (i64, f64, f64, f64)> = HashMap::from([
        ("nodeA".into(), (105_000, 1.2, 99.9, 0.42)),
        ("nodeB".into(), (105_080, 0.9, 99.8, 0.38)),
        ("nodeC".into(), (105_030, 2.1, 98.9, 0.45)),
    ]);
    let median_b = 105_050_i64;

    let feats_a = normalize_features(&telemetry_a, median_a);
    let feats_b = normalize_features(&telemetry_b, median_b);

    assert_eq!(feats_a.len(), feats_b.len());

    // Map features by node_id for comparison
    let map_a: HashMap<String, (i64, Fixed, Fixed, Fixed)> = feats_a
        .into_iter()
        .map(|f| {
            (
                f.node_id.clone(),
                (f.delta_time_us, f.latency_ms, f.uptime_pct, f.peer_entropy),
            )
        })
        .collect();

    let map_b: HashMap<String, (i64, Fixed, Fixed, Fixed)> = feats_b
        .into_iter()
        .map(|f| {
            (
                f.node_id.clone(),
                (f.delta_time_us, f.latency_ms, f.uptime_pct, f.peer_entropy),
            )
        })
        .collect();

    assert_eq!(map_a, map_b);
}

#[test]
fn compute_scores_and_certificate_consistency() {
    let model_a = build_simple_model();
    let model_b = build_simple_model();

    let telemetry: HashMap<String, (i64, f64, f64, f64)> = HashMap::from([
        ("nodeA".into(), (100_000, 1.2, 99.9, 0.42)),
        ("nodeB".into(), (100_080, 0.9, 99.8, 0.38)),
        ("nodeC".into(), (100_030, 2.1, 98.9, 0.45)),
    ]);
    let median = 100_050_i64;

    let feats = normalize_features(&telemetry, median);
    let round_hash_timer = "4b2e18f2fa7c_round_example";

    let scores_a = compute_scores(&model_a, &feats, round_hash_timer);
    let scores_b = compute_scores(&model_b, &feats, round_hash_timer);

    assert_eq!(scores_a, scores_b);

    let cert_a = model_a.model_hash(round_hash_timer).unwrap();
    let cert_b = model_b.model_hash(round_hash_timer).unwrap();

    assert_eq!(cert_a, cert_b);
    assert!(!cert_a.is_empty());
}

#[test]
fn cross_node_consensus_scores_identical() {
    // All nodes compute identical scores given same telemetry + HashTimer
    let model = build_simple_model();
    let telemetry: HashMap<String, (i64, f64, f64, f64)> = HashMap::from([
        ("val1".into(), (10_000_050, 1.2, 99.9, 0.95)),
        ("val2".into(), (10_000_100, 2.5, 98.5, 0.85)),
    ]);
    let median = 10_000_000_i64;
    let round_ht = "round_42_hashtimer_abc123";

    let f1 = normalize_features(&telemetry, median);
    let s1 = compute_scores(&model, &f1, round_ht);

    let f2 = normalize_features(&telemetry, median);
    let s2 = compute_scores(&model, &f2, round_ht);

    let f3 = normalize_features(&telemetry, median);
    let s3 = compute_scores(&model, &f3, round_ht);

    assert_eq!(s1, s2);
    assert_eq!(s2, s3);
}

#[test]
fn model_hash_is_reproducible() {
    let model = build_simple_model();
    let round_ht = "hash_42_test";
    let h1 = model.model_hash(round_ht).unwrap();
    let h2 = model.model_hash(round_ht).unwrap();
    assert_eq!(h1, h2);
}

#[test]
fn fixed_point_prediction_stable() {
    let model = build_simple_model();
    let features = vec![
        Fixed::from_int(1),
        Fixed::from_int(2),
        Fixed::from_int(3),
        Fixed::from_int(4),
    ];
    let y1 = model.predict(&features);
    let y2 = model.predict(&features);
    assert_eq!(y1, y2);
}

#[test]
fn normalize_features_produces_expected_dimensions() {
    let median = 5_000_000;
    let telemetry: HashMap<String, (i64, f64, f64, f64)> = HashMap::from([
        ("validator1".into(), (5_000_100, 1.0, 99.5, 0.9)),
        ("validator2".into(), (4_999_900, 2.5, 98.0, 0.7)),
    ]);

    let feats = normalize_features(&telemetry, median);
    assert_eq!(feats.len(), 2);
    for f in feats {
        assert!(f.node_id.starts_with("validator"));
    }
}
