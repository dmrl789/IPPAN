//! Integration tests for deterministic GBDT trainer
//!
//! Ensures identical models are produced across multiple runs.

use anyhow::Result;
use ippan_ai_core::serialization::canonical_json_string;
use ippan_ai_trainer::{Dataset, GbdtConfig, GbdtTrainer};
use std::io::Write;
use tempfile::NamedTempFile;

/// Create a synthetic dataset for testing
fn create_synthetic_dataset() -> Result<NamedTempFile> {
    let mut file = NamedTempFile::new()?;

    // Simple linear relationship: target = feature1 + feature2
    // All values scaled to fixed-point (multiplied by 1000)
    writeln!(file, "100000,200000,300000")?;
    writeln!(file, "150000,250000,400000")?;
    writeln!(file, "200000,300000,500000")?;
    writeln!(file, "250000,350000,600000")?;
    writeln!(file, "300000,400000,700000")?;
    writeln!(file, "350000,450000,800000")?;
    writeln!(file, "400000,500000,900000")?;
    writeln!(file, "450000,550000,1000000")?;

    file.flush()?;
    Ok(file)
}

#[test]
fn test_deterministic_training() -> Result<()> {
    // Create dataset
    let file = create_synthetic_dataset()?;
    let dataset = Dataset::from_csv(file.path())?;

    // Train first model
    let config = GbdtConfig {
        num_trees: 4,
        max_depth: 3,
        min_samples_leaf: 2,
        learning_rate: 100_000, // 0.1
        quant_step: 50_000,
        scale: 10_000,
    };

    let trainer1 = GbdtTrainer::new(config.clone());
    let model1 = trainer1.train(&dataset)?;

    // Train second model with same config
    let trainer2 = GbdtTrainer::new(config);
    let model2 = trainer2.train(&dataset)?;

    // Models should be identical
    assert_eq!(model1.bias, model2.bias, "Bias should be identical");
    assert_eq!(model1.scale, model2.scale, "Scale should be identical");
    assert_eq!(
        model1.trees.len(),
        model2.trees.len(),
        "Number of trees should be identical"
    );

    // Check each tree
    for (i, (tree1, tree2)) in model1.trees.iter().zip(model2.trees.iter()).enumerate() {
        assert_eq!(
            tree1.nodes.len(),
            tree2.nodes.len(),
            "Tree {} should have same number of nodes",
            i
        );

        for (j, (node1, node2)) in tree1.nodes.iter().zip(tree2.nodes.iter()).enumerate() {
            assert_eq!(
                node1.feature_index, node2.feature_index,
                "Tree {} node {} feature_index should match",
                i, j
            );
            assert_eq!(
                node1.threshold, node2.threshold,
                "Tree {} node {} threshold should match",
                i, j
            );
            assert_eq!(
                node1.left, node2.left,
                "Tree {} node {} left should match",
                i, j
            );
            assert_eq!(
                node1.right, node2.right,
                "Tree {} node {} right should match",
                i, j
            );
            assert_eq!(
                node1.value, node2.value,
                "Tree {} node {} value should match",
                i, j
            );
        }
    }

    Ok(())
}

#[test]
fn test_canonical_json_determinism() -> Result<()> {
    // Create dataset
    let file = create_synthetic_dataset()?;
    let dataset = Dataset::from_csv(file.path())?;

    // Train model
    let config = GbdtConfig {
        num_trees: 2,
        max_depth: 2,
        min_samples_leaf: 2,
        learning_rate: 100_000,
        quant_step: 50_000,
        scale: 10_000,
    };

    let trainer = GbdtTrainer::new(config);
    let model = trainer.train(&dataset)?;

    // Serialize multiple times
    let json1 = canonical_json_string(&model)?;
    let json2 = canonical_json_string(&model)?;

    assert_eq!(json1, json2, "Canonical JSON should be identical");

    // Calculate hashes
    let hash1 = blake3::hash(json1.as_bytes());
    let hash2 = blake3::hash(json2.as_bytes());

    assert_eq!(
        hash1.as_bytes(),
        hash2.as_bytes(),
        "Hashes should be identical"
    );

    Ok(())
}

#[test]
fn test_model_hash_consistency() -> Result<()> {
    // Create dataset
    let file = create_synthetic_dataset()?;
    let dataset = Dataset::from_csv(file.path())?;

    // Train two models with same config
    let config = GbdtConfig {
        num_trees: 3,
        max_depth: 2,
        min_samples_leaf: 2,
        learning_rate: 100_000,
        quant_step: 50_000,
        scale: 10_000,
    };

    let trainer1 = GbdtTrainer::new(config.clone());
    let model1 = trainer1.train(&dataset)?;

    let trainer2 = GbdtTrainer::new(config);
    let model2 = trainer2.train(&dataset)?;

    // Model hashes should match
    assert_eq!(
        model1.metadata.model_hash, model2.metadata.model_hash,
        "Model hashes should be identical"
    );

    Ok(())
}

#[test]
fn test_dataset_shuffle_determinism() -> Result<()> {
    let file = create_synthetic_dataset()?;
    let mut dataset1 = Dataset::from_csv(file.path())?;
    let mut dataset2 = dataset1.clone();

    // Shuffle both with same seed
    dataset1.shuffle(42);
    dataset2.shuffle(42);

    assert_eq!(
        dataset1.features, dataset2.features,
        "Shuffled features should be identical"
    );
    assert_eq!(
        dataset1.targets, dataset2.targets,
        "Shuffled targets should be identical"
    );

    Ok(())
}

#[test]
fn test_small_dataset() -> Result<()> {
    let mut file = NamedTempFile::new()?;
    writeln!(file, "100000,200000,300000")?;
    writeln!(file, "150000,250000,400000")?;
    writeln!(file, "200000,300000,500000")?;
    file.flush()?;

    let dataset = Dataset::from_csv(file.path())?;

    let config = GbdtConfig {
        num_trees: 2,
        max_depth: 2,
        min_samples_leaf: 1,
        learning_rate: 100_000,
        quant_step: 50_000,
        scale: 10_000,
    };

    let trainer = GbdtTrainer::new(config);
    let model = trainer.train(&dataset)?;

    assert_eq!(model.trees.len(), 2);
    assert_eq!(model.metadata.feature_count, 2);

    Ok(())
}

#[test]
fn test_cross_run_determinism() -> Result<()> {
    // This test verifies that running the same training multiple times
    // produces byte-identical JSON output
    let file = create_synthetic_dataset()?;
    let dataset = Dataset::from_csv(file.path())?;

    let config = GbdtConfig {
        num_trees: 4,
        max_depth: 3,
        min_samples_leaf: 2,
        learning_rate: 100_000,
        quant_step: 50_000,
        scale: 10_000,
    };

    let mut json_outputs = Vec::new();

    // Run training 3 times
    for _ in 0..3 {
        let trainer = GbdtTrainer::new(config.clone());
        let model = trainer.train(&dataset)?;
        let json = canonical_json_string(&model)?;
        json_outputs.push(json);
    }

    // All outputs should be identical
    for i in 1..json_outputs.len() {
        assert_eq!(
            json_outputs[0], json_outputs[i],
            "JSON output from run {} should match run 0",
            i
        );
    }

    Ok(())
}
