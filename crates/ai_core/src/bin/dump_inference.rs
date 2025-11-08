use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use ippan_ai_core::deterministic_gbdt::{compute_scores, normalize_features, DeterministicGBDT};
use serde::Serialize;

#[derive(Serialize)]
struct TelemetryRecord {
    node_id: String,
    local_time_us: i64,
    latency_ms: f64,
    uptime_pct: f64,
    peer_entropy: f64,
}

#[derive(Serialize)]
struct FeatureRecord {
    node_id: String,
    delta_time_us: i64,
    latency_ms: f64,
    uptime_pct: f64,
    peer_entropy: f64,
}

#[derive(Serialize)]
struct ScoreRecord {
    node_id: String,
    score_micro: i64,
    score: f64,
}

#[derive(Serialize)]
struct DeterminismArtifact {
    arch: &'static str,
    round_hash_timer: String,
    model_hash: String,
    telemetry: Vec<TelemetryRecord>,
    features: Vec<FeatureRecord>,
    scores: Vec<ScoreRecord>,
}

fn main() -> Result<()> {
    let output_path = parse_output_path()?;

    let telemetry_samples = sample_telemetry();
    let telemetry_map: HashMap<String, (i64, f64, f64, f64)> = telemetry_samples
        .iter()
        .map(|record| {
            (
                record.node_id.clone(),
                (
                    record.local_time_us,
                    record.latency_ms,
                    record.uptime_pct,
                    record.peer_entropy,
                ),
            )
        })
        .collect();

    let ippan_time_median = 10_000_050_i64;
    let features = normalize_features(&telemetry_map, ippan_time_median);

    let model_path = Path::new("models/reputation_v1.json");
    let model = DeterministicGBDT::from_json_file(model_path)
        .with_context(|| format!("failed to load model from {}", model_path.display()))?;

    let round_hash_timer = "determinism_round_v1";
    let scores = compute_scores(&model, &features, round_hash_timer);
    let model_hash = model
        .model_hash(round_hash_timer)
        .context("failed to compute model hash")?;

    let mut feature_records: Vec<FeatureRecord> = features
        .iter()
        .map(|feature| FeatureRecord {
            node_id: feature.node_id.clone(),
            delta_time_us: feature.delta_time_us,
            latency_ms: feature.latency_ms.to_f64(),
            uptime_pct: feature.uptime_pct.to_f64(),
            peer_entropy: feature.peer_entropy.to_f64(),
        })
        .collect();
    feature_records.sort_by(|a, b| a.node_id.cmp(&b.node_id));

    let mut score_records: Vec<ScoreRecord> = scores
        .into_iter()
        .map(|(node_id, score)| ScoreRecord {
            score_micro: score.to_micro(),
            score: score.to_f64(),
            node_id,
        })
        .collect();
    score_records.sort_by(|a, b| a.node_id.cmp(&b.node_id));

    let mut telemetry_sorted = telemetry_samples;
    telemetry_sorted.sort_by(|a, b| a.node_id.cmp(&b.node_id));

    let artifact = DeterminismArtifact {
        arch: env::consts::ARCH,
        round_hash_timer: round_hash_timer.to_string(),
        model_hash,
        telemetry: telemetry_sorted,
        features: feature_records,
        scores: score_records,
    };

    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {}", parent.display()))?;
        }
    }

    let serialized =
        serde_json::to_string_pretty(&artifact).context("failed to serialize artifact")?;
    fs::write(&output_path, serialized)
        .with_context(|| format!("failed to write artifact to {}", output_path.display()))?;

    println!(
        "Wrote deterministic inference artifact for {} to {}",
        artifact.arch,
        output_path.display()
    );

    Ok(())
}

fn parse_output_path() -> Result<PathBuf> {
    let mut args = env::args().skip(1);
    let mut output = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--output" => {
                let path = args.next().context("expected value after --output")?;
                output = Some(PathBuf::from(path));
            }
            other => bail!("unknown argument: {other}"),
        }
    }

    Ok(output.unwrap_or_else(|| PathBuf::from("determinism-output.json")))
}

fn sample_telemetry() -> Vec<TelemetryRecord> {
    vec![
        TelemetryRecord {
            node_id: "validator-alpha".to_string(),
            local_time_us: 10_000_120,
            latency_ms: 1.18,
            uptime_pct: 99.92,
            peer_entropy: 0.82,
        },
        TelemetryRecord {
            node_id: "validator-beta".to_string(),
            local_time_us: 10_000_070,
            latency_ms: 1.45,
            uptime_pct: 98.75,
            peer_entropy: 0.76,
        },
        TelemetryRecord {
            node_id: "validator-gamma".to_string(),
            local_time_us: 10_000_180,
            latency_ms: 0.97,
            uptime_pct: 99.65,
            peer_entropy: 0.88,
        },
        TelemetryRecord {
            node_id: "validator-delta".to_string(),
            local_time_us: 10_000_010,
            latency_ms: 1.62,
            uptime_pct: 97.95,
            peer_entropy: 0.73,
        },
    ]
}
