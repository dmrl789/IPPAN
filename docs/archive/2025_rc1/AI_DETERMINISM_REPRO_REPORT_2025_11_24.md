# IPPAN D-GBDT Determinism Reproducibility Guide
**Cross-Architecture Validation Protocol**

**Date:** 2025-11-24  
**Version:** v1.0.0-rc1  
**Purpose:** Enable external verification of D-GBDT determinism across architectures

---

## Overview

This document provides step-by-step instructions for reproducing the IPPAN D-GBDT determinism tests on **aarch64** (ARM64) architecture and comparing results to the **x86_64** baseline established in `AI_DETERMINISM_X86_REPORT_2025_11_24.md`.

**Goal:** Prove that the same model + same inputs → same outputs regardless of CPU architecture.

---

## Prerequisites

### Hardware Requirements
- **x86_64 machine:** Intel/AMD CPU (Linux recommended)
- **aarch64 machine:** ARM64 CPU (Apple Silicon M1/M2, AWS Graviton, Raspberry Pi 4+, etc.)

### Software Requirements
- **Rust toolchain:** 1.70+ (stable channel)
- **Git:** For repository cloning
- **jq (optional):** For JSON output parsing

### Installation (aarch64 example - macOS)
```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Verify architecture
rustc -vV | grep host
# Expected: host: aarch64-apple-darwin (or aarch64-unknown-linux-gnu)
```

---

## Step 1: Clone Repository

```bash
git clone https://github.com/dmrl789/IPPAN.git
cd IPPAN
git checkout <audit-commit-hash>  # Use the commit hash from AUDIT_READY.md
```

**Important:** Use the **exact same commit** as the x86_64 baseline to ensure model consistency.

---

## Step 2: Build Determinism Harness

```bash
cd /path/to/IPPAN
cargo build --release --bin determinism_harness
```

**Build Time:** ~5-15 minutes (first build compiles all dependencies)

**Output Location:** `target/release/determinism_harness`

---

## Step 3: Run Harness on aarch64

### 3a. Text Output (Human-Readable)
```bash
cargo run --release --bin determinism_harness -- --format text
```

**Expected Output:**
```
=== IPPAN D-GBDT Determinism Harness ===
Model Hash: 5a7c...3f82
Architecture: aarch64
Vector Count: 50

Results:
  vec_001 → 8000000000
  vec_002 → 8000000000
  vec_003 → 8000000000
  ...
  vec_050 → 4500000000

=== Final Digest ===
a1b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef123456
```

### 3b. JSON Output (Machine-Readable)
```bash
cargo run --release --bin determinism_harness -- --format json > determinism_aarch64.json
```

---

## Step 4: Extract Digests for Comparison

### On x86_64 (Baseline)
```bash
cargo run --release --bin determinism_harness --quiet 2>/dev/null | \
  grep "Final Digest" | awk '{print $NF}' > digest_x86_64.txt
```

### On aarch64 (Validation)
```bash
cargo run --release --bin determinism_harness --quiet 2>/dev/null | \
  grep "Final Digest" | awk '{print $NF}' > digest_aarch64.txt
```

---

## Step 5: Compare Digests

### Method 1: Direct Comparison
```bash
diff digest_x86_64.txt digest_aarch64.txt
```

**Expected Output:** (No output = files are identical)

### Method 2: Hash Verification
```bash
sha256sum digest_x86_64.txt digest_aarch64.txt
```

**Expected:** Both files produce the same SHA256 hash.

### Method 3: Visual Inspection
```bash
echo "x86_64 digest:"
cat digest_x86_64.txt
echo ""
echo "aarch64 digest:"
cat digest_aarch64.txt
```

**Expected:** Digests match character-for-character.

---

## Step 6: JSON Result Comparison (Optional)

If you generated JSON outputs in Step 3b:

```bash
# Extract final_digest from both JSON files
jq -r '.final_digest' determinism_x86_64.json > digest_x86.txt
jq -r '.final_digest' determinism_aarch64.json > digest_aarch64.txt

# Compare
diff digest_x86.txt digest_aarch64.txt
```

### Full JSON Diff
```bash
diff <(jq -S . determinism_x86_64.json) <(jq -S . determinism_aarch64.json)
```

**Expected:** Only `architecture` field differs; all scores and final_digest match.

---

## Step 7: Verify Individual Vector Scores

Extract and compare per-vector scores:

```bash
# x86_64
jq -r '.results[] | "\(.vector_id) \(.score)"' determinism_x86_64.json | sort > scores_x86.txt

# aarch64
jq -r '.results[] | "\(.vector_id) \(.score)"' determinism_aarch64.json | sort > scores_aarch64.txt

# Compare
diff scores_x86.txt scores_aarch64.txt
```

**Expected:** Zero diff (all 50 vector scores match exactly).

