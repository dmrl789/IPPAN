# Agent 2: Inference Engine Rewrite

**Phase:** 2 of 7  
**Branch:** `phase2/inference-engine` (from `feat/d-gbdt-rollout` after Phase 1 merge)  
**Assignee:** Agent-Alpha  
**Scope:** `crates/ai_core/src/{deterministic_gbdt.rs, gbdt.rs, model.rs}`

---

## 🎯 Objective

Rewrite the GBDT inference engine to use **only fixed-point math** from Phase 1. Ensure prediction pipeline produces bit-identical results across all platforms.

---

## 📋 Task Checklist

### 1. Branch Setup

**Prerequisites:** Phase 1 PR must be merged to `feat/d-gbdt-rollout`

```bash
cd /workspace
git checkout feat/d-gbdt-rollout
git pull origin feat/d-gbdt-rollout
git checkout -b phase2/inference-engine
```

### 2. Audit Current Inference Code

**Primary file:** `crates/ai_core/src/deterministic_gbdt.rs`

**Identify all float operations:**
```bash
rg -n "(f32|f64)" crates/ai_core/src/deterministic_gbdt.rs \
  | grep -v "tests\|test_" | grep -v "//"
```

**Key areas to refactor:**
- [ ] Tree traversal (split threshold comparisons)
- [ ] Leaf value accumulation
- [ ] Feature preprocessing (normalization, scaling)
- [ ] Ensemble aggregation (summing tree predictions)
- [ ] Post-processing (sigmoid, softmax if present)

### 3. Refactor Tree Traversal

**Current (floating-point):**
```rust
fn traverse_tree(tree: &Tree, features: &[f32]) -> f32 {
    let mut node = &tree.root;
    loop {
        if node.is_leaf {
            return node.value;
        }
        let feature_val = features[node.feature_idx];
        node = if feature_val <= node.threshold {
            &node.left
        } else {
            &node.right
        };
    }
}
```

**Target (fixed-point):**
```rust
fn traverse_tree(tree: &Tree, features: &[Fixed]) -> Fixed {
    let mut node = &tree.root;
    loop {
        if node.is_leaf {
            return node.value; // Fixed type
        }
        let feature_val = features[node.feature_idx];
        node = if feature_val <= node.threshold {
            &node.left
        } else {
            &node.right
        };
    }
}
```

**Tasks:**
- [ ] Change all `f32`/`f64` parameters to `Fixed`
- [ ] Update `Node` struct to use `Fixed` for thresholds and values
- [ ] Ensure comparison operators work correctly (Fixed must impl Ord)

### 4. Refactor Ensemble Prediction

**Current:**
```rust
pub fn predict(&self, features: &[f32]) -> f32 {
    let mut sum = 0.0;
    for tree in &self.trees {
        sum += traverse_tree(tree, features);
    }
    sum / self.trees.len() as f32
}
```

**Target:**
```rust
pub fn predict(&self, features: &[Fixed]) -> Fixed {
    let mut sum = Fixed::ZERO;
    for tree in &self.trees {
        sum = sum.saturating_add(traverse_tree(tree, features));
    }
    // Average using fixed-point division
    sum.saturating_div(Fixed::from_raw(self.trees.len() as i64 * Fixed::SCALE))
}
```

**Tasks:**
- [ ] Use `saturating_add` for accumulation
- [ ] Use `saturating_div` for averaging
- [ ] Handle edge case of zero trees
- [ ] Add overflow protection (max tree count)

### 5. Update Model Structures

**File:** `crates/ai_core/src/model.rs`

