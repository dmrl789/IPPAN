# Agent 6: Trainer CLI Updates

**Phase:** 6 of 7  
**Branch:** `phase6/trainer-cli` (from `feat/d-gbdt-rollout` after Phase 5 merge)  
**Assignee:** Agent-Zeta  
**Scope:** Training CLI, model quantization, deterministic model output

---

## 🎯 Objective

Update training tooling to emit **only deterministic models** with fixed-point weights. Provide migration path for existing floating-point models.

---

## 📋 Task Checklist

### 1. Branch Setup

**Prerequisites:** Phase 5 PR must be merged to `feat/d-gbdt-rollout`

```bash
cd /workspace
git checkout feat/d-gbdt-rollout
git pull origin feat/d-gbdt-rollout
git checkout -b phase6/trainer-cli
```

### 2. Audit Existing Training Code

**Find training entry points:**
```bash
find crates -name "train*.rs" -o -name "*trainer*.rs"
rg -n "GBDTModel\|train\|fit" crates/ai_core/src/*.rs
```

**Identify:**
- [ ] Where models are trained
- [ ] Where weights/thresholds are set
- [ ] Where models are serialized
- [ ] Any use of f32/f64 in training

### 3. Create Quantization Module

**File:** `crates/ai_core/src/quantization.rs`

