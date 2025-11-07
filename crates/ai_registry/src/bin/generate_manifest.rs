use std::path::PathBuf;

use chrono::{DateTime, NaiveDate, Utc};
use ippan_ai_core::types::{ModelId, ModelMetadata};
use ippan_ai_registry::manifest::{
    write_architecture_hash_files, ModelManifest, ModelManifestEntry, DEFAULT_INFERENCE_SEED,
};

fn main() -> anyhow::Result<()> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let models_dir = repo_root.join("models");
    let manifest_path = models_dir.join("canonical_manifest.json");
    let deterministic_model_path = PathBuf::from("models/deterministic_gbdt_model.json");

    let created_at = to_unix_micros(2025, 1, 1, 0, 0, 0);

    let metadata = ModelMetadata {
        id: ModelId::new("deterministic_gbdt", "1.0.0", ""),
        name: "Deterministic Validator Reputation Model".to_string(),
        version: "1.0.0".to_string(),
        description: "Gradient boosted decision tree used for consensus reputation scoring."
            .to_string(),
        author: "IPPAN Core Team".to_string(),
        license: "Apache-2.0".to_string(),
        tags: vec![
            "deterministic".to_string(),
            "gbdt".to_string(),
            "reputation".to_string(),
        ],
        created_at,
        updated_at: created_at,
        architecture: "deterministic_gbdt".to_string(),
        input_shape: Vec::new(),
        output_shape: vec![1],
        size_bytes: 0,
        parameter_count: 0,
    };

    let entry = ModelManifestEntry::from_deterministic_gbdt(
        metadata,
        &deterministic_model_path,
        &repo_root,
        DEFAULT_INFERENCE_SEED,
    )?;

    let generated_at = Utc::now().to_rfc3339();
    let manifest = ModelManifest::new(1, generated_at, vec![entry.clone()]);
    manifest.write_to_path(&manifest_path)?;

    let hash_targets = vec![
        (
            "x86_64".to_string(),
            PathBuf::from("models/deterministic_gbdt_model.x86_64.sha256"),
        ),
        (
            "aarch64".to_string(),
            PathBuf::from("models/deterministic_gbdt_model.aarch64.sha256"),
        ),
    ];

    write_architecture_hash_files(&entry, &repo_root, &hash_targets)?;

    println!("Canonical manifest written to {}", manifest_path.display());
    Ok(())
}

fn to_unix_micros(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> u64 {
    let naive = NaiveDate::from_ymd_opt(year, month, day)
        .expect("invalid date")
        .and_hms_opt(hour, minute, second)
        .expect("invalid time");
    let datetime = DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc);
    datetime.timestamp_micros() as u64
}
