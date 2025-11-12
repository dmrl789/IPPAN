# Agent 3: Model Registry Determinism

**Phase:** 3 of 7  
**Branch:** `phase3/model-registry` (from `feat/d-gbdt-rollout` after Phase 2 merge)  
**Assignee:** Agent-Theta  
**Scope:** `crates/ai_registry/src/{manifest.rs, registry.rs, storage.rs}`

---

## 🎯 Objective

Implement **canonical JSON serialization** and **reproducible BLAKE3 hashing** for model storage. Ensure model IDs are identical across all platforms.

---

## 📋 Task Checklist

### 1. Branch Setup

**Prerequisites:** Phase 2 PR must be merged to `feat/d-gbdt-rollout`

```bash
cd /workspace
git checkout feat/d-gbdt-rollout
git pull origin feat/d-gbdt-rollout
git checkout -b phase3/model-registry
```

### 2. Audit Current Serialization

**Files to audit:**
```bash
rg -n "(f32|f64)" crates/ai_registry/src/*.rs | grep -v "tests\|test_"
rg -n "serde_json::to" crates/ai_registry/src/*.rs
```

**Problems to identify:**
- [ ] Non-deterministic JSON key ordering
- [ ] Float precision issues in JSON serialization
- [ ] Inconsistent whitespace/formatting
- [ ] Platform-dependent timestamp formats

### 3. Implement Canonical JSON Serialization

**File:** `crates/ai_registry/src/manifest.rs`

**Add canonical serialization module:**
```rust
use serde::{Serialize, Serializer};
use serde_json::value::{Map, Value};
use std::collections::BTreeMap;

/// Canonical JSON serializer ensuring reproducible output
pub fn to_canonical_json<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let value = serde_json::to_value(value)?;
    let canonical = canonicalize_value(value);
    // No whitespace, sorted keys, fixed precision
    serde_json::to_string(&canonical)
}

fn canonicalize_value(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            // Sort keys for determinism
            let sorted: BTreeMap<String, Value> = map
                .into_iter()
                .map(|(k, v)| (k, canonicalize_value(v)))
                .collect();
            Value::Object(sorted.into_iter().collect())
        }
        Value::Array(arr) => {
            Value::Array(arr.into_iter().map(canonicalize_value).collect())
        }
        // Numbers: ensure Fixed types are used upstream
        Value::Number(_) => value,
        _ => value,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_canonical_json_ordering() {
        #[derive(Serialize)]
        struct Test {
            z_field: i32,
            a_field: i32,
            m_field: i32,
        }
        
        let obj = Test { z_field: 1, a_field: 2, m_field: 3 };
        let json = to_canonical_json(&obj).unwrap();
        
        // Keys must be sorted
        assert_eq!(json, r#"{"a_field":2,"m_field":3,"z_field":1}"#);
    }
    
    #[test]
    fn test_canonical_json_determinism() {
        let obj = create_test_model();
        let json1 = to_canonical_json(&obj).unwrap();
        let json2 = to_canonical_json(&obj).unwrap();
        
        // Must be byte-identical
        assert_eq!(json1, json2);
    }
}
```

**Tasks:**
- [ ] Implement `to_canonical_json` function
- [ ] Sort all object keys (use BTreeMap)
- [ ] Remove all whitespace from output
- [ ] Add unit tests for key ordering

### 4. Implement BLAKE3 Model Hashing

**File:** `crates/ai_registry/src/manifest.rs`

```rust
use blake3::Hasher;

/// Compute deterministic BLAKE3 hash of model
pub fn compute_model_hash(model: &GBDTModel) -> Result<[u8; 32], Error> {
    // 1. Serialize to canonical JSON
    let json = to_canonical_json(model)
        .map_err(|e| Error::Serialization(e.to_string()))?;
    
    // 2. Hash the canonical JSON bytes
    let mut hasher = Hasher::new();
    hasher.update(json.as_bytes());
    
    Ok(*hasher.finalize().as_bytes())
}

/// Convert hash to hex string for display
pub fn hash_to_hex(hash: &[u8; 32]) -> String {
    hex::encode(hash)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_hash_determinism_same_platform() {
        let model = create_test_model();
        let hash1 = compute_model_hash(&model).unwrap();
        let hash2 = compute_model_hash(&model).unwrap();
        
        assert_eq!(hash1, hash2);
    }
    
    #[test]
    fn test_hash_determinism_known_vector() {
        let model = create_simple_model();
        let hash = compute_model_hash(&model).unwrap();
        
        // Known expected hash (computed offline on x86_64)
        let expected: [u8; 32] = hex::decode(
            "a7c5e8d2f1b9c4a3e6d8f2b5c7a9e4d1f3b8c6a5e9d2f7b4c8a6e3d5f1b9c7a4"
        ).unwrap().try_into().unwrap();
        
        assert_eq!(hash, expected, "Hash mismatch - serialization not deterministic");
    }
}
```