**Changes needed:**
```rust
#[derive(Serialize, Deserialize)]
pub struct GBDTModel {
    pub trees: Vec<Tree>,
    pub feature_count: usize,
    // REMOVE: pub learning_rate: f32,
    pub learning_rate: Fixed, // ADD
    // REMOVE: pub feature_importance: Vec<f32>,
    pub feature_importance: Vec<Fixed>, // ADD
}

#[derive(Serialize, Deserialize)]
pub struct Node {
    pub is_leaf: bool,
    pub feature_idx: usize,
    // REMOVE: pub threshold: f32,
    pub threshold: Fixed, // ADD
    // REMOVE: pub value: f32,
    pub value: Fixed, // ADD
    pub left: Option<Box<Node>>,
    pub right: Option<Box<Node>>,
}
```

**Tasks:**
- [ ] Update all model structs to use `Fixed`
- [ ] Update serialization tests
- [ ] Ensure backward compatibility (versioning)

### 6. Feature Preprocessing

**If normalization is needed:**
```rust
pub fn normalize_features(features: &[Fixed], stats: &FeatureStats) -> Vec<Fixed> {
    features.iter().zip(stats.means.iter().zip(stats.stds.iter()))
        .map(|(feat, (mean, std))| {
            // (x - mean) / std using fixed-point
            feat.saturating_sub(*mean)
                .saturating_div(*std)
        })
        .collect()
}
```

**Tasks:**
- [ ] Convert normalization to fixed-point
- [ ] Convert any feature engineering to fixed-point
- [ ] Validate preprocessing determinism

### 7. Add Test Vectors

Create `crates/ai_core/tests/inference_determinism.rs`:

```rust
use ippan_ai_core::{Fixed, GBDTModel};

#[test]
fn test_single_tree_prediction() {
    // Manually constructed tree: if x[0] <= 5.0 then 1.0 else 2.0
    let model = GBDTModel {
        trees: vec![Tree {
            root: Node {
                is_leaf: false,
                feature_idx: 0,
                threshold: Fixed::from_raw(5_000_000), // 5.0
                left: Some(Box::new(Node::leaf(Fixed::from_raw(1_000_000)))), // 1.0
                right: Some(Box::new(Node::leaf(Fixed::from_raw(2_000_000)))), // 2.0
                ..Default::default()
            }
        }],
        feature_count: 1,
        learning_rate: Fixed::from_raw(100_000), // 0.1
        feature_importance: vec![Fixed::from_raw(1_000_000)],
    };
    
    // Test input: x[0] = 3.0 (should go left)
    let features = vec![Fixed::from_raw(3_000_000)];
    let prediction = model.predict(&features);
    
    // Must be exactly 1.0 on ALL platforms
    assert_eq!(prediction.to_raw(), 1_000_000);
    
    // Test input: x[0] = 7.0 (should go right)
    let features = vec![Fixed::from_raw(7_000_000)];
    let prediction = model.predict(&features);
    
    // Must be exactly 2.0 on ALL platforms
    assert_eq!(prediction.to_raw(), 2_000_000);
}

#[test]
fn test_ensemble_aggregation() {
    // Three identical trees each returning 1.0
    // Ensemble should return average: 3.0 / 3 = 1.0
    let model = create_ensemble_model(3, Fixed::from_raw(1_000_000));
    let features = vec![Fixed::from_raw(0)];
    let prediction = model.predict(&features);
    
    assert_eq!(prediction.to_raw(), 1_000_000);
}

#[test]
fn test_overflow_protection() {
    // 1000 trees each returning max value
    let model = create_ensemble_model(1000, Fixed::from_raw(i64::MAX / 2000));
    let features = vec![Fixed::from_raw(0)];
    let prediction = model.predict(&features);
    
    // Should saturate, not panic
    assert!(prediction.to_raw() > 0);
}

#[cfg(target_arch = "x86_64")]
#[test]
fn test_x86_64_inference() {
    let hash = run_inference_and_hash();
    assert_eq!(hash, EXPECTED_HASH_X86_64);
}

#[cfg(target_arch = "aarch64")]
#[test]
fn test_aarch64_inference() {
    let hash = run_inference_and_hash();
    assert_eq!(hash, EXPECTED_HASH_AARCH64);
}
```

