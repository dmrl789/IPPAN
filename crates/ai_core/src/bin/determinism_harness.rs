//! D-GBDT Determinism Harness
//!
//! This binary validates that D-GBDT inference is deterministic across:
//! - Multiple runs on the same architecture
//! - Different CPU architectures (x86_64, aarch64)
//! - Different operating systems (Linux, macOS, Windows)
//!
//! ## Usage
//!
//! ```bash
//! cargo run --bin determinism_harness -- --model models/deterministic_gbdt_model.json
//! ```
//!
//! ## Output Format
//!
//! The harness outputs:
//! 1. Individual vector results (vector_id, score)
//! 2. Final BLAKE3 digest of all outputs
//! 3. Machine-readable JSON summary
//!
//! ## Golden Vectors
//!
//! The harness uses 50 pre-defined feature vectors covering:
//! - Typical validator metrics (uptime 90-99%, latency 10-500ms)
//! - Edge cases (min/max values, zero stake)
//! - Boundary conditions (reputation thresholds)

use blake3::Hasher;
use ippan_ai_core::gbdt::{model_hash_hex, Model, SCALE};
use ippan_ai_core::DeterministicModel;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "determinism_harness")]
struct Opt {
    /// Path to the D-GBDT model JSON file
    #[structopt(short, long, parse(from_os_str))]
    model: Option<PathBuf>,

    /// Output format: text or json
    #[structopt(short, long, default_value = "text")]
    format: String,
}

/// Golden test vector with ID and features
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GoldenVector {
    id: String,
    description: String,
    features: Vec<i64>,
}

/// Inference result for a golden vector
#[derive(Debug, Clone, Serialize, Deserialize)]
struct InferenceResult {
    vector_id: String,
    score: i64,
}

/// Summary output with digest
#[derive(Debug, Serialize, Deserialize)]
struct DeterminismReport {
    model_hash: String,
    vector_count: usize,
    results: Vec<InferenceResult>,
    final_digest: String,
    architecture: String,
    timestamp: String,
}

fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();

    // Load or create default model
    let model = if let Some(model_path) = opt.model {
        let json = fs::read_to_string(&model_path)?;
        serde_json::from_str(&json)?
    } else {
        create_default_model()
    };

    // Compute model hash
    let model_hash = model_hash_hex(&model)?;

    // Get golden vectors
    let vectors = create_golden_vectors();

    // Run inference on all vectors
    let mut results = Vec::new();
    let mut hasher = Hasher::new();

    for vector in &vectors {
        let score = model.score(&vector.features);
        results.push(InferenceResult {
            vector_id: vector.id.clone(),
            score,
        });

        // Add to digest
        hasher.update(vector.id.as_bytes());
        hasher.update(&score.to_le_bytes());
    }

    let final_digest = hasher.finalize().to_hex().to_string();

    // Create report
    let report = DeterminismReport {
        model_hash,
        vector_count: vectors.len(),
        results,
        final_digest,
        architecture: std::env::consts::ARCH.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    // Output results
    match opt.format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        _ => {
            println!("=== IPPAN D-GBDT Determinism Harness ===");
            println!("Model Hash: {}", report.model_hash);
            println!("Architecture: {}", report.architecture);
            println!("Vector Count: {}", report.vector_count);
            println!("\nResults:");
            for result in &report.results {
                println!("  {} → {}", result.vector_id, result.score);
            }
            println!("\n=== Final Digest ===");
            println!("{}", report.final_digest);
        }
    }

    Ok(())
}

/// Create default test model
fn create_default_model() -> Model {
    use ippan_ai_core::gbdt::{Node, Tree};

    // Simple 2-tree model for testing
    let tree1 = Tree::new(
        vec![
            Node::internal(0, 0, 50 * SCALE, 1, 2),
            Node::leaf(1, 8500 * SCALE), // High reputation
            Node::leaf(2, 5000 * SCALE), // Medium reputation
        ],
        SCALE,
    );

    let tree2 = Tree::new(
        vec![
            Node::internal(0, 1, 100 * SCALE, 1, 2),
            Node::leaf(1, -500 * SCALE), // Penalty for high latency
            Node::leaf(2, 500 * SCALE),  // Bonus for low latency
        ],
        SCALE,
    );

    Model::new(vec![tree1, tree2], 0)
}

