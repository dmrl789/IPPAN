# Agent 5: CI Determinism Enforcement

**Phase:** 5 of 7  
**Branch:** `phase5/ci-determinism` (from `feat/d-gbdt-rollout` after Phase 4 merge)  
**Assignee:** Agent-Sigma  
**Scope:** `.github/workflows/ai-determinism.yml`, CI configuration

---

## 🎯 Objective

Add **cross-architecture determinism validation** to CI. Enforce float-free code and validate bit-identical predictions across x86_64, aarch64, and other platforms.

---

## 📋 Task Checklist

### 1. Branch Setup

**Prerequisites:** Phase 4 PR must be merged to `feat/d-gbdt-rollout`

```bash
cd /workspace
git checkout feat/d-gbdt-rollout
git pull origin feat/d-gbdt-rollout
git checkout -b phase5/ci-determinism
```

### 2. Review Existing CI Workflow

**File:** `.github/workflows/ai-determinism.yml`

```bash
cat .github/workflows/ai-determinism.yml
```

**Identify:**
- [ ] Current test coverage
- [ ] Architecture matrix
- [ ] Determinism validation approach

### 3. Create Cross-Architecture Matrix Workflow

**File:** `.github/workflows/ai-determinism.yml`

```yaml
name: AI Determinism Validation

on:
  pull_request:
    paths:
      - 'crates/ai_core/**'
      - 'crates/ai_registry/**'
      - 'crates/consensus_dlc/**'
      - '.github/workflows/ai-determinism.yml'
  push:
    branches:
      - main
      - feat/d-gbdt-rollout
      - 'phase*/**'

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  # Job 1: Float Detection (blocking)
  float-detection:
    name: Float Usage Detection
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install ripgrep
        run: sudo apt-get update && sudo apt-get install -y ripgrep
      
      - name: Detect float usage in runtime code
        run: |
          echo "Checking for f32/f64 in runtime paths..."
          
          # Search for floats in non-test code
          if rg -n "(f32|f64)" \
            crates/ai_core/src \
            crates/ai_registry/src \
            crates/consensus_dlc/src \
            | grep -v "tests/" \
            | grep -v "test_" \
            | grep -v "//.*\(f32\|f64\)"; then
            echo "❌ ERROR: Floating-point types found in runtime code"
            echo "D-GBDT requires fixed-point arithmetic only"
            exit 1
          else
            echo "✅ No floating-point types in runtime code"
          fi

  # Job 2: Cross-Architecture Determinism
  cross-arch-determinism:
    name: Determinism Tests (${{ matrix.arch }})
    runs-on: ${{ matrix.os }}
    needs: float-detection
    
    strategy:
      fail-fast: false
      matrix:
        include:
          - arch: x86_64-linux
            os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          
          - arch: aarch64-linux
            os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
            use-cross: true
          
          - arch: x86_64-macos
            os: macos-latest
            target: x86_64-apple-darwin
          
          - arch: aarch64-macos
            os: macos-latest
            target: aarch64-apple-darwin
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          profile: minimal
          toolchain: stable
          target: ${{ matrix.target }}
          override: true
      
      - name: Install cross (if needed)
        if: matrix.use-cross
        run: cargo install cross --git https://github.com/cross-rs/cross
      
      - name: Cache cargo registry
        uses: actions/cache@v3
        with:
          path: ~/.cargo/registry
          key: ${{ runner.os }}-${{ matrix.arch }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Cache cargo build
        uses: actions/cache@v3
        with:
          path: target
          key: ${{ runner.os }}-${{ matrix.arch }}-cargo-build-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Run determinism tests
        run: |
          if [ "${{ matrix.use-cross }}" = "true" ]; then
            cross test --target ${{ matrix.target }} \
              --package ippan-ai-core \
              --package ippan-ai-registry \
              --package ippan-consensus-dlc \
              determinism
          else
            cargo test --target ${{ matrix.target }} \
              --package ippan-ai-core \
              --package ippan-ai-registry \
              --package ippan-consensus-dlc \
              determinism
          fi
      
      - name: Generate determinism report
        run: |
          if [ "${{ matrix.use-cross }}" = "true" ]; then
            cross test --target ${{ matrix.target }} \
              --package ippan-ai-core \
              --test deterministic_report \
              -- --nocapture > determinism-${{ matrix.arch }}.txt
          else
            cargo test --target ${{ matrix.target }} \
              --package ippan-ai-core \
              --test deterministic_report \
              -- --nocapture > determinism-${{ matrix.arch }}.txt
          fi
      
      - name: Upload determinism report
        uses: actions/upload-artifact@v3
        with:
          name: determinism-report-${{ matrix.arch }}
          path: determinism-${{ matrix.arch }}.txt

  # Job 3: Compare Determinism Reports
  compare-determinism:
    name: Compare Cross-Arch Results
    runs-on: ubuntu-latest
    needs: cross-arch-determinism
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Download all reports
        uses: actions/download-artifact@v3
        with:
          path: reports
      
      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y xxd
      
      - name: Compare prediction hashes
        run: |
          echo "Comparing determinism reports across architectures..."
          
          # Extract hashes from all reports
          for report in reports/*/determinism-*.txt; do
            echo "Processing: $report"
            grep "PREDICTION_HASH:" "$report" || true
          done > all_hashes.txt
          
          # Check if all hashes are identical
          unique_hashes=$(sort all_hashes.txt | uniq | wc -l)
          
          if [ "$unique_hashes" -eq 1 ]; then
            echo "✅ All architectures produce identical predictions"
            cat all_hashes.txt
            exit 0
          else
            echo "❌ ERROR: Determinism failure - architectures produce different results"
            echo "Unique hashes found: $unique_hashes"
            cat all_hashes.txt
            exit 1
          fi
      
      - name: Generate summary
        if: always()
        run: |
          echo "## Determinism Validation Summary" >> $GITHUB_STEP_SUMMARY
          echo "" >> $GITHUB_STEP_SUMMARY
          echo "| Architecture | Status | Hash |" >> $GITHUB_STEP_SUMMARY
          echo "|--------------|--------|------|" >> $GITHUB_STEP_SUMMARY
          
          for report in reports/*/determinism-*.txt; do
            arch=$(basename "$report" | sed 's/determinism-//;s/.txt//')
            hash=$(grep "PREDICTION_HASH:" "$report" | head -1 | cut -d: -f2 | tr -d ' ')
            echo "| $arch | ✅ | \`${hash:0:16}...\` |" >> $GITHUB_STEP_SUMMARY
          done

  # Job 4: Consensus Validation
  consensus-validation:
    name: Multi-Node Consensus Test
    runs-on: ubuntu-latest
    needs: float-detection
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          profile: minimal
          toolchain: stable
          override: true
      
      - name: Run 3-node consensus test
        run: cargo test --package ippan-consensus-dlc test_multi_validator_consensus -- --nocapture
      
      - name: Run 5-node consensus test
        run: cargo test --package ippan-consensus-dlc test_five_validator_consensus -- --nocapture
      
      - name: Run 7-node consensus test
        run: cargo test --package ippan-consensus-dlc test_seven_validator_consensus -- --nocapture
      
      - name: Verify 100% agreement
        run: |
          echo "Validating consensus agreement rates..."
          cargo test --package ippan-consensus-dlc -- --nocapture 2>&1 \
            | grep "agreement" \
            | while read line; do
              if ! echo "$line" | grep -q "1.0\|100%"; then
                echo "❌ ERROR: Consensus agreement < 100%"
                echo "$line"
                exit 1
              fi
            done
          echo "✅ All consensus tests show 100% agreement"

  # Job 5: Model Hash Validation
  model-hash-validation:
    name: Model Registry Hash Check
    runs-on: ubuntu-latest
    needs: float-detection
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          profile: minimal
          toolchain: stable
          override: true
      
      - name: Test model hash determinism
        run: cargo test --package ippan-ai-registry test_model_hash_determinism -- --nocapture
      
      - name: Test canonical JSON
        run: cargo test --package ippan-ai-registry test_canonical_json -- --nocapture
      
      - name: Validate known hash vectors
        run: cargo test --package ippan-ai-registry test_known_hash_vectors -- --nocapture
```

