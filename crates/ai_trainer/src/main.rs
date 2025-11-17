//! IPPAN GBDT Trainer CLI
//!
//! Provides a deterministic offline entrypoint for training consensus models.

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use ippan_ai_core::{canonical_model_json, model_hash_hex};
use ippan_ai_trainer::{train_model_from_csv, TrainingParams};
use std::fs;
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
#[command(name = "ai-trainer")]
#[command(author = "IPPAN Contributors")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Deterministic D-GBDT trainer", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Train a deterministic model from a telemetry dataset
    Train(TrainArgs),
}

#[derive(Args, Debug)]
struct TrainArgs {
    /// Input CSV dataset path
    #[arg(long)]
    dataset: PathBuf,

    /// Output model JSON path
    #[arg(long)]
    out: PathBuf,

    /// Number of boosting trees
    #[arg(long, default_value_t = 32)]
    tree_count: usize,

    /// Maximum tree depth
    #[arg(long, default_value_t = 4)]
    max_depth: usize,

    /// Minimum samples per leaf
    #[arg(long, default_value_t = 8)]
    min_samples_leaf: usize,

    /// Learning rate in micros (100000 = 0.1)
    #[arg(long, default_value_t = 100_000)]
    learning_rate_micro: i64,

    /// Feature quantization step
    #[arg(long, default_value_t = 10_000)]
    quantization_step: i64,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .context("failed to initialize logging")?;

    match cli.command {
        Commands::Train(args) => run_train(args),
    }
}

fn run_train(args: TrainArgs) -> Result<()> {
    info!(dataset = %args.dataset.display(), out = %args.out.display(), starting = true);

    let params = TrainingParams {
        tree_count: args.tree_count,
        max_depth: args.max_depth,
        min_samples_leaf: args.min_samples_leaf,
        learning_rate_micro: args.learning_rate_micro,
        quantization_step: args.quantization_step,
    };

    let model = train_model_from_csv(&args.dataset, params)
        .context("failed to train deterministic model")?;

    let canonical = canonical_model_json(&model).context("failed to canonicalize model")?;
    fs::create_dir_all(
        args.out
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".")),
    )
    .context("failed to create output directory")?;
    fs::write(&args.out, canonical.as_bytes()).context("failed to write model")?;

    let hash = model_hash_hex(&model).context("failed to hash model")?;
    info!(model_hash = %hash);
    println!("model_hash={}", hash);

    Ok(())
}
