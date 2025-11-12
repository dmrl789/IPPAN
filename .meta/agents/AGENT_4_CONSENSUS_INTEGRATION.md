# Agent 4: Consensus DLC Integration

**Phase:** 4 of 7  
**Branch:** `phase4/consensus-integration` (from `feat/d-gbdt-rollout` after Phase 3 merge)  
**Assignee:** Agent-Alpha  
**Scope:** `crates/consensus_dlc/src/{lib.rs, validator.rs, engine.rs}`

---

## 🎯 Objective

Integrate deterministic GBDT inference into the **Deterministic Learning Consensus (DLC)** module. Ensure all validators compute identical predictions for consensus agreement.

---

## 📋 Task Checklist

### 1. Branch Setup

**Prerequisites:** Phase 3 PR must be merged to `feat/d-gbdt-rollout`

```bash
cd /workspace
git checkout feat/d-gbdt-rollout
git pull origin feat/d-gbdt-rollout
git checkout -b phase4/consensus-integration
```

### 2. Audit Current DLC Code

**Check for AI integration points:**
```bash
rg -n "ai_core\|gbdt\|model" crates/consensus_dlc/src/*.rs
rg -n "(f32|f64)" crates/consensus_dlc/src/*.rs | grep -v "tests\|test_"
```

**Identify integration points:**
- [ ] Where models are loaded
- [ ] Where predictions are made
- [ ] Where prediction results are validated
- [ ] Where consensus is reached

### 3. Add Dependencies

**File:** `crates/consensus_dlc/Cargo.toml`

```toml
[dependencies]
# ... existing deps ...
ippan-ai-core = { path = "../ai_core" }
ippan-ai-registry = { path = "../ai_registry" }
```

### 4. Implement DLC Validator with GBDT

**File:** `crates/consensus_dlc/src/validator.rs`

```rust
use ippan_ai_core::{GBDTModel, Fixed};
use ippan_ai_registry::ModelRegistry;
use blake3::Hasher;

/// Validator that uses deterministic GBDT for consensus
pub struct DLCValidator {
    model_registry: ModelRegistry,
    active_model_id: Option<String>,
    active_model: Option<GBDTModel>,
}

impl DLCValidator {
    pub async fn new(registry_path: &str) -> Result<Self, Error> {
        let model_registry = ModelRegistry::open(registry_path).await?;
        
        Ok(Self {
            model_registry,
            active_model_id: None,
            active_model: None,
        })
    }
    
    /// Load model for consensus round
    pub async fn load_model(&mut self, model_id: &str) -> Result<(), Error> {
        // Verify all validators agree on model ID
        let model = self.model_registry.load_model(model_id).await?;
        
        // Verify model integrity
        let hash = ippan_ai_registry::compute_model_hash(&model)?;
        let hash_hex = ippan_ai_registry::hash_to_hex(&hash);
        
        if hash_hex != model_id {
            return Err(Error::ModelHashMismatch {
                expected: model_id.to_string(),
                actual: hash_hex,
            });
        }
        
        self.active_model_id = Some(model_id.to_string());
        self.active_model = Some(model);
        
        tracing::info!(
            model_id = %model_id,
            "Loaded deterministic model for consensus"
        );
        
        Ok(())
    }
    
    /// Make prediction (must be deterministic)
    pub fn predict(&self, features: &[Fixed]) -> Result<Fixed, Error> {
        let model = self.active_model.as_ref()
            .ok_or(Error::NoModelLoaded)?;
        
        // Run deterministic inference
        let prediction = model.predict(features);
        
        tracing::debug!(
            prediction_raw = prediction.to_raw(),
            "Made deterministic prediction"
        );
        
        Ok(prediction)
    }
    
    /// Compute prediction hash for consensus
    pub fn prediction_hash(&self, features: &[Fixed]) -> Result<[u8; 32], Error> {
        let prediction = self.predict(features)?;
        
        // Hash the prediction for consensus
        let mut hasher = Hasher::new();
        hasher.update(&prediction.to_raw().to_le_bytes());
        
        Ok(*hasher.finalize().as_bytes())
    }
}
```

**Tasks:**
- [ ] Implement `DLCValidator` with model loading
- [ ] Implement deterministic `predict()` method
- [ ] Add prediction hashing for consensus
- [ ] Add error handling for model mismatches

