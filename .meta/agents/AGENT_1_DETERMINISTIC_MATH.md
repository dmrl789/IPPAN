# Agent 1: Deterministic Math Foundation

**Phase:** 1 of 7  
**Branch:** `phase1/deterministic-math` (from `feat/d-gbdt-rollout`)  
**Assignee:** Agent-Alpha  
**Scope:** `crates/ai_core/src/{fixed.rs, fixed_point.rs, determinism.rs}`

---

## 🎯 Objective

Harden the fixed-point math foundation to ensure **bit-identical** results across all platforms (x86_64, aarch64, wasm32). Eliminate all f32/f64 usage from runtime inference paths.

---

## 📋 Task Checklist

### 1. Branch Setup
```bash
cd /workspace
git checkout feat/d-gbdt-rollout
git pull origin feat/d-gbdt-rollout
git checkout -b phase1/deterministic-math
```

### 2. Audit Fixed-Point Implementation

**Files to audit:**
- `crates/ai_core/src/fixed.rs` - Primary fixed-point type
- `crates/ai_core/src/fixed_point.rs` - Fixed-point utilities
- `crates/ai_core/src/determinism.rs` - Determinism helpers

**Check for:**
- [ ] Any f32/f64 conversions in non-test code
- [ ] Undefined overflow behavior (use saturating/wrapping arithmetic)
- [ ] Division operations that could produce platform-dependent rounding
- [ ] Transcendental functions (exp, log, sqrt) - ensure fixed-point implementations

**Command to detect floats:**
```bash
rg -n "(f32|f64)" crates/ai_core/src/fixed*.rs crates/ai_core/src/determinism.rs \
  | grep -v "tests\|test_" | grep -v "//.*\(f32\|f64\)"
```

### 3. Harden Arithmetic Operations

Ensure all operations use deterministic Rust primitives:

```rust
// ❌ BAD - platform-dependent
let result = (a * b) / c;

// ✅ GOOD - saturating arithmetic
let result = a.saturating_mul(b).saturating_div(c);

// ❌ BAD - may panic or have undefined behavior
let result = a / b;

// ✅ GOOD - checked division with fallback
let result = a.checked_div(b).unwrap_or(Fixed::ZERO);
```

**Required changes:**
- [ ] Replace all unchecked arithmetic with `saturating_*` or `checked_*` variants
- [ ] Document overflow behavior in doc comments
- [ ] Add debug assertions for overflow detection in dev builds

### 4. Implement Missing Operations

**Required operations:**
- [ ] Fixed-point multiplication with scaling
- [ ] Fixed-point division with rounding strategy
- [ ] Comparison operations (PartialOrd, Ord)
- [ ] Conversion from i64/i128 (no floats!)
- [ ] Serialization (serde) maintaining precision

**Example implementation:**
```rust
impl Fixed {
    pub const SCALE: i64 = 1_000_000; // 6 decimal places
    
    pub fn saturating_mul(self, rhs: Self) -> Self {
        let raw = (self.0 as i128 * rhs.0 as i128) / Self::SCALE as i128;
        Self(raw.saturating_cast::<i64>())
    }
    
    pub fn saturating_div(self, rhs: Self) -> Self {
        let raw = (self.0 as i128 * Self::SCALE as i128) / rhs.0 as i128;
        Self(raw.saturating_cast::<i64>())
    }
}
```

### 5. Add Cross-Platform Unit Tests

Create `crates/ai_core/tests/deterministic_math.rs`:

```rust
#[test]
fn test_fixed_mul_determinism() {
    let a = Fixed::from_raw(1_500_000); // 1.5
    let b = Fixed::from_raw(2_000_000); // 2.0
    let result = a.saturating_mul(b);
    
    // Must be exactly 3.0 on ALL platforms
    assert_eq!(result.to_raw(), 3_000_000);
}

#[test]
fn test_fixed_div_determinism() {
    let a = Fixed::from_raw(5_000_000); // 5.0
    let b = Fixed::from_raw(2_000_000); // 2.0
    let result = a.saturating_div(b);
    
    // Must be exactly 2.5 on ALL platforms
    assert_eq!(result.to_raw(), 2_500_000);
}

#[test]
fn test_overflow_behavior() {
    let max = Fixed::from_raw(i64::MAX);
    let result = max.saturating_mul(Fixed::from_raw(2_000_000));
    
    // Must saturate to MAX, not panic
    assert_eq!(result.to_raw(), i64::MAX);
}

#[cfg(target_arch = "x86_64")]
#[test]
fn test_x86_64_specific() {
    // Validate x86_64 SIMD doesn't affect results
}

#[cfg(target_arch = "aarch64")]
#[test]
fn test_aarch64_specific() {
    // Validate ARM NEON doesn't affect results
}
```

