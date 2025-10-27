use std::collections::HashMap;

use ippan_ai_core::deterministic_gbdt::{
    compute_scores, normalize_features, DecisionNode, DeterministicGBDT, GBDTTree, ValidatorFeatures,
};

fn build_simple_model() -> DeterministicGBDT {
    // Two tiny trees to exercise traversal and accumulation
    let tree1 = GBDTTree {
        nodes: vec![
            // root: if feature[0] <= 0 go left else right
            DecisionNode { feature: 0, threshold: 0.0, left: Some(1), right: Some(2), value: None },
            // left leaf
            DecisionNode { feature: 0, threshold: 0.0, left: None, right: None, value: Some(1.5) },
            // right leaf
            DecisionNode { feature: 0, threshold: 0.0, left: None, right: None, value: Some(-0.5) },
        ],
    };

    let tree2 = GBDTTree {
        nodes: vec![
            // root: if feature[1] <= 1.0 go left else right
            DecisionNode { feature: 1, threshold: 1.0, left: Some(1), right: Some(2), value: None },
            // left leaf
            DecisionNode { feature: 1, threshold: 0.0, left: None, right: None, value: Some(0.25) },
            // right leaf
            DecisionNode { feature: 1, threshold: 0.0, left: None, right: None, value: Some(0.75) },
        ],
    };

    DeterministicGBDT { trees: vec![tree1, tree2], learning_rate: 0.1 }
}

#[test]
fn deterministic_prediction_same_features() {
    let model = build_simple_model();
    let features = vec![0.0_f64, 0.5, 99.9, 0.42];

    let y1 = model.predict(&features);
    let y2 = model.predict(&features);

    assert_eq!(y1, y2);
}

#[test]
fn normalize_features_clock_offset_cancels_when_median_also_offset() {
    // Simulate two observers with a +5000us clock offset, but both use IPPAN median that is also +5000us
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

    // After normalization, features should match exactly
    assert_eq!(feats_a.len(), feats_b.len());

    // Sort by node_id for stable comparison
    let mut map_a: HashMap<String, (i64, f64, f64, f64)> = HashMap::new();
    for f in feats_a {
        map_a.insert(f.node_id.clone(), (f.delta_time_us, f.latency_ms, f.uptime_pct, f.peer_entropy));
    }
    let mut map_b: HashMap<String, (i64, f64, f64, f64)> = HashMap::new();
    for f in feats_b {
        map_b.insert(f.node_id.clone(), (f.delta_time_us, f.latency_ms, f.uptime_pct, f.peer_entropy));
    }

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
    let round_hash_timer = "4b2e18f2fa7c...";

    let scores_a = compute_scores(&model_a, &feats, round_hash_timer);
    let scores_b = compute_scores(&model_b, &feats, round_hash_timer);

    assert_eq!(scores_a, scores_b);

    let cert_a = model_a.model_hash(round_hash_timer);
    let cert_b = model_b.model_hash(round_hash_timer);

    assert_eq!(cert_a, cert_b);
    assert!(!cert_a.is_empty());
}