**Tasks:**
- [ ] Implement `compute_model_hash` using canonical JSON
- [ ] Add known test vectors for validation
- [ ] Test on multiple platforms (use CI)

### 5. Update ModelManifest Structure

**File:** `crates/ai_registry/src/manifest.rs`

```rust
use ippan_ai_core::Fixed;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModelManifest {
    pub id: String, // BLAKE3 hash hex
    pub name: String,
    pub version: String,
    pub created_at: i64, // Unix timestamp (deterministic)
    pub model_type: String, // "gbdt"
    
    // Metadata using Fixed instead of f32/f64
    pub accuracy: Fixed,
    pub training_samples: u64,
    
    // Model hash for integrity
    pub content_hash: String, // BLAKE3 hex
    
    // Serialized model (canonical JSON)
    pub model_data: Vec<u8>, // Compressed canonical JSON
}

impl ModelManifest {
    pub fn new(model: &GBDTModel, name: String, version: String) -> Result<Self, Error> {
        // Compute hash
        let hash = compute_model_hash(model)?;
        let id = hash_to_hex(&hash);
        
        // Serialize to canonical JSON
        let json = to_canonical_json(model)?;
        
        // Compress for storage
        let model_data = compress_data(json.as_bytes())?;
        
        Ok(Self {
            id: id.clone(),
            name,
            version,
            created_at: current_timestamp(),
            model_type: "gbdt".to_string(),
            accuracy: Fixed::ZERO, // To be set by caller
            training_samples: 0,
            content_hash: id,
            model_data,
        })
    }
    
    pub fn verify_integrity(&self) -> Result<bool, Error> {
        // Decompress model data
        let json = decompress_data(&self.model_data)?;
        
        // Deserialize model
        let model: GBDTModel = serde_json::from_slice(&json)
            .map_err(|e| Error::Deserialization(e.to_string()))?;
        
        // Recompute hash
        let computed_hash = compute_model_hash(&model)?;
        let computed_hex = hash_to_hex(&computed_hash);
        
        // Verify match
        Ok(computed_hex == self.content_hash)
    }
}
```

**Tasks:**
- [ ] Update ModelManifest to store BLAKE3 hash
- [ ] Add `verify_integrity()` method
- [ ] Use Fixed types for all metrics
- [ ] Use i64 Unix timestamps (not chrono DateTime)

### 6. Update Registry Storage

**File:** `crates/ai_registry/src/registry.rs`

```rust
use crate::manifest::{ModelManifest, compute_model_hash, hash_to_hex};

pub struct ModelRegistry {
    storage: Storage,
}

impl ModelRegistry {
    pub async fn register_model(
        &mut self,
        model: &GBDTModel,
        name: String,
        version: String,
    ) -> Result<String, Error> {
        // Create manifest with deterministic hash
        let manifest = ModelManifest::new(model, name, version)?;
        
        // Verify hash before storing
        if !manifest.verify_integrity()? {
            return Err(Error::IntegrityCheckFailed);
        }
        
        // Store in registry
        self.storage.store(&manifest.id, &manifest).await?;
        
        tracing::info!(
            model_id = %manifest.id,
            name = %manifest.name,
            version = %manifest.version,
            "Model registered with deterministic hash"
        );
        
        Ok(manifest.id)
    }
    
    pub async fn load_model(&self, model_id: &str) -> Result<GBDTModel, Error> {
        // Load manifest
        let manifest = self.storage.load::<ModelManifest>(model_id).await?;
        
        // Verify integrity
        if !manifest.verify_integrity()? {
            return Err(Error::CorruptedModel(model_id.to_string()));
        }
        
        // Deserialize model
        let json = decompress_data(&manifest.model_data)?;
        let model = serde_json::from_slice(&json)
            .map_err(|e| Error::Deserialization(e.to_string()))?;
        
        Ok(model)
    }
}
```

**Tasks:**
- [ ] Add integrity checking on load
- [ ] Add integrity checking on store
- [ ] Log model hashes for audit trail

### 7. Add Cross-Platform Tests

Create `crates/ai_registry/tests/determinism.rs`:

```rust
use ippan_ai_registry::{ModelRegistry, ModelManifest};
use ippan_ai_core::{GBDTModel, Fixed};

#[tokio::test]
async fn test_model_hash_determinism() {
    let model = create_test_model();
    
    // Compute hash multiple times
    let hash1 = compute_model_hash(&model).unwrap();
    let hash2 = compute_model_hash(&model).unwrap();
    
    assert_eq!(hash1, hash2, "Hash not deterministic");
}

#[tokio::test]
async fn test_registry_round_trip() {
    let mut registry = ModelRegistry::new_temp().await.unwrap();
    let model = create_test_model();
    
    // Register model
    let model_id = registry.register_model(&model, "test".into(), "1.0".into())
        .await.unwrap();
    
    // Load model
    let loaded = registry.load_model(&model_id).await.unwrap();
    
    // Verify hash matches
    let original_hash = compute_model_hash(&model).unwrap();
    let loaded_hash = compute_model_hash(&loaded).unwrap();
    
    assert_eq!(original_hash, loaded_hash);
}

#[cfg(target_arch = "x86_64")]
#[tokio::test]
async fn test_x86_64_hash() {
    let model = create_standard_test_model();
    let hash = compute_model_hash(&model).unwrap();
    
    // Known hash computed on x86_64
    let expected = hex::decode(EXPECTED_HASH_X86_64).unwrap();
    assert_eq!(&hash[..], &expected[..]);
}

#[cfg(target_arch = "aarch64")]
#[tokio::test]
async fn test_aarch64_hash() {
    let model = create_standard_test_model();
    let hash = compute_model_hash(&model).unwrap();
    
    // Must match x86_64 hash
    let expected = hex::decode(EXPECTED_HASH_X86_64).unwrap();
    assert_eq!(&hash[..], &expected[..]);
}
```

**Tasks:**
- [ ] Add round-trip serialization tests
- [ ] Add known hash vectors for validation
- [ ] Add cross-platform consistency tests

### 8. Validation & Testing

```bash
# Float detection
rg -n "(f32|f64)" crates/ai_registry/src/*.rs | grep -v "tests\|test_"

# Build
cargo build --package ippan-ai-registry --release

# Run tests
cargo test --package ippan-ai-registry determinism

# All tests
cargo test --package ippan-ai-registry
```

### 9. Create Pull Request

```bash
git add crates/ai_registry/src/{manifest,registry,storage}.rs
git add crates/ai_registry/tests/determinism.rs

git commit -m "$(cat <<'EOF'
Phase 3: Model registry with deterministic serialization

- Canonical JSON serialization with sorted keys
- BLAKE3 model hashing for reproducible IDs
- Integrity verification on load/store
- Cross-platform hash validation tests

Acceptance gates:
✅ Canonical JSON with sorted keys
✅ BLAKE3 hashes match across platforms
✅ Integrity checks pass
✅ No floats in registry code

Related: D-GBDT Rollout Phase 3
EOF
)"

git push -u origin phase3/model-registry

gh pr create \
  --base feat/d-gbdt-rollout \
  --title "Phase 3: Model Registry Determinism" \
  --body "$(cat <<'EOF'
## Summary
- Implemented canonical JSON serialization for models
- BLAKE3 hashing ensures reproducible model IDs
- Added integrity verification for stored models

## Changes
- `manifest.rs`: Canonical serialization + BLAKE3 hashing
- `registry.rs`: Integrity checking on load/store
- New tests: `tests/determinism.rs`

## Determinism Validation
- [x] JSON key ordering: BTreeMap ensures sorted keys
- [x] Hash reproducibility: Same model → same hash on all platforms
- [x] Round-trip: Load + hash matches original hash

## Acceptance Gates
- [x] No floats in registry code
- [x] Canonical JSON tests pass
- [x] Hash tests pass with known vectors
- [x] Cross-platform tests (x86_64 + aarch64)

## Next Phase
Phase 4 will integrate into consensus_dlc module.
EOF
)"
```

---

## 🚦 Acceptance Gates

- [ ] **Canonical JSON:** Sorted keys, no whitespace, deterministic
- [ ] **BLAKE3 hashing:** Same model produces same hash on all platforms
- [ ] **Known vectors:** At least 3 test vectors with expected hashes
- [ ] **Integrity checks:** Load/store validation passes
- [ ] **No floats:** No f32/f64 in ai_registry/src

---

**Estimated Effort:** 2-3 days  
**Priority:** P0 (blocking Phase 4)  
**Dependencies:** Phase 2 must be merged  
**Status:** Ready after Phase 2 completion
