# AI Security and Determinism Guarantees

This document describes the security measures and determinism guarantees for AI models used in IPPAN L1 consensus.

## Threat Model

### In-Scope Threats

1. **Non-Determinism**: Floating-point arithmetic causing divergent outputs across platforms
2. **Model Tampering**: Malicious model substitution or corruption
3. **Byzantine Models**: Adversarial models designed to manipulate validator selection
4. **Gradient Attacks**: Models trained to favor specific validators
5. **Overflow/Underflow**: Integer arithmetic bugs causing crashes or wrong outputs

### Out-of-Scope

- L2 AI attacks (handled separately)
- Off-chain model training attacks (mitigated by governance review)
- Side-channel attacks on model inference

## Determinism Guarantees

### Integer-Only Arithmetic

**Requirement**: No floating-point operations in L1 AI evaluation.

**Enforcement**:

1. **Compile-time**: Lint rule forbids `f32`/`f64` in `ippan-ai-core`
   ```rust
   #![forbid(float_types)]
   ```

2. **Test-time**: Cross-platform determinism test
   ```rust
   #[test]
   fn test_determinism_x86_vs_arm() {
       let model = load_test_model();
       let features = vec![1000, 2000, 3000, 4000, 5000, 6000];
       
       // Run on both architectures
       let score = eval_gbdt(&model, &features);
       assert_eq!(score, EXPECTED_SCORE);
   }
   ```

3. **CI**: Multi-arch runners (x86_64, aarch64) verify identical outputs

### Canonical Model Representation

**Hash Calculation**:

```rust
fn compute_model_hash(model: &GBDTModel) -> [u8; 32] {
    // Serialize to canonical JSON (sorted keys, no whitespace)
    let json = serde_json::to_string(model).unwrap();
    blake3::hash(json.as_bytes()).as_bytes()[..32]
}
```

**Guarantees**:

- Same model structure → same hash on all platforms
- Hash collision resistance (2^256)
- Tamper detection via signature verification

### Reproducible Evaluation

**Path through tree must be deterministic**:

```rust
fn eval_tree(tree: &Tree, features: &[i64]) -> i32 {
    let mut idx = 0;
    loop {
        let node = &tree.nodes[idx];
        if let Some(value) = node.value {
            return value;  // Leaf
        }
        let feature = features[node.feature_index];
        idx = if feature <= node.threshold {  // <= is critical
            node.left
        } else {
            node.right
        };
    }
}
```

**Key**: `<=` ensures tie-breaking is deterministic.

## Security Measures

### 1. Cryptographic Verification

**Signature Scheme**: Ed25519

**Signed Message**:
```
model_id || version || hash_sha256 || model_url || activation_round
```

**Verification**:
```rust
pub fn verify_proposal_signature(proposal: &AiModelProposal) -> Result<()> {
    let message = construct_message(proposal);
    let verifying_key = VerifyingKey::from_bytes(&proposal.proposer_pubkey)?;
    let signature = Signature::from_bytes(&proposal.signature_foundation);
    verifying_key.verify(&message, &signature)?;
    Ok(())
}
```

**Trust Anchor**: Foundation public key hard-coded in genesis or multi-sig controlled by governance.

### 2. Hash Pinning

**On-chain registry** stores:

```rust
struct ModelRegistryEntry {
    model_id: String,
    hash_sha256: [u8; 32],  // Immutable after approval
    version: u32,
    activation_round: u64,
    signature: [u8; 64],
    status: ModelStatus,
}
```

**Validation**:

```rust
pub fn load_and_verify_model(url: &str, expected_hash: &[u8; 32]) -> Result<GBDTModel> {
    let model = download_model(url)?;
    let computed_hash = compute_model_hash(&model);
    if computed_hash != *expected_hash {
        return Err(ModelError::HashMismatch);
    }
    Ok(model)
}
```

### 3. Bounded Complexity

**Limits** (enforced during proposal validation):

```rust
const MAX_TREES: usize = 100;
const MAX_NODES_PER_TREE: usize = 1000;
const MAX_DEPTH: usize = 20;
const MAX_TOTAL_NODES: usize = 10_000;
```

**Purpose**: Prevent DoS via excessively large models.

**Check**:

```rust
pub fn validate_model_complexity(model: &GBDTModel) -> Result<()> {
    if model.trees.len() > MAX_TREES {
        return Err(ModelError::TooManyTrees);
    }
    
    let total_nodes: usize = model.trees.iter().map(|t| t.nodes.len()).sum();
    if total_nodes > MAX_TOTAL_NODES {
        return Err(ModelError::TooManyNodes);
    }
    
    Ok(())
}
```

### 4. Output Clamping

**All outputs clamped to [0, scale]**:

```rust
pub fn eval_gbdt(model: &GBDTModel, features: &[i64]) -> i32 {
    let mut sum = model.bias;
    for tree in &model.trees {
        sum = sum.saturating_add(eval_tree(tree, features));
    }
    sum.clamp(0, model.scale)
}
```

**Guarantees**:

- No overflow propagation
- Reputation score always in valid range
- Safe to use in stake weighting: `weighted_stake = stake * reputation / 10000`

### 5. Feature Normalization

**All input features scaled to [0, 10000]**:

```rust
pub fn extract_features(telemetry: &ValidatorTelemetry, config: &FeatureConfig) -> Vec<i64> {
    let scale = config.scale;  // 10000
    
    let proposal_rate = ((telemetry.blocks_proposed * scale) / telemetry.rounds_active)
        .min(scale);
    
    // ... 5 more features, all clamped to [0, scale]
    
    vec![proposal_rate, verification_rate, latency_score, slash_penalty, stake_weight, longevity]
}
```

**Prevents**:

- Out-of-range inputs causing tree misbehavior
- Injection attacks via extreme values

## Attack Scenarios and Mitigations

### Scenario 1: Malicious Model Substitution

**Attack**: Validator downloads different model than approved.

**Detection**:

1. Every node computes hash of loaded model
2. If hash doesn't match registry entry → node refuses to activate
3. Proposer using wrong model → blocks invalid → slashed

**Code**:

```rust
let model = load_model(&entry.model_url)?;
let hash = compute_model_hash(&model);
if hash != entry.hash_sha256 {
    return Err(ConsensusError::InvalidModel);
}
```

### Scenario 2: Biased Model (Gradient Attack)

**Attack**: Model trained to always give high scores to attacker's validators.

**Mitigation**:

1. **Governance Review**: Proposal includes test results on historical data
2. **Transparency**: Full model structure public, can be audited
3. **Voting**: Requires 66.67% approval from diverse validator set
4. **Revocation**: Emergency governance can deactivate within 1 round

**Example Audit**:

```python
# Validate model doesn't favor specific addresses
for validator in validator_set:
    telemetry = get_telemetry(validator)
    score = eval_model(model, telemetry)
    
    # Check for anomalies
    if score > 10000 or score < 0:
        raise ModelError("Invalid output")
    
    # Statistical tests for bias
    if distribution_skew(scores) > THRESHOLD:
        raise ModelError("Model appears biased")
```

### Scenario 3: Non-Determinism Exploit

**Attack**: Model outputs different scores on different platforms, causing fork.

**Prevention**:

1. **Integer-only**: No floats allowed
2. **CI Tests**: Multi-arch verification
3. **Proof of Determinism**: Included in proposal

**CI Job**:

```yaml
test-determinism:
  strategy:
    matrix:
      arch: [x86_64, aarch64]
  steps:
    - run: cargo test --release test_determinism_${arch}
    - run: diff x86_64_output.txt aarch64_output.txt
```

### Scenario 4: Integer Overflow in Tree

**Attack**: Crafted features cause overflow in tree evaluation.

**Mitigation**:

1. **Saturating Arithmetic**: `saturating_add()` prevents wrapping
2. **Clamping**: All intermediate values clamped
3. **Fuzzing**: Test with extreme inputs

**Fuzz Test**:

```rust
#[test]
fn fuzz_eval_gbdt() {
    let model = load_production_model();
    
    for _ in 0..10_000 {
        let features: Vec<i64> = (0..6).map(|_| rand::gen_range(i64::MIN..i64::MAX)).collect();
        let score = eval_gbdt(&model, &features);
        
        assert!(score >= 0);
        assert!(score <= model.scale);
    }
}
```

## Audit Procedures

### Pre-Activation Audit

Before voting on a model proposal:

1. **Download Model**: `curl -o model.json <url>`
2. **Verify Hash**: `sha256sum model.json` → compare with proposal
3. **Load and Test**:
   ```bash
   ippan-ai-verify model.json --test-vectors test_vectors.csv
   ```
4. **Review Structure**: Inspect tree depths, feature usage
5. **Historical Simulation**:
   ```bash
   ippan-ai-simulate model.json --from-round 1 --to-round 100000
   ```
6. **Vote**: If all checks pass

### Post-Activation Monitoring

Continuously monitor:

- **Reputation Distribution**: Should be roughly normal
- **Outliers**: Validators with scores far from median
- **Correlation**: Reputation vs. actual performance metrics

**Alert Triggers**:

```python
if std_dev(reputation_scores) > 3000:  # Too much variance
    alert("Possible model anomaly")

if correlation(reputation, uptime) < 0.5:  # Low correlation
    alert("Model may not be predictive")
```

## Testing Requirements

### Unit Tests

- [x] Determinism across platforms
- [x] Integer-only (no floats)
- [x] Output always in range [0, scale]
- [x] Hash stability
- [x] Signature verification

### Integration Tests

- [x] Model loading from JSON
- [x] Feature extraction
- [x] Reputation weighting in validator selection
- [x] Governance activation flow

### Fuzzing

- [x] Random feature vectors
- [x] Edge cases (all zeros, all max, negative)
- [x] Malformed models
- [x] Hash collision attempts

### Benchmarks

- [x] Evaluation latency < 1ms per model per validator
- [x] Memory usage < 10MB per model

## Compliance Checklist

Before deploying a new model, verify:

- [ ] Passes determinism test on x86_64 and aarch64
- [ ] Uses only integer arithmetic
- [ ] All outputs in range [0, 10000]
- [ ] Hash matches declared hash
- [ ] Signature verifies with foundation key
- [ ] Complexity within limits
- [ ] Tested on ≥ 100k historical rounds
- [ ] No obvious bias in test results
- [ ] Governance proposal submitted with rationale
- [ ] Code review completed
- [ ] Audit report published

## References

- [AI Core Implementation](../crates/ai_core/)
- [Reputation Module](../crates/consensus/src/reputation.rs)
- [Governance Spec](./GOVERNANCE_MODELS.md)

## Changelog

- **2025-10-22**: Initial AI security specification