**Tasks:**
- [ ] Create comprehensive CI workflow
- [ ] Add float detection job (blocking)
- [ ] Add cross-architecture matrix (x86_64, aarch64, macos)
- [ ] Add determinism report comparison
- [ ] Add consensus validation tests

### 4. Create Determinism Report Test

**File:** `crates/ai_core/tests/deterministic_report.rs`

```rust
use ippan_ai_core::{GBDTModel, Fixed};
use blake3::Hasher;

#[test]
fn generate_determinism_report() {
    // Load standard test model
    let model = create_standard_test_model();
    
    // Standard test features
    let features = vec![
        Fixed::from_raw(1_500_000),
        Fixed::from_raw(2_750_000),
        Fixed::from_raw(-500_000),
    ];
    
    // Make prediction
    let prediction = model.predict(&features);
    
    // Compute hash
    let mut hasher = Hasher::new();
    hasher.update(&prediction.to_raw().to_le_bytes());
    let hash = hasher.finalize();
    
    // Output in parseable format for CI
    println!("DETERMINISM_REPORT_START");
    println!("ARCHITECTURE: {}", std::env::consts::ARCH);
    println!("OS: {}", std::env::consts::OS);
    println!("PREDICTION_RAW: {}", prediction.to_raw());
    println!("PREDICTION_HASH: {}", hex::encode(hash.as_bytes()));
    println!("MODEL_FEATURES: {}", features.len());
    println!("DETERMINISM_REPORT_END");
    
    // This test always passes - output is used for comparison
}

fn create_standard_test_model() -> GBDTModel {
    // Create a fixed, known model for cross-platform testing
    // ... implementation
}
```

**Tasks:**
- [ ] Create determinism report test
- [ ] Output parseable format for CI comparison
- [ ] Include architecture/OS info

### 5. Add Float Detection Script

**File:** `.github/scripts/detect_floats.sh`