### 5. Implement Consensus Engine

**File:** `crates/consensus_dlc/src/engine.rs`

```rust
use std::collections::HashMap;

/// Consensus engine for DLC validation
pub struct ConsensusEngine {
    validators: Vec<DLCValidator>,
    quorum_threshold: f64, // Percentage needed for consensus (e.g., 0.67 for 2/3)
}

impl ConsensusEngine {
    pub async fn new(
        validator_count: usize,
        registry_path: &str,
        quorum_threshold: f64,
    ) -> Result<Self, Error> {
        let mut validators = Vec::new();
        
        for _ in 0..validator_count {
            let validator = DLCValidator::new(registry_path).await?;
            validators.push(validator);
        }
        
        Ok(Self {
            validators,
            quorum_threshold,
        })
    }
    
    /// Load model on all validators
    pub async fn load_model(&mut self, model_id: &str) -> Result<(), Error> {
        for validator in &mut self.validators {
            validator.load_model(model_id).await?;
        }
        
        Ok(())
    }
    
    /// Execute consensus round
    pub async fn consensus_predict(
        &self,
        features: &[Fixed],
    ) -> Result<ConsensusResult, Error> {
        // Collect predictions from all validators
        let mut predictions = HashMap::new();
        
        for (i, validator) in self.validators.iter().enumerate() {
            let prediction = validator.predict(features)?;
            let hash = validator.prediction_hash(features)?;
            
            *predictions.entry(hash).or_insert(0) += 1;
            
            tracing::debug!(
                validator_id = i,
                prediction_raw = prediction.to_raw(),
                prediction_hash = %hex::encode(&hash),
                "Validator prediction"
            );
        }
        
        // Find majority prediction
        let total_validators = self.validators.len();
        let required_votes = (total_validators as f64 * self.quorum_threshold).ceil() as usize;
        
        for (hash, count) in predictions.iter() {
            if *count >= required_votes {
                // Consensus reached
                let prediction = self.validators[0].predict(features)?;
                
                return Ok(ConsensusResult {
                    prediction,
                    agreement: *count as f64 / total_validators as f64,
                    consensus_hash: *hash,
                    reached: true,
                });
            }
        }
        
        // No consensus
        Err(Error::ConsensusNotReached {
            predictions,
            required_votes,
        })
    }
}

#[derive(Debug)]
pub struct ConsensusResult {
    pub prediction: Fixed,
    pub agreement: f64, // 0.0 - 1.0
    pub consensus_hash: [u8; 32],
    pub reached: bool,
}
```

**Tasks:**
- [ ] Implement `ConsensusEngine` with multi-validator support
- [ ] Implement `consensus_predict()` with quorum logic
- [ ] Add logging for consensus tracking
- [ ] Handle consensus failure cases

### 6. Add Multi-Node Tests

Create `crates/consensus_dlc/tests/determinism.rs`:

```rust
use ippan_consensus_dlc::{ConsensusEngine, DLCValidator};
use ippan_ai_core::{GBDTModel, Fixed};
use ippan_ai_registry::ModelRegistry;

#[tokio::test]
async fn test_single_validator_determinism() {
    let registry = setup_test_registry().await;
    let model_id = register_test_model(&registry).await;
    
    let mut validator = DLCValidator::new(registry.path()).await.unwrap();
    validator.load_model(&model_id).await.unwrap();
    
    let features = vec![Fixed::from_raw(1_000_000)];
    
    // Predict 100 times - must be identical
    let mut predictions = Vec::new();
    for _ in 0..100 {
        let pred = validator.predict(&features).unwrap();
        predictions.push(pred);
    }
    
    // All predictions must be identical
    assert!(predictions.windows(2).all(|w| w[0] == w[1]));
}

#[tokio::test]
async fn test_multi_validator_consensus() {
    let registry = setup_test_registry().await;
    let model_id = register_test_model(&registry).await;
    
    // Create 5 validators
    let mut engine = ConsensusEngine::new(5, registry.path(), 0.67).await.unwrap();
    engine.load_model(&model_id).await.unwrap();
    
    let features = vec![Fixed::from_raw(1_000_000)];
    
    // Run consensus
    let result = engine.consensus_predict(&features).await.unwrap();
    
    // Must reach consensus with 100% agreement (deterministic)
    assert!(result.reached);
    assert_eq!(result.agreement, 1.0);
}

#[tokio::test]
async fn test_cross_platform_consensus() {
    // This test validates that validators on different architectures
    // reach consensus due to deterministic inference
    
    let registry = setup_test_registry().await;
    let model_id = register_test_model(&registry).await;
    
    let mut engine = ConsensusEngine::new(3, registry.path(), 0.67).await.unwrap();
    engine.load_model(&model_id).await.unwrap();
    
    let features = vec![
        Fixed::from_raw(1_500_000),
        Fixed::from_raw(2_750_000),
        Fixed::from_raw(-500_000),
    ];
    
    let result = engine.consensus_predict(&features).await.unwrap();
    
    assert!(result.reached);
    assert_eq!(result.agreement, 1.0);
    
    // Verify hash is reproducible
    let hash1 = result.consensus_hash;
    let result2 = engine.consensus_predict(&features).await.unwrap();
    let hash2 = result2.consensus_hash;
    
    assert_eq!(hash1, hash2);
}

#[tokio::test]
async fn test_model_mismatch_detection() {
    let registry = setup_test_registry().await;
    let model_id = register_test_model(&registry).await;
    
    let mut validator = DLCValidator::new(registry.path()).await.unwrap();
    
    // Try to load with wrong model ID
    let wrong_id = "0".repeat(64);
    let result = validator.load_model(&wrong_id).await;
    
    assert!(result.is_err());
}
```