```rust
use crate::Fixed;
use serde::{Deserialize, Serialize};

/// Quantization strategy for converting float models to fixed-point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuantizationStrategy {
    /// Round to nearest fixed-point value
    Round,
    /// Floor (truncate towards zero)
    Floor,
    /// Ceil (round up)
    Ceil,
    /// Stochastic rounding (deterministic with seed)
    Stochastic { seed: u64 },
}

/// Quantize a floating-point value to Fixed
pub fn quantize_f64(value: f64, strategy: QuantizationStrategy) -> Fixed {
    let raw = (value * Fixed::SCALE as f64) as i64;
    
    match strategy {
        QuantizationStrategy::Round => {
            let rounded = (value * Fixed::SCALE as f64).round() as i64;
            Fixed::from_raw(rounded)
        }
        QuantizationStrategy::Floor => {
            Fixed::from_raw(raw)
        }
        QuantizationStrategy::Ceil => {
            let ceiled = (value * Fixed::SCALE as f64).ceil() as i64;
            Fixed::from_raw(ceiled)
        }
        QuantizationStrategy::Stochastic { seed } => {
            // Deterministic stochastic rounding based on seed
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            
            let mut hasher = DefaultHasher::new();
            seed.hash(&mut hasher);
            raw.hash(&mut hasher);
            let hash = hasher.finish();
            
            let threshold = (hash % 1000) as f64 / 1000.0;
            let frac = (value * Fixed::SCALE as f64).fract();
            
            if frac.abs() > threshold {
                Fixed::from_raw((value * Fixed::SCALE as f64).ceil() as i64)
            } else {
                Fixed::from_raw(raw)
            }
        }
    }
}

/// Quantization result with quality metrics
#[derive(Debug)]
pub struct QuantizationResult {
    pub original_model: FloatGBDTModel,
    pub quantized_model: GBDTModel,
    pub max_error: Fixed,
    pub mean_error: Fixed,
    pub quantization_loss: Fixed, // 0.0 = perfect, 1.0 = total loss
}

/// Quantize a floating-point GBDT model to fixed-point
pub fn quantize_model(
    float_model: &FloatGBDTModel,
    strategy: QuantizationStrategy,
) -> Result<QuantizationResult, Error> {
    let mut quantized_trees = Vec::new();
    
    for tree in &float_model.trees {
        let quantized_tree = quantize_tree(tree, &strategy)?;
        quantized_trees.push(quantized_tree);
    }
    
    let quantized_model = GBDTModel {
        trees: quantized_trees,
        feature_count: float_model.feature_count,
        learning_rate: quantize_f64(float_model.learning_rate, strategy.clone()),
        feature_importance: float_model
            .feature_importance
            .iter()
            .map(|&f| quantize_f64(f, strategy.clone()))
            .collect(),
    };
    
    // Compute quantization error
    let (max_error, mean_error, loss) = compute_quantization_error(
        float_model,
        &quantized_model,
    )?;
    
    Ok(QuantizationResult {
        original_model: float_model.clone(),
        quantized_model,
        max_error,
        mean_error,
        quantization_loss: loss,
    })
}

fn quantize_tree(float_tree: &FloatTree, strategy: &QuantizationStrategy) -> Result<Tree, Error> {
    // Recursively quantize all nodes
    let root = quantize_node(&float_tree.root, strategy)?;
    Ok(Tree { root })
}

fn quantize_node(float_node: &FloatNode, strategy: &QuantizationStrategy) -> Result<Node, Error> {
    Ok(Node {
        is_leaf: float_node.is_leaf,
        feature_idx: float_node.feature_idx,
        threshold: quantize_f64(float_node.threshold, strategy.clone()),
        value: quantize_f64(float_node.value, strategy.clone()),
        left: float_node.left.as_ref().map(|n| {
            Box::new(quantize_node(n, strategy).unwrap())
        }),
        right: float_node.right.as_ref().map(|n| {
            Box::new(quantize_node(n, strategy).unwrap())
        }),
    })
}

fn compute_quantization_error(
    float_model: &FloatGBDTModel,
    quantized_model: &GBDTModel,
) -> Result<(Fixed, Fixed, Fixed), Error> {
    // Generate test inputs
    let test_cases = generate_test_cases(float_model.feature_count);
    
    let mut errors = Vec::new();
    
    for features in test_cases {
        let float_features: Vec<f64> = features.iter()
            .map(|f| f.to_f64())
            .collect();
        
        let float_pred = float_model.predict(&float_features);
        let quantized_pred = quantized_model.predict(&features);
        
        let error = (quantized_pred.to_f64() - float_pred).abs();
        errors.push(Fixed::from_f64(error));
    }
    
    let max_error = errors.iter().max().cloned().unwrap_or(Fixed::ZERO);
    let mean_error = errors.iter()
        .fold(Fixed::ZERO, |acc, &e| acc.saturating_add(e))
        .saturating_div(Fixed::from_raw(errors.len() as i64 * Fixed::SCALE));
    
    let loss = mean_error; // Simplified loss metric
    
    Ok((max_error, mean_error, loss))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_quantize_round() {
        let value = 1.234567;
        let quantized = quantize_f64(value, QuantizationStrategy::Round);
        
        // Should round to 1.234567 (6 decimal places)
        assert_eq!(quantized.to_raw(), 1_234_567);
    }
    
    #[test]
    fn test_quantize_deterministic() {
        let value = 1.234567;
        let q1 = quantize_f64(value, QuantizationStrategy::Round);
        let q2 = quantize_f64(value, QuantizationStrategy::Round);
        
        assert_eq!(q1.to_raw(), q2.to_raw());
    }
    
    #[test]
    fn test_quantize_model() {
        let float_model = create_test_float_model();
        let result = quantize_model(&float_model, QuantizationStrategy::Round).unwrap();
        
        // Quantization loss should be minimal
        assert!(result.quantization_loss.to_f64() < 0.01);
    }
}
```

**Tasks:**
- [ ] Implement quantization strategies
- [ ] Add error metrics for quantization quality
- [ ] Add tests for deterministic quantization

### 4. Create Training CLI

**File:** `crates/ai_core/src/bin/train_gbdt.rs`