/// Create golden test vectors
///
/// These vectors cover:
/// - vec_001-010: Typical high-performance validators
/// - vec_011-020: Medium-performance validators
/// - vec_021-030: Low-performance validators
/// - vec_031-040: Edge cases (max/min values)
/// - vec_041-050: Boundary conditions
fn create_golden_vectors() -> Vec<GoldenVector> {
    vec![
        // High-performance validators (vec_001-010)
        GoldenVector {
            id: "vec_001".to_string(),
            description: "Excellent validator (99% uptime, 10ms latency)".to_string(),
            features: vec![99 * SCALE, 10 * SCALE],
        },
        GoldenVector {
            id: "vec_002".to_string(),
            description: "Excellent validator (98% uptime, 15ms latency)".to_string(),
            features: vec![98 * SCALE, 15 * SCALE],
        },
        GoldenVector {
            id: "vec_003".to_string(),
            description: "Excellent validator (97% uptime, 20ms latency)".to_string(),
            features: vec![97 * SCALE, 20 * SCALE],
        },
        GoldenVector {
            id: "vec_004".to_string(),
            description: "Excellent validator (99.5% uptime, 5ms latency)".to_string(),
            features: vec![9950000, 5 * SCALE], // 99.5 * 1e6 / 10 = 9950000
        },
        GoldenVector {
            id: "vec_005".to_string(),
            description: "Excellent validator (96% uptime, 25ms latency)".to_string(),
            features: vec![96 * SCALE, 25 * SCALE],
        },
        GoldenVector {
            id: "vec_006".to_string(),
            description: "High validator (95% uptime, 30ms latency)".to_string(),
            features: vec![95 * SCALE, 30 * SCALE],
        },
        GoldenVector {
            id: "vec_007".to_string(),
            description: "High validator (94% uptime, 35ms latency)".to_string(),
            features: vec![94 * SCALE, 35 * SCALE],
        },
        GoldenVector {
            id: "vec_008".to_string(),
            description: "High validator (93% uptime, 40ms latency)".to_string(),
            features: vec![93 * SCALE, 40 * SCALE],
        },
        GoldenVector {
            id: "vec_009".to_string(),
            description: "High validator (92% uptime, 45ms latency)".to_string(),
            features: vec![92 * SCALE, 45 * SCALE],
        },
        GoldenVector {
            id: "vec_010".to_string(),
            description: "High validator (91% uptime, 50ms latency)".to_string(),
            features: vec![91 * SCALE, 50 * SCALE],
        },
        // Medium-performance validators (vec_011-020)
        GoldenVector {
            id: "vec_011".to_string(),
            description: "Medium validator (90% uptime, 60ms latency)".to_string(),
            features: vec![90 * SCALE, 60 * SCALE],
        },
        GoldenVector {
            id: "vec_012".to_string(),
            description: "Medium validator (85% uptime, 80ms latency)".to_string(),
            features: vec![85 * SCALE, 80 * SCALE],
        },
        GoldenVector {
            id: "vec_013".to_string(),
            description: "Medium validator (80% uptime, 100ms latency)".to_string(),
            features: vec![80 * SCALE, 100 * SCALE],
        },
        GoldenVector {
            id: "vec_014".to_string(),
            description: "Medium validator (75% uptime, 120ms latency)".to_string(),
            features: vec![75 * SCALE, 120 * SCALE],
        },
        GoldenVector {
            id: "vec_015".to_string(),
            description: "Medium validator (70% uptime, 150ms latency)".to_string(),
            features: vec![70 * SCALE, 150 * SCALE],
        },
        GoldenVector {
            id: "vec_016".to_string(),
            description: "Medium validator (88% uptime, 70ms latency)".to_string(),
            features: vec![88 * SCALE, 70 * SCALE],
        },
        GoldenVector {
            id: "vec_017".to_string(),
            description: "Medium validator (82% uptime, 90ms latency)".to_string(),
            features: vec![82 * SCALE, 90 * SCALE],
        },
        GoldenVector {
            id: "vec_018".to_string(),
            description: "Medium validator (78% uptime, 110ms latency)".to_string(),
            features: vec![78 * SCALE, 110 * SCALE],
        },
        GoldenVector {
            id: "vec_019".to_string(),
            description: "Medium validator (72% uptime, 130ms latency)".to_string(),
            features: vec![72 * SCALE, 130 * SCALE],
        },
        GoldenVector {
            id: "vec_020".to_string(),
            description: "Medium validator (68% uptime, 140ms latency)".to_string(),
            features: vec![68 * SCALE, 140 * SCALE],
        },
        // Low-performance validators (vec_021-030)
        GoldenVector {
            id: "vec_021".to_string(),
            description: "Low validator (65% uptime, 180ms latency)".to_string(),
            features: vec![65 * SCALE, 180 * SCALE],
        },
        GoldenVector {
            id: "vec_022".to_string(),
            description: "Low validator (60% uptime, 200ms latency)".to_string(),
            features: vec![60 * SCALE, 200 * SCALE],
        },
        GoldenVector {
            id: "vec_023".to_string(),
            description: "Low validator (55% uptime, 250ms latency)".to_string(),
            features: vec![55 * SCALE, 250 * SCALE],
        },
        GoldenVector {
            id: "vec_024".to_string(),
            description: "Low validator (50% uptime, 300ms latency)".to_string(),
            features: vec![50 * SCALE, 300 * SCALE],
        },
        GoldenVector {
            id: "vec_025".to_string(),
            description: "Low validator (45% uptime, 350ms latency)".to_string(),
            features: vec![45 * SCALE, 350 * SCALE],
        },
        GoldenVector {
            id: "vec_026".to_string(),
            description: "Low validator (62% uptime, 190ms latency)".to_string(),
            features: vec![62 * SCALE, 190 * SCALE],
        },
        GoldenVector {
            id: "vec_027".to_string(),
            description: "Low validator (58% uptime, 220ms latency)".to_string(),
            features: vec![58 * SCALE, 220 * SCALE],
        },
        GoldenVector {
            id: "vec_028".to_string(),
            description: "Low validator (52% uptime, 280ms latency)".to_string(),
            features: vec![52 * SCALE, 280 * SCALE],
        },
        GoldenVector {
            id: "vec_029".to_string(),
            description: "Low validator (48% uptime, 320ms latency)".to_string(),
            features: vec![48 * SCALE, 320 * SCALE],
        },
        GoldenVector {
            id: "vec_030".to_string(),
            description: "Low validator (42% uptime, 380ms latency)".to_string(),
            features: vec![42 * SCALE, 380 * SCALE],
        },
        // Edge cases (vec_031-040)
        GoldenVector {
            id: "vec_031".to_string(),
            description: "Edge: 100% uptime, 1ms latency (perfect)".to_string(),
            features: vec![100 * SCALE, 1 * SCALE],
        },
        GoldenVector {
            id: "vec_032".to_string(),
            description: "Edge: 0% uptime, 1000ms latency (worst)".to_string(),
            features: vec![0, 1000 * SCALE],
        },
        GoldenVector {
            id: "vec_033".to_string(),
            description: "Edge: 100% uptime, 500ms latency".to_string(),
            features: vec![100 * SCALE, 500 * SCALE],
        },
        GoldenVector {
            id: "vec_034".to_string(),
            description: "Edge: 1% uptime, 1ms latency".to_string(),
            features: vec![1 * SCALE, 1 * SCALE],
        },
        GoldenVector {
            id: "vec_035".to_string(),
            description: "Edge: 50% uptime, 50ms latency (median)".to_string(),
            features: vec![50 * SCALE, 50 * SCALE],
        },
        GoldenVector {
            id: "vec_036".to_string(),
            description: "Edge: 99.99% uptime, 0ms latency".to_string(),
            features: vec![9999000, 0], // 99.99 * 1e6 / 10 = 9999000
        },
        GoldenVector {
            id: "vec_037".to_string(),
            description: "Edge: Zero features".to_string(),
            features: vec![0, 0],
        },
        GoldenVector {
            id: "vec_038".to_string(),
            description: "Edge: Max SCALE values".to_string(),
            features: vec![100 * SCALE, 1000 * SCALE],
        },
        GoldenVector {
            id: "vec_039".to_string(),
            description: "Edge: Negative (invalid) uptime handled gracefully".to_string(),
            features: vec![-10 * SCALE, 50 * SCALE],
        },
        GoldenVector {
            id: "vec_040".to_string(),
            description: "Edge: Very large latency".to_string(),
            features: vec![90 * SCALE, 10000 * SCALE],
        },
        // Boundary conditions (vec_041-050)
        GoldenVector {
            id: "vec_041".to_string(),
            description: "Boundary: Just above 95% uptime threshold".to_string(),
            features: vec![9500001, 30 * SCALE], // 95.00001%
        },
        GoldenVector {
            id: "vec_042".to_string(),
            description: "Boundary: Just below 95% uptime threshold".to_string(),
            features: vec![9499999, 30 * SCALE], // 94.99999%
        },
        GoldenVector {
            id: "vec_043".to_string(),
            description: "Boundary: Exactly 90% uptime".to_string(),
            features: vec![90 * SCALE, 50 * SCALE],
        },
        GoldenVector {
            id: "vec_044".to_string(),
            description: "Boundary: Exactly 80% uptime".to_string(),
            features: vec![80 * SCALE, 100 * SCALE],
        },
        GoldenVector {
            id: "vec_045".to_string(),
            description: "Boundary: Exactly 70% uptime".to_string(),
            features: vec![70 * SCALE, 150 * SCALE],
        },
        GoldenVector {
            id: "vec_046".to_string(),
            description: "Boundary: Just above 50ms latency threshold".to_string(),
            features: vec![95 * SCALE, 50 * SCALE + 1],
        },
        GoldenVector {
            id: "vec_047".to_string(),
            description: "Boundary: Just below 50ms latency threshold".to_string(),
            features: vec![95 * SCALE, 50 * SCALE - 1],
        },
        GoldenVector {
            id: "vec_048".to_string(),
            description: "Boundary: Exactly 100ms latency".to_string(),
            features: vec![85 * SCALE, 100 * SCALE],
        },
        GoldenVector {
            id: "vec_049".to_string(),
            description: "Boundary: Exactly 200ms latency".to_string(),
            features: vec![75 * SCALE, 200 * SCALE],
        },
        GoldenVector {
            id: "vec_050".to_string(),
            description: "Boundary: Exactly 500ms latency".to_string(),
            features: vec![60 * SCALE, 500 * SCALE],
        },
    ]
}