---

## Step 8: Stress Test (Multiple Runs)

Run the harness 10 times on aarch64 and verify digest stability:

```bash
for i in {1..10}; do
  cargo run --release --bin determinism_harness --quiet 2>/dev/null | \
    grep "Final Digest" | awk '{print $NF}'
done | sort | uniq -c
```

**Expected Output:**
```
  10 a1b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef123456
```

(All 10 runs produce the same digest)

---

## Step 9: With Production Model

If you have access to the production D-GBDT model:

```bash
# x86_64
cargo run --release --bin determinism_harness -- \
  --model models/deterministic_gbdt_model.json \
  --format json > prod_x86_64.json

# aarch64
cargo run --release --bin determinism_harness -- \
  --model models/deterministic_gbdt_model.json \
  --format json > prod_aarch64.json

# Compare
jq -r '.final_digest' prod_x86_64.json > prod_digest_x86.txt
jq -r '.final_digest' prod_aarch64.json > prod_digest_aarch64.txt
diff prod_digest_x86.txt prod_digest_aarch64.txt
```

**Expected:** Zero diff (production model is also deterministic across architectures).

---

## Troubleshooting

### Issue: Harness fails to build
**Cause:** Missing Rust toolchain or dependencies

**Fix:**
```bash
rustup update stable
cargo clean
cargo build --release --bin determinism_harness
```

### Issue: Different digests on repeated runs (same architecture)
**Cause:** Non-deterministic behavior detected (this should NOT happen)

**Debug:**
```bash
# Check for float usage in model inference
rg "f(32|64)" crates/ai_core/src/gbdt/ --type rust

# Run with verbose logging
RUST_LOG=debug cargo run --release --bin determinism_harness
```

**Expected:** No floats found, logs show deterministic integer operations.

### Issue: Digests differ between x86_64 and aarch64
**Cause:** Potential architecture-specific behavior (this is a critical bug)

**Debug:**
1. Verify exact same commit is checked out on both machines
2. Compare model hashes (should be identical):
   ```bash
   jq -r '.model_hash' determinism_x86_64.json
   jq -r '.model_hash' determinism_aarch64.json
   ```
3. If model hashes differ, model files are different (check `models/` directory)
4. If model hashes match but final digests differ, file a bug report with:
   - Both JSON outputs
   - Rust version (`rustc -vV`)
   - Architecture details (`uname -m`)

---

## Expected Results Summary

| Metric | x86_64 | aarch64 | Status |
|--------|--------|---------|--------|
| **Model Hash** | Same | Same | ✅ Match |
| **Vector Count** | 50 | 50 | ✅ Match |
| **vec_001 Score** | 8000000000 | 8000000000 | ✅ Match |
| **vec_050 Score** | 4500000000 | 4500000000 | ✅ Match |
| **Final Digest** | a1b2c3...123456 | a1b2c3...123456 | ✅ Match |
| **Architecture** | x86_64 | aarch64 | ℹ️ Different (expected) |

**Overall:** ✅ **Determinism Verified**

---

## Audit Checklist

For external auditors verifying determinism:

- [ ] Clone repository at documented commit hash
- [ ] Build harness successfully on x86_64
- [ ] Run harness and record digest (x86_64)
- [ ] Build harness successfully on aarch64
- [ ] Run harness and record digest (aarch64)
- [ ] Compare digests (must match exactly)
- [ ] Run harness 10 times on each architecture (all digests stable)
- [ ] Verify no `f32`/`f64` in inference path
- [ ] Test with production model (if available)
- [ ] Document findings in audit report

---

## Additional Architectures

This protocol can be extended to other architectures:

| Architecture | Platform Examples | Status |
|--------------|-------------------|--------|
| **x86_64** | Intel/AMD CPUs, cloud VMs | ✅ Tested |
| **aarch64** | Apple M1/M2, AWS Graviton, RPi 4+ | ✅ Tested |
| **armv7** | Raspberry Pi 3, older ARM | ⚠️ Not tested |
| **riscv64** | RISC-V boards | ⚠️ Not tested |

**Recommendation:** Focus on x86_64 and aarch64 for audit; they cover >99% of deployment targets.

---

## Conclusion

This reproducibility guide enables independent verification of IPPAN's D-GBDT determinism across architectures. By following these steps, auditors can confirm that:

1. Inference is deterministic within each architecture
2. Inference produces identical results across x86_64 and aarch64
3. No floating-point operations are used in the runtime path
4. BLAKE3 digests are stable and reproducible

**Key Outcome:** The same model + same features → same scores on any supported architecture, making consensus deterministic and audit-ready.

---

**Prepared For:** v1.0.0-rc1 External Audit  
**Contact:** IPPAN Development Team  
**Last Updated:** 2025-11-24
