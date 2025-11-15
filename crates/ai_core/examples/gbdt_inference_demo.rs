//! GBDT Inference Engine Demo
//!
//! This example demonstrates the deterministic GBDT inference engine
//! with zero floating-point operations.

use ippan_ai_core::gbdt::{model::SCALE, Model, Node, Tree};
use std::io::Write;
use tempfile::NamedTempFile;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Deterministic GBDT Inference Engine Demo ===\n");

    // 1. Create a simple 2-tree GBDT model
    println!("1. Creating a 2-tree GBDT model...");
    let tree1 = Tree::new(
        vec![
            Node::internal(0, 0, 50 * SCALE, 1, 2),
            Node::leaf(1, 100 * SCALE),
            Node::leaf(2, 200 * SCALE),
        ],
        SCALE,
    );

    let tree2 = Tree::new(
        vec![
            Node::internal(0, 1, 30 * SCALE, 1, 2),
            Node::leaf(1, -50 * SCALE),
            Node::leaf(2, 50 * SCALE),
        ],
        SCALE,
    );

    let model = Model::new(vec![tree1, tree2], 0);
    println!("   Model created with {} trees", model.num_trees());
    println!("   Scale: {}", model.scale);
    println!("   Version: {}", model.version);

    // 2. Perform inference
    println!("\n2. Performing inference...");
    let test_cases = vec![
        (vec![30 * SCALE, 20 * SCALE], "Test 1: [30, 20]"),
        (vec![60 * SCALE, 40 * SCALE], "Test 2: [60, 40]"),
        (vec![50 * SCALE, 30 * SCALE], "Test 3: [50, 30] (boundary)"),
    ];

    for (features, label) in test_cases {
        let score = model.score(&features);
        println!(
            "   {} -> Score: {} (unscaled: {:.2})",
            label,
            score,
            score as f64 / SCALE as f64
        );
    }

    // 3. Demonstrate determinism
    println!("\n3. Verifying determinism (100 iterations)...");
    let features = vec![30 * SCALE, 20 * SCALE];
    let first_score = model.score(&features);
    let mut all_match = true;

    for _ in 0..100 {
        let score = model.score(&features);
        if score != first_score {
            all_match = false;
            break;
        }
    }

    println!(
        "   All scores match: {} (Score: {})",
        all_match, first_score
    );

    // 4. Canonical JSON serialization
    println!("\n4. Testing canonical JSON serialization...");
    let json = model.to_canonical_json()?;
    println!("   JSON length: {} bytes", json.len());
    println!("   First 100 chars: {}", &json[..100.min(json.len())]);

    // 5. Model hashing
    println!("\n5. Computing model hash...");
    let hash = model.hash_hex()?;
    println!("   Blake3 hash: {}", hash);
    println!("   Hash length: {} chars (32 bytes)", hash.len());

    // Verify hash stability
    let hash2 = model.hash_hex()?;
    println!("   Hash stable: {}", hash == hash2);

    // 6. Save and load model
    println!("\n6. Testing save/load roundtrip...");
    let temp_file = NamedTempFile::new()?;
    let path = temp_file.path();

    model.save_json(path)?;
    println!("   Model saved to: {}", path.display());

    let loaded = Model::load_json(path)?;
    println!("   Model loaded successfully");

    // Verify loaded model matches
    let original_score = model.score(&features);
    let loaded_score = loaded.score(&features);
    println!(
        "   Scores match: {} ({} == {})",
        original_score == loaded_score,
        original_score,
        loaded_score
    );

    let original_hash = model.hash_hex()?;
    let loaded_hash = loaded.hash_hex()?;
    println!("   Hashes match: {}", original_hash == loaded_hash);

    // 7. Demonstrate exact integer arithmetic
    println!("\n7. Exact integer arithmetic demonstration...");
    let simple_tree = Tree::new(
        vec![
            Node::internal(0, 0, 0, 1, 2),
            Node::leaf(1, -1000),
            Node::leaf(2, 2000),
        ],
        SCALE,
    );
    let simple_model = Model::new(vec![simple_tree], 500);

    let neg_score = simple_model.score(&[-100]);
    let pos_score = simple_model.score(&[100]);

    println!("   Input: [-100] -> Score: {} (expected: -500)", neg_score);
    println!("   Input: [100]  -> Score: {} (expected: 2500)", pos_score);
    println!("   Exact match: {}", neg_score == -500 && pos_score == 2500);

    // 8. Model validation
    println!("\n8. Model validation...");
    match model.validate() {
        Ok(_) => println!("   ✓ Model validation passed"),
        Err(e) => println!("   ✗ Model validation failed: {}", e),
    }

    // 9. Create a custom model from JSON
    println!("\n9. Creating model from custom JSON...");
    let json_model = r#"{
        "version": 1,
        "scale": 1000000,
        "trees": [
            {
                "nodes": [
                    {"id":0,"left":1,"right":2,"feature":0,"threshold":50000000,"leaf":null},
                    {"id":1,"left":-1,"right":-1,"feature":-1,"threshold":0,"leaf":100000000},
                    {"id":2,"left":-1,"right":-1,"feature":-1,"threshold":0,"leaf":200000000}
                ],
                "weight": 1000000
            }
        ],
        "bias": 0,
        "post_scale": 1000000
    }"#;

    let custom_model: Model = serde_json::from_str(json_model)?;
    let custom_score = custom_model.score(&[30 * SCALE]);
    println!("   Custom model loaded");
    println!("   Score for [30]: {}", custom_score);

    println!("\n=== Demo Complete ===");
    println!("\nKey Features Demonstrated:");
    println!("  ✓ Zero floating-point operations");
    println!("  ✓ Deterministic inference across all platforms");
    println!("  ✓ Canonical JSON serialization with sorted keys");
    println!("  ✓ Blake3 hashing for model verification");
    println!("  ✓ Save/load roundtrip preservation");
    println!("  ✓ Exact integer arithmetic");
    println!("  ✓ Model validation");

    Ok(())
}