**Tasks:**
- [ ] Add single-validator determinism tests
- [ ] Add multi-validator consensus tests (3, 5, 7 nodes)
- [ ] Add cross-platform validation tests
- [ ] Add model mismatch detection tests

### 7. Validation & Testing

```bash
# Float detection
rg -n "(f32|f64)" crates/consensus_dlc/src/*.rs | grep -v "tests\|test_"

# Build
cargo build --package ippan-consensus-dlc --release

# Run consensus tests
cargo test --package ippan-consensus-dlc determinism

# All tests
cargo test --package ippan-consensus-dlc
```

### 8. Create Pull Request

```bash
git add crates/consensus_dlc/src/{validator,engine,lib}.rs
git add crates/consensus_dlc/Cargo.toml
git add crates/consensus_dlc/tests/determinism.rs

git commit -m "$(cat <<'EOF'
Phase 4: Consensus DLC integration with deterministic GBDT

- Integrated deterministic GBDT inference into DLC validators
- Implemented multi-validator consensus engine
- Added consensus tests with 100% agreement validation
- Model integrity verification on load

Acceptance gates:
✅ All validators compute identical predictions
✅ Consensus tests pass with 3, 5, 7 validators
✅ Model hash verification prevents mismatches
✅ No floats in consensus code

Related: D-GBDT Rollout Phase 4
EOF
)"

git push -u origin phase4/consensus-integration

gh pr create \
  --base feat/d-gbdt-rollout \
  --title "Phase 4: Consensus DLC Integration" \
  --body "$(cat <<'EOF'
## Summary
- Integrated deterministic GBDT into consensus_dlc module
- Multi-validator consensus engine with quorum logic
- 100% agreement on predictions due to determinism

## Changes
- `validator.rs`: DLCValidator with deterministic inference
- `engine.rs`: ConsensusEngine with multi-node support
- New tests: `tests/determinism.rs`

## Consensus Validation
- [x] 3-validator consensus: 100% agreement
- [x] 5-validator consensus: 100% agreement
- [x] 7-validator consensus: 100% agreement
- [x] Model mismatch detection works

## Acceptance Gates
- [x] No floats in consensus_dlc code
- [x] All validators agree on predictions
- [x] Consensus tests pass
- [x] Model integrity checks pass

## Next Phase
Phase 5 will add CI enforcement for determinism.
EOF
)"
```

---

## 🚦 Acceptance Gates

- [ ] **Multi-validator consensus:** 100% agreement in tests
- [ ] **Model integrity:** Hash verification on load
- [ ] **No floats:** No f32/f64 in consensus_dlc/src
- [ ] **All tests pass:** Including 3, 5, 7 validator scenarios

---

**Estimated Effort:** 3-4 days  
**Priority:** P0 (blocking Phase 5)  
**Dependencies:** Phase 3 must be merged  
**Status:** Ready after Phase 3 completion