**Tasks:**
- [ ] Add at least 10 test vectors
- [ ] Cover edge cases (empty features, overflow, underflow)
- [ ] Add architecture-specific validation tests

### 8. Performance Benchmark

Create `crates/ai_core/benches/inference_benchmark.rs`:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_inference(c: &mut Criterion) {
    let model = load_test_model();
    let features = generate_test_features();
    
    c.bench_function("gbdt_inference_fixed", |b| {
        b.iter(|| model.predict(black_box(&features)))
    });
}

criterion_group!(benches, bench_inference);
criterion_main!(benches);
```

**Acceptance:** <5% performance regression vs baseline

### 9. Validation & Testing

```bash
# Float detection (must be empty)
rg -n "(f32|f64)" crates/ai_core/src/deterministic_gbdt.rs \
  crates/ai_core/src/gbdt.rs crates/ai_core/src/model.rs \
  | grep -v "tests\|test_" | grep -v "//"

# Build check
cargo build --package ippan-ai-core --release

# Run inference tests
cargo test --package ippan-ai-core inference_determinism

# Run all tests
cargo test --package ippan-ai-core

# Run benchmarks
cargo bench --package ippan-ai-core inference_benchmark
```

### 10. Create Pull Request

```bash
git add crates/ai_core/src/{deterministic_gbdt,gbdt,model}.rs
git add crates/ai_core/tests/inference_determinism.rs
git add crates/ai_core/benches/inference_benchmark.rs

git commit -m "$(cat <<'EOF'
Phase 2: Fixed-point inference engine

- Rewrite GBDT inference to use only Fixed type from Phase 1
- Refactor tree traversal, ensemble aggregation, preprocessing
- Add comprehensive test vectors for determinism validation
- Performance regression: <2% vs floating-point baseline

Acceptance gates:
✅ Zero floats in inference code
✅ All tests pass with expected outputs
✅ Bit-identical results across x86_64/aarch64

Related: D-GBDT Rollout Phase 2
EOF
)"

git push -u origin phase2/inference-engine

gh pr create \
  --base feat/d-gbdt-rollout \
  --title "Phase 2: Fixed-Point Inference Engine" \
  --body "$(cat <<'EOF'
## Summary
- Rewrote GBDT inference engine to use only fixed-point math
- Tree traversal, ensemble aggregation, and preprocessing now deterministic
- Added 10+ test vectors validating cross-platform determinism

## Changes
- `deterministic_gbdt.rs`: Fixed-point inference implementation
- `model.rs`: Updated structs to use Fixed type
- `gbdt.rs`: Refactored utility functions
- New tests: `tests/inference_determinism.rs`
- New benchmarks: `benches/inference_benchmark.rs`

## Performance
- Benchmark: 1.8% regression vs floating-point (acceptable)
- Throughput: ~95K predictions/sec (vs 97K baseline)

## Acceptance Gates
- [x] Float check: No f32/f64 in inference code
- [x] Tests: All 10 test vectors pass
- [x] Determinism: x86_64 and aarch64 produce identical hashes
- [x] Performance: <5% regression

## Next Phase
Phase 3 will add model registry with canonical serialization.
EOF
)"
```

---

## 🚦 Acceptance Gates

- [ ] **Float check:** No f32/f64 in deterministic_gbdt.rs, gbdt.rs, model.rs
- [ ] **Test vectors:** ≥10 tests with known inputs/outputs
- [ ] **Cross-arch:** x86_64 and aarch64 tests pass with identical results
- [ ] **Performance:** <5% regression in benchmarks
- [ ] **All tests pass:** `cargo test --package ippan-ai-core`

---

**Estimated Effort:** 3-4 days  
**Priority:** P0 (blocking Phase 3)  
**Dependencies:** Phase 1 must be merged  
**Status:** Ready after Phase 1 completion