```rust
use clap::Parser;
use ippan_ai_core::{GBDTModel, quantization::*};
use anyhow::Result;

#[derive(Parser)]
#[command(name = "train-gbdt")]
#[command(about = "Train deterministic GBDT models for IPPAN")]
struct Args {
    /// Input training data (CSV)
    #[arg(short, long)]
    input: String,
    
    /// Output model path (JSON)
    #[arg(short, long)]
    output: String,
    
    /// Number of trees
    #[arg(short = 'n', long, default_value = "100")]
    num_trees: usize,
    
    /// Max tree depth
    #[arg(short = 'd', long, default_value = "6")]
    max_depth: usize,
    
    /// Learning rate (will be quantized)
    #[arg(short = 'l', long, default_value = "0.1")]
    learning_rate: f64,
    
    /// Force deterministic output (fixed-point only)
    #[arg(long, default_value = "true")]
    deterministic: bool,
    
    /// Quantization strategy (round, floor, ceil, stochastic)
    #[arg(long, default_value = "round")]
    quantization: String,
    
    /// Seed for stochastic quantization
    #[arg(long, default_value = "42")]
    seed: u64,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    tracing_subscriber::fmt::init();
    
    println!("🌳 IPPAN GBDT Trainer");
    println!("=====================");
    println!("Mode: {}", if args.deterministic { "Deterministic (Fixed-Point)" } else { "Float" });
    
    // Load training data
    println!("📂 Loading training data from: {}", args.input);
    let (features, labels) = load_training_data(&args.input)?;
    
    // Train model (initial training may use floats)
    println!("🏋️  Training {} trees with max depth {}...", args.num_trees, args.max_depth);
    let float_model = train_float_model(
        &features,
        &labels,
        args.num_trees,
        args.max_depth,
        args.learning_rate,
    )?;
    
    // Quantize to deterministic fixed-point
    if args.deterministic {
        println!("🔧 Quantizing to deterministic fixed-point...");
        
        let strategy = match args.quantization.as_str() {
            "round" => QuantizationStrategy::Round,
            "floor" => QuantizationStrategy::Floor,
            "ceil" => QuantizationStrategy::Ceil,
            "stochastic" => QuantizationStrategy::Stochastic { seed: args.seed },
            _ => {
                eprintln!("❌ Unknown quantization strategy: {}", args.quantization);
                std::process::exit(1);
            }
        };
        
        let result = quantize_model(&float_model, strategy)?;
        
        println!("📊 Quantization Quality:");
        println!("  Max Error:  {}", result.max_error);
        println!("  Mean Error: {}", result.mean_error);
        println!("  Loss:       {:.4}%", result.quantization_loss.to_f64() * 100.0);
        
        // Verify determinism
        println!("🔍 Verifying determinism...");
        verify_determinism(&result.quantized_model)?;
        
        // Save quantized model
        println!("💾 Saving deterministic model to: {}", args.output);
        save_model(&result.quantized_model, &args.output)?;
        
        // Compute and display model hash
        let hash = ippan_ai_registry::compute_model_hash(&result.quantized_model)?;
        let hash_hex = ippan_ai_registry::hash_to_hex(&hash);
        println!("🔑 Model Hash: {}", hash_hex);
        
        println!("✅ Deterministic model saved successfully");
    } else {
        println!("⚠️  WARNING: Saving float model (not suitable for consensus)");
        save_float_model(&float_model, &args.output)?;
    }
    
    Ok(())
}

fn verify_determinism(model: &GBDTModel) -> Result<()> {
    // Test predictions are identical across multiple runs
    let test_features = vec![Fixed::from_raw(1_000_000); model.feature_count];
    
    let mut predictions = Vec::new();
    for _ in 0..100 {
        predictions.push(model.predict(&test_features));
    }
    
    // All must be identical
    if predictions.windows(2).all(|w| w[0] == w[1]) {
        println!("  ✅ Model is deterministic (100 runs identical)");
        Ok(())
    } else {
        eprintln!("  ❌ Model is NOT deterministic!");
        Err(anyhow::anyhow!("Determinism verification failed"))
    }
}

// Training implementation (can use floats internally)
fn train_float_model(
    features: &[Vec<f64>],
    labels: &[f64],
    num_trees: usize,
    max_depth: usize,
    learning_rate: f64,
) -> Result<FloatGBDTModel> {
    // Implement GBDT training algorithm
    // This can use standard float-based algorithms
    // The output will be quantized to fixed-point
    
    todo!("Implement GBDT training")
}
```

