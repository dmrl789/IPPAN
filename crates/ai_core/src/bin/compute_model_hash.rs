use std::env;
use std::path::PathBuf;
use anyhow::{Context, Result};
use ippan_ai_core::{load_model_from_path, model_hash_hex};

fn main() -> Result<()> {
    let model_path = env::args()
        .nth(1)
        .map(PathBuf::from)
        .context("Usage: compute_model_hash <model_path>")?;

    let model = load_model_from_path(&model_path)
        .with_context(|| format!("Failed to load model from {}", model_path.display()))?;

    let hash = model_hash_hex(&model)?;
    println!("{}", hash);
    Ok(())
}

