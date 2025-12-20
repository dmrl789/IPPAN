use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use ippan_ai_core::{load_model_from_path, model_hash_hex};
use toml::Value;

fn main() -> Result<()> {
    let config_path = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("config/dlc.toml"));

    let (model_path, expected_hash) = parse_model_entry(&config_path)?;
    verify_one(&config_path, &model_path, &expected_hash, "active")?;

    if let Some((shadow_path, shadow_expected)) = parse_shadow_model_entry(&config_path)? {
        verify_one(&config_path, &shadow_path, &shadow_expected, "shadow")?;
    }

    Ok(())
}

fn parse_model_entry(config_path: &Path) -> Result<(PathBuf, String)> {
    let contents = fs::read_to_string(config_path)
        .with_context(|| format!("Unable to read config file at {}", config_path.display()))?;

    let value: Value = toml::from_str(&contents)
        .with_context(|| format!("Failed to parse TOML at {}", config_path.display()))?;

    let model_table = value
        .get("dgbdt")
        .and_then(|section| section.get("model"))
        .and_then(Value::as_table)
        .context("Missing [dgbdt.model] section with path and expected_hash")?;

    let model_path = model_table
        .get("path")
        .and_then(Value::as_str)
        .context("Missing dgbdt.model.path entry")?;

    let expected_hash = model_table
        .get("expected_hash")
        .and_then(Value::as_str)
        .context("Missing dgbdt.model.expected_hash entry")?
        .trim()
        .to_owned();

    Ok((PathBuf::from(model_path), expected_hash))
}

fn parse_shadow_model_entry(config_path: &Path) -> Result<Option<(PathBuf, String)>> {
    let contents = fs::read_to_string(config_path)
        .with_context(|| format!("Unable to read config file at {}", config_path.display()))?;

    let value: Value = toml::from_str(&contents)
        .with_context(|| format!("Failed to parse TOML at {}", config_path.display()))?;

    let Some(dgbdt) = value.get("dgbdt") else {
        return Ok(None);
    };
    let Some(shadow) = dgbdt.get("shadow_model").and_then(Value::as_table) else {
        return Ok(None);
    };

    let model_path = shadow
        .get("path")
        .and_then(Value::as_str)
        .context("Missing dgbdt.shadow_model.path entry")?;

    let expected_hash = shadow
        .get("expected_hash")
        .and_then(Value::as_str)
        .context("Missing dgbdt.shadow_model.expected_hash entry")?
        .trim()
        .to_owned();

    Ok(Some((PathBuf::from(model_path), expected_hash)))
}

fn verify_one(
    config_path: &Path,
    model_path: &Path,
    expected_hash: &str,
    label: &str,
) -> Result<()> {
    let resolved_path = resolve_model_path(config_path, model_path);
    let model = load_model_from_path(&resolved_path).with_context(|| {
        format!(
            "Failed to load {label} model from {} referenced in {}",
            resolved_path.display(),
            config_path.display()
        )
    })?;

    let actual_hash = model_hash_hex(&model)?;

    if !actual_hash.eq_ignore_ascii_case(expected_hash) {
        bail!(
            "{label} model hash mismatch: expected {}, computed {} for {}",
            expected_hash,
            actual_hash,
            resolved_path.display()
        );
    }

    println!(
        "✅ {label} model hash matches expected value {} for {}",
        actual_hash,
        resolved_path.display()
    );
    Ok(())
}

fn resolve_model_path(config_path: &Path, model_path: &Path) -> PathBuf {
    if model_path.is_absolute() {
        return model_path.to_path_buf();
    }

    let base = config_path.parent().unwrap_or_else(|| Path::new("."));
    let candidate = base.join(model_path);
    if candidate.exists() {
        return candidate;
    }

    let cwd_candidate = env::current_dir()
        .map(|cwd| cwd.join(model_path))
        .unwrap_or_else(|_| model_path.to_path_buf());

    if cwd_candidate.exists() {
        return cwd_candidate;
    }

    model_path.to_path_buf()
}