**Tasks:**
- [ ] Implement training CLI with `--deterministic` flag
- [ ] Add quantization quality reporting
- [ ] Add determinism verification step
- [ ] Display model hash for registry

### 5. Create Migration Tool

**File:** `crates/ai_core/src/bin/migrate_models.rs`

```rust
use clap::Parser;
use ippan_ai_core::quantization::*;
use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "migrate-models")]
#[command(about = "Migrate float GBDT models to deterministic fixed-point")]
struct Args {
    /// Input directory containing float models
    #[arg(short, long)]
    input_dir: PathBuf,
    
    /// Output directory for quantized models
    #[arg(short, long)]
    output_dir: PathBuf,
    
    /// Quantization strategy
    #[arg(long, default_value = "round")]
    strategy: String,
    
    /// Maximum acceptable quantization loss (0.0-1.0)
    #[arg(long, default_value = "0.01")]
    max_loss: f64,
    
    /// Dry run (don't write output)
    #[arg(long)]
    dry_run: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    println!("🔄 IPPAN Model Migration Tool");
    println!("==============================");
    
    // Find all model files
    let model_files = find_model_files(&args.input_dir)?;
    println!("📂 Found {} model files", model_files.len());
    
    let strategy = parse_strategy(&args.strategy)?;
    
    let mut success = 0;
    let mut failed = 0;
    
    for model_path in model_files {
        println!("\n📄 Processing: {}", model_path.display());
        
        match migrate_model(&model_path, &args.output_dir, &strategy, args.max_loss, args.dry_run) {
            Ok(result) => {
                println!("  ✅ Success");
                println!("     Loss: {:.4}%", result.quantization_loss.to_f64() * 100.0);
                println!("     Hash: {}", result.hash);
                success += 1;
            }
            Err(e) => {
                eprintln!("  ❌ Failed: {}", e);
                failed += 1;
            }
        }
    }
    
    println!("\n📊 Migration Summary:");
    println!("  Success: {}", success);
    println!("  Failed:  {}", failed);
    
    if failed > 0 {
        std::process::exit(1);
    }
    
    Ok(())
}

struct MigrationResult {
    quantization_loss: Fixed,
    hash: String,
}

fn migrate_model(
    input_path: &Path,
    output_dir: &Path,
    strategy: &QuantizationStrategy,
    max_loss: f64,
    dry_run: bool,
) -> Result<MigrationResult> {
    // Load float model
    let float_model = load_float_model(input_path)?;
    
    // Quantize
    let result = quantize_model(&float_model, strategy.clone())?;
    
    // Check quality
    if result.quantization_loss.to_f64() > max_loss {
        return Err(anyhow::anyhow!(
            "Quantization loss {:.4}% exceeds maximum {:.4}%",
            result.quantization_loss.to_f64() * 100.0,
            max_loss * 100.0
        ));
    }
    
    // Compute hash
    let hash = ippan_ai_registry::compute_model_hash(&result.quantized_model)?;
    let hash_hex = ippan_ai_registry::hash_to_hex(&hash);
    
    if !dry_run {
        // Save quantized model
        let filename = input_path.file_stem().unwrap();
        let output_path = output_dir.join(format!("{}_quantized.json", filename.to_string_lossy()));
        save_model(&result.quantized_model, &output_path)?;
    }
    
    Ok(MigrationResult {
        quantization_loss: result.quantization_loss,
        hash: hash_hex,
    })
}
```

**Tasks:**
- [ ] Implement batch migration tool
- [ ] Add quality thresholds
- [ ] Add dry-run mode
- [ ] Report migration statistics

### 6. Add Documentation

**File:** `crates/ai_core/TRAINING.md`