### 6. Remove Float Usage

**Files to clean:**
- `crates/ai_core/src/config.rs` - Replace f32/f64 thresholds
- `crates/ai_core/src/types.rs` - Convert float fields to Fixed
- `crates/ai_core/src/health.rs` - Use Fixed for metrics

**Strategy:**
1. Search: `rg "(f32|f64)" crates/ai_core/src/*.rs | grep -v test`
2. For each match:
   - If in struct field: change to `Fixed` type
   - If in function param: change to `Fixed` or `i64`
   - If in constant: convert to Fixed constant
3. Update tests to use Fixed constructors

### 7. Validation & Testing

**Pre-PR checklist:**
```bash
# Float detection (must be empty)
rg -n "(f32|f64)" crates/ai_core/src/*.rs | grep -v "tests\|test_" | grep -v "//"

# Build check
cargo build --package ippan-ai-core --release

# Run new tests
cargo test --package ippan-ai-core deterministic_math

# Run all ai_core tests
cargo test --package ippan-ai-core

# Check formatting
cargo fmt --package ippan-ai-core -- --check

# Check lints
cargo clippy --package ippan-ai-core -- -D warnings
```

### 8. Create Pull Request

```bash
git add crates/ai_core/src/fixed*.rs crates/ai_core/src/determinism.rs
git add crates/ai_core/tests/deterministic_math.rs
git add crates/ai_core/src/{config,types,health}.rs

git commit -m "$(cat <<'EOF'
Phase 1: Harden deterministic math foundation

- Replace all unchecked arithmetic with saturating operations
- Remove f32/f64 from runtime paths in ai_core
- Add cross-platform determinism unit tests
- Document overflow behavior and rounding strategies

Acceptance gates:
✅ Zero floats in non-test runtime code
✅ All tests pass
✅ Bit-identical results across platforms

Related: D-GBDT Rollout Phase 1
EOF
)"

git push -u origin phase1/deterministic-math

gh pr create \
  --base feat/d-gbdt-rollout \
  --title "Phase 1: Deterministic Math Foundation" \
  --body "$(cat <<'EOF'
## Summary
- Hardened fixed-point math for bit-identical cross-platform results
- Eliminated f32/f64 from ai_core runtime paths
- Added comprehensive determinism unit tests

## Changes
- `fixed.rs`: Saturating arithmetic, overflow handling
- `fixed_point.rs`: Conversion utilities without floats
- `determinism.rs`: Cross-platform validation helpers
- New tests: `tests/deterministic_math.rs`

## Acceptance Gates
- [x] Float check: `rg "(f32|f64)" crates/ai_core/src | grep -v test` → EMPTY
- [x] Build: `cargo build --package ippan-ai-core` → SUCCESS
- [x] Tests: `cargo test --package ippan-ai-core` → ALL PASS
- [x] Determinism: Cross-platform tests validate bit-identical outputs

## Next Phase
Phase 2 will use this foundation to refactor the inference engine.
EOF
)"
```

---

## 🚦 Acceptance Gates

Before requesting review:
- [ ] **Float check passes:** No f32/f64 in src/*.rs (excluding tests)
- [ ] **All tests pass:** `cargo test --package ippan-ai-core`
- [ ] **Lints clean:** `cargo clippy` with no warnings
- [ ] **Format clean:** `cargo fmt --check`
- [ ] **Determinism tests added:** At least 5 new unit tests
- [ ] **Documentation updated:** Doc comments explain overflow behavior

---

## 📚 References

- [Rust Saturating Arithmetic](https://doc.rust-lang.org/std/primitive.i64.html#method.saturating_add)
- [Fixed-Point Arithmetic](https://en.wikipedia.org/wiki/Fixed-point_arithmetic)
- [Deterministic Computing](https://gafferongames.com/post/deterministic_lockstep/)

---

**Estimated Effort:** 2-3 days  
**Priority:** P0 (blocking Phase 2)  
**Status:** Ready for assignment