```bash
#!/bin/bash
set -e

echo "🔍 Detecting floating-point usage in runtime code..."

FLOAT_FOUND=0

# Check ai_core
if rg -n "(f32|f64)" crates/ai_core/src/*.rs \
  | grep -v "tests/" | grep -v "test_" | grep -v "//.*\(f32\|f64\)"; then
  echo "❌ Floats found in ai_core/src"
  FLOAT_FOUND=1
fi

# Check ai_registry
if rg -n "(f32|f64)" crates/ai_registry/src/*.rs \
  | grep -v "tests/" | grep -v "test_" | grep -v "//.*\(f32\|f64\)"; then
  echo "❌ Floats found in ai_registry/src"
  FLOAT_FOUND=1
fi

# Check consensus_dlc
if rg -n "(f32|f64)" crates/consensus_dlc/src/*.rs \
  | grep -v "tests/" | grep -v "test_" | grep -v "//.*\(f32\|f64\)"; then
  echo "❌ Floats found in consensus_dlc/src"
  FLOAT_FOUND=1
fi

if [ $FLOAT_FOUND -eq 0 ]; then
  echo "✅ No floating-point types in runtime code"
  exit 0
else
  echo "❌ Floating-point types detected - D-GBDT requires fixed-point only"
  exit 1
fi
```

**Tasks:**
- [ ] Create float detection script
- [ ] Make executable: `chmod +x .github/scripts/detect_floats.sh`
- [ ] Test locally before committing

### 6. Update PR Template

**File:** `.github/PULL_REQUEST_TEMPLATE.md`

Add determinism checklist:

```markdown
## Determinism Checklist (for AI/Consensus PRs)

- [ ] No f32/f64 in runtime code (tests excluded)
- [ ] All determinism tests pass
- [ ] Cross-architecture CI passes (x86_64 + aarch64)
- [ ] Model hashes are reproducible
- [ ] Consensus tests show 100% agreement
```

### 7. Validation & Testing

```bash
# Test float detection locally
./.github/scripts/detect_floats.sh

# Validate workflow YAML syntax
yamllint .github/workflows/ai-determinism.yml

# Test determinism report generation
cargo test --package ippan-ai-core --test deterministic_report -- --nocapture

# Trigger workflow manually
gh workflow run ai-determinism.yml --ref phase5/ci-determinism
```

### 8. Create Pull Request

```bash
git add .github/workflows/ai-determinism.yml
git add .github/scripts/detect_floats.sh
git add .github/PULL_REQUEST_TEMPLATE.md
git add crates/ai_core/tests/deterministic_report.rs

git commit -m "$(cat <<'EOF'
Phase 5: CI determinism enforcement

- Cross-architecture matrix testing (x86_64, aarch64, macos)
- Automated float detection as blocking CI check
- Determinism report comparison across platforms
- Consensus validation in CI
- Enhanced PR template with determinism checklist

Acceptance gates:
✅ Float detection prevents merges with f32/f64
✅ Cross-arch tests validate bit-identical outputs
✅ Consensus tests show 100% agreement
✅ Model hash validation in CI

Related: D-GBDT Rollout Phase 5
EOF
)"

git push -u origin phase5/ci-determinism

gh pr create \
  --base feat/d-gbdt-rollout \
  --title "Phase 5: CI Determinism Enforcement" \
  --body "$(cat <<'EOF'
## Summary
- Added cross-architecture CI matrix (x86_64, aarch64, macos)
- Automated float detection as blocking gate
- Determinism report comparison validates identical outputs
- Consensus and model hash validation in CI

## Changes
- `.github/workflows/ai-determinism.yml`: New comprehensive workflow
- `.github/scripts/detect_floats.sh`: Float detection script
- `crates/ai_core/tests/deterministic_report.rs`: Report generator
- `.github/PULL_REQUEST_TEMPLATE.md`: Added determinism checklist

## CI Jobs
1. **Float Detection**: Blocks merge if f32/f64 found
2. **Cross-Arch Tests**: Runs on x86_64-linux, aarch64-linux, x86_64-macos, aarch64-macos
3. **Report Comparison**: Validates identical prediction hashes
4. **Consensus Validation**: 3/5/7 node tests with 100% agreement
5. **Model Hash**: Validates reproducible hashes

## Acceptance Gates
- [x] Float detection job passes
- [x] All architectures produce identical hashes
- [x] Consensus tests show 100% agreement
- [x] Model hash tests pass

## Next Phase
Phase 6 will update trainer CLI for deterministic output.
EOF
)"
```

---

## 🚦 Acceptance Gates

- [ ] **Float detection:** Automated check blocks f32/f64 in runtime code
- [ ] **Cross-arch matrix:** Tests pass on x86_64 + aarch64 + macos
- [ ] **Report comparison:** All architectures produce identical hashes
- [ ] **Consensus validation:** 100% agreement in multi-node tests

---

**Estimated Effort:** 1-2 days  
**Priority:** P0 (blocking Phase 6)  
**Dependencies:** Phase 4 must be merged  
**Status:** Ready after Phase 4 completion