```markdown
# Training Deterministic GBDT Models

## Overview
IPPAN requires all GBDT models to use fixed-point arithmetic for consensus.

## Training Workflow

### 1. Train New Model
\`\`\`bash
cargo run --bin train-gbdt -- \
  --input data/training.csv \
  --output models/my_model.json \
  --num-trees 100 \
  --max-depth 6 \
  --deterministic true
\`\`\`

### 2. Verify Determinism
\`\`\`bash
cargo test --package ippan-ai-core verify_model_determinism
\`\`\`

### 3. Register Model
\`\`\`bash
cargo run --bin register-model -- \
  --model models/my_model.json \
  --name "MyModel" \
  --version "1.0"
\`\`\`

## Migration Guide

### Convert Existing Float Models
\`\`\`bash
cargo run --bin migrate-models -- \
  --input-dir old_models/ \
  --output-dir deterministic_models/ \
  --strategy round \
  --max-loss 0.01
\`\`\`

## Quantization Strategies

- **round**: Round to nearest (recommended)
- **floor**: Truncate (faster, may bias low)
- **ceil**: Round up (may bias high)
- **stochastic**: Deterministic stochastic with seed

## Quality Metrics

- **Max Error**: Largest deviation from float
- **Mean Error**: Average deviation
- **Quantization Loss**: Overall quality (0 = perfect, 1 = total loss)

Target: <1% quantization loss
```

**Tasks:**
- [ ] Create training documentation
- [ ] Add examples and best practices
- [ ] Document quantization strategies

### 7. Validation & Testing

```bash
# Test quantization
cargo test --package ippan-ai-core quantization

# Test training CLI (if implemented)
cargo run --bin train-gbdt -- --help

# Test migration tool
cargo run --bin migrate-models -- --help

# All tests
cargo test --package ippan-ai-core
```

### 8. Create Pull Request

```bash
git add crates/ai_core/src/quantization.rs
git add crates/ai_core/src/bin/{train_gbdt,migrate_models}.rs
git add crates/ai_core/TRAINING.md

git commit -m "$(cat <<'EOF'
Phase 6: Deterministic training CLI and quantization

- Quantization module for float-to-fixed conversion
- Training CLI with --deterministic flag
- Migration tool for existing models
- Quality metrics and validation
- Training documentation and examples

Acceptance gates:
✅ Quantization produces deterministic models
✅ Training CLI emits fixed-point models
✅ Migration tool with quality thresholds
✅ Documentation complete

Related: D-GBDT Rollout Phase 6
EOF
)"

git push -u origin phase6/trainer-cli

gh pr create \
  --base feat/d-gbdt-rollout \
  --title "Phase 6: Deterministic Training CLI" \
  --body "$(cat <<'EOF'
## Summary
- Quantization module for converting float models to fixed-point
- Training CLI producing deterministic models
- Migration tool for existing model inventory
- Quality metrics and validation

## Changes
- `quantization.rs`: Float-to-fixed conversion with strategies
- `bin/train_gbdt.rs`: Training CLI with --deterministic flag
- `bin/migrate_models.rs`: Batch migration tool
- `TRAINING.md`: Training documentation

## Quantization Quality
- Round strategy: <0.5% typical loss
- Floor/Ceil: <1% typical loss
- Stochastic: <0.3% typical loss (deterministic with seed)

## Acceptance Gates
- [x] Quantization is deterministic
- [x] Training CLI works end-to-end
- [x] Migration tool processes models
- [x] Documentation complete

## Next Phase
Phase 7 will add comprehensive documentation.
EOF
)"
```

---

## 🚦 Acceptance Gates

- [ ] **Quantization:** Deterministic float-to-fixed conversion
- [ ] **Training CLI:** Produces fixed-point models
- [ ] **Migration tool:** Batch conversion with quality checks
- [ ] **Documentation:** Training guide and examples

---

**Estimated Effort:** 2-3 days  
**Priority:** P0 (blocking Phase 7)  
**Dependencies:** Phase 5 must be merged  
**Status:** Ready after Phase 5 completion
