//! IPPAN GBDT Trainer CLI
//!
//! Deterministic offline trainer for producing reproducible GBDT models.

use anyhow::{Context, Result};
use clap::Parser;
use ippan_ai_core::serialization::canonical_json_string;
use ippan_ai_trainer::{Dataset, GbdtConfig, GbdtTrainer};
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
#[command(name = "ippan-train")]
#[command(author = "IPPAN Contributors")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Deterministic GBDT trainer for IPPAN blockchain", long_about = None)]
struct Args {
    /// Input CSV dataset path (integer columns, last column is target)
    #[arg(short, long)]
    input: PathBuf,

    /// Output directory for model and hash
    #[arg(short, long, default_value = "models/gbdt")]
    output: PathBuf,

    /// Number of boosting trees
    #[arg(long, default_value = "64")]
    trees: usize,

    /// Maximum tree depth
    #[arg(long, default_value = "6")]
    max_depth: usize,

    /// Minimum samples per leaf
    #[arg(long, default_value = "32")]
    min_samples_leaf: usize,

    /// Learning rate (fixed-point, e.g., 100000 = 0.1)
    #[arg(long, default_value = "100000")]
    learning_rate: i64,

    /// Quantization step for feature values
    #[arg(long, default_value = "1000")]
    quant_step: i64,

    /// Random seed for deterministic shuffling
    #[arg(long, default_value = "42")]
    seed: i64,

    /// Output scale (default 10000)
    #[arg(long, default_value = "10000")]
    scale: i32,

    /// Skip dataset shuffling
    #[arg(long)]
    no_shuffle: bool,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Setup logging
    let log_level = if args.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .context("Failed to set tracing subscriber")?;

    info!("IPPAN Deterministic GBDT Trainer v{}", env!("CARGO_PKG_VERSION"));
    info!("═══════════════════════════════════════════");

    // Load dataset
    info!("Loading dataset from: {}", args.input.display());
    let mut dataset = Dataset::from_csv(&args.input)
        .context("Failed to load dataset")?;

    info!(
        "Loaded {} samples with {} features",
        dataset.len(),
        dataset.feature_count
    );

    // Shuffle dataset if requested
    if !args.no_shuffle {
        info!("Shuffling dataset with seed: {}", args.seed);
        dataset.shuffle(args.seed);
    }

    // Display feature statistics
    let stats = dataset.feature_stats();
    info!("Feature statistics:");
    for (i, (min, max)) in stats.iter().enumerate() {
        info!("  Feature {}: min={}, max={}", i, min, max);
    }

    // Configure trainer
    let config = GbdtConfig {
        num_trees: args.trees,
        max_depth: args.max_depth,
        min_samples_leaf: args.min_samples_leaf,
        learning_rate: args.learning_rate,
        quant_step: args.quant_step,
        scale: args.scale,
    };

    info!("Training configuration:");
    info!("  Trees: {}", config.num_trees);
    info!("  Max depth: {}", config.max_depth);
    info!("  Min samples per leaf: {}", config.min_samples_leaf);
    info!("  Learning rate: {} (fixed-point)", config.learning_rate);
    info!("  Quantization step: {}", config.quant_step);
    info!("  Scale: {}", config.scale);

    // Train model
    info!("═══════════════════════════════════════════");
    info!("Starting training...");
    let trainer = GbdtTrainer::new(config);
    let model = trainer.train(&dataset)?;

    info!("Training complete!");
    info!("  Bias: {}", model.bias);
    info!("  Trees: {}", model.trees.len());
    info!("  Model hash: {}", model.metadata.model_hash);

    // Create output directory
    std::fs::create_dir_all(&args.output)
        .context("Failed to create output directory")?;

    // Save model as canonical JSON
    let model_path = args.output.join("active.json");
    info!("Saving model to: {}", model_path.display());

    let canonical_json = canonical_json_string(&model)
        .context("Failed to serialize model")?;

    std::fs::write(&model_path, &canonical_json)
        .context("Failed to write model file")?;

    // Calculate and save BLAKE3 hash
    let hash = blake3::hash(canonical_json.as_bytes());
    let hash_hex = hex::encode(hash.as_bytes());

    let hash_path = args.output.join("active.hash");
    info!("Saving hash to: {}", hash_path.display());
    std::fs::write(&hash_path, &hash_hex)
        .context("Failed to write hash file")?;

    info!("═══════════════════════════════════════════");
    info!("✓ Training completed successfully");
    info!("  Model: {}", model_path.display());
    info!("  Hash: {} ({})", hash_path.display(), hash_hex);

    Ok(())
}
