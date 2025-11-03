# Migration Guide: From PoA/BFT to DLC

## Overview

This guide walks you through migrating your IPPAN node from traditional Proof-of-Authority (PoA) or Byzantine Fault Tolerant (BFT) consensus to **Deterministic Learning Consensus (DLC)**.

## Prerequisites

- IPPAN node version 0.1.0+
- Rust 1.70+
- 10 IPN for validator bond (if running a validator)

## Migration Steps

### 1. Update Configuration

**Before (PoA):**
```toml
[consensus]
slot_duration_ms = 100
finalization_interval_ms = 200
enable_ai_reputation = false
```

**After (DLC):**
```toml
[consensus]
model = "DLC"
temporal_finality_ms = 250
hashtimer_precision_us = 1
shadow_verifier_count = 3

[dlc]
enable_dgbdt_fairness = true
enable_shadow_verifiers = true
require_validator_bond = true
```

### 2. Update Node Initialization

**Before:**
```rust
use ippan_consensus::{PoAConfig, PoAConsensus};

let config = PoAConfig::default();
let mut consensus = PoAConsensus::new(config, storage, validator_id);
consensus.start().await?;
```

**After:**
```rust
use ippan_consensus::{DLCConfig, DLCIntegratedConsensus, dlc_config_from_poa};

// Option 1: Pure DLC
let dlc_config = DLCConfig::default();
let mut dlc = DLCConsensus::new(dlc_config, validator_id);
dlc.start().await?;

// Option 2: Integrated (recommended for gradual migration)
let poa_config = PoAConfig::default();
let poa = PoAConsensus::new(poa_config, storage, validator_id);
let dlc_config = dlc_config_from_poa(true, 250);
let mut integrated = DLCIntegratedConsensus::new(poa, dlc_config, validator_id);
integrated.start().await?;
```

### 3. Bond Validator Stake (If Running Validator)

```rust
use ippan_consensus::{BondingManager, VALIDATOR_BOND_AMOUNT};

let mut bonding_manager = BondingManager::new();
bonding_manager.add_bond(validator_id, VALIDATOR_BOND_AMOUNT)?;

// Verify bond
assert!(bonding_manager.has_valid_bond(&validator_id));
```

### 4. Update Environment Variables

```bash
# Add to your .env or export in shell
export CONSENSUS_MODE=DLC
export TEMPORAL_FINALITY_MS=250
export REQUIRE_VALIDATOR_BOND=true
export ENABLE_DGBDT_FAIRNESS=true
export ENABLE_SHADOW_VERIFIERS=true
```

### 5. Remove BFT-Specific Code

**Remove these imports (if present):**
```rust
// ❌ Remove
use ippan_consensus::bft::*;
use ippan_consensus::pbft::*;
use ippan_consensus::tendermint::*;

// ✅ Add instead
use ippan_consensus::{DLCConsensus, DGBDTEngine, ShadowVerifierSet};
```

**Remove voting logic:**
```rust
// ❌ Remove
if reached_quorum() {
    finalize_block();
}

// ✅ Use temporal finality instead
if should_close_round(round_start, finality_window_ms) {
    dlc.finalize_round(round_id).await?;
}
```

### 6. Update Block Validation

**Before:**
```rust
let is_valid = validate_block_signatures(&block)?;
if !is_valid {
    reject_block();
}
```

**After:**
```rust
// Primary + shadow validation
let is_valid = dlc.verify_block(&block).await?;

if !is_valid {
    reject_block();
}
// Shadow verifiers automatically check in parallel
```

### 7. Update Metrics and Monitoring

**Add DLC-specific metrics:**
```rust
// Reputation tracking
let metrics = ValidatorMetrics {
    blocks_proposed: /* ... */,
    blocks_verified: /* ... */,
    uptime_percentage: /* ... */,
    // ... other fields
};
dlc.update_validator_metrics(validator_id, metrics);

// Monitor shadow verifier consistency
let stats = shadow_verifiers.get_stats();
for (validator_id, (count, inconsistencies)) in stats {
    if inconsistencies > 0 {
        warn!("Validator {} has {} inconsistencies", 
              hex::encode(validator_id), inconsistencies);
    }
}
```

### 8. Test Migration

```bash
# Run DLC tests
cargo test --package ippan-consensus --test dlc_integration_tests

# Start node in test mode
cargo run --bin ippan-node -- --dev

# Verify DLC is active
curl http://localhost:8080/consensus/state | jq .
```

Expected output:
```json
{
  "consensus_mode": "DLC",
  "round_id": 42,
  "primary_verifier": "0x...",
  "shadow_verifiers": ["0x...", "0x...", "0x..."],
  "temporal_finality_ms": 250
}
```

## Rollback Plan

If you need to rollback to PoA:

1. **Stop the node:**
   ```bash
   pkill ippan-node
   ```

2. **Revert configuration:**
   ```bash
   git checkout HEAD~1 config/
   ```

3. **Restart with PoA:**
   ```bash
   export CONSENSUS_MODE=PoA
   cargo run --bin ippan-node
   ```

## Gradual Migration (Recommended)

Use `DLCIntegratedConsensus` for a gradual migration:

### Phase 1: Enable DLC alongside PoA
```rust
let dlc_config = dlc_config_from_poa(true, 250);
let integrated = DLCIntegratedConsensus::new(poa, dlc_config, validator_id);
// DLC runs but PoA still drives finality
```

### Phase 2: Switch finality to DLC
```rust
// Configure DLC to drive finality
let dlc_config = DLCConfig {
    enable_dgbdt_fairness: true,
    enable_shadow_verifiers: true,
    ..Default::default()
};
```

### Phase 3: Remove PoA completely
```rust
// Use pure DLC
let mut dlc = DLCConsensus::new(dlc_config, validator_id);
dlc.start().await?;
```

## Breaking Changes

### API Changes

| Old (PoA) | New (DLC) |
|-----------|-----------|
| `PoAConfig` | `DLCConfig` |
| `PoAConsensus::new()` | `DLCConsensus::new()` |
| `select_proposer()` | D-GBDT automatic selection |
| `validate_quorum()` | Shadow verifier consensus |
| `slot_duration_ms` | `temporal_finality_ms` |

### Configuration Changes

| Old Key | New Key | Default |
|---------|---------|---------|
| `slot_duration_ms` | `temporal_finality_ms` | 250 |
| `enable_ai_reputation` | `enable_dgbdt_fairness` | true |
| N/A | `shadow_verifier_count` | 3 |
| N/A | `require_validator_bond` | true |

### Behavioral Changes

1. **No more voting:** Rounds close deterministically after temporal window
2. **Bonding required:** Validators must bond 10 IPN
3. **Shadow verification:** All blocks verified by 3-5 validators
4. **D-GBDT selection:** Weighted deterministic selection instead of round-robin

## Troubleshooting

### Issue: "Validator bond required"

**Solution:**
```rust
bonding_manager.add_bond(validator_id, VALIDATOR_BOND_AMOUNT)?;
```

### Issue: "Shadow verifier inconsistency"

**Solution:**
1. Check network connectivity
2. Verify all nodes on same version
3. Review flagged validator logs
4. Consider slashing misbehaving validator

### Issue: "Temporal finality not closing rounds"

**Solution:**
1. Check system clock sync (NTP)
2. Verify HashTimer initialization
3. Confirm `temporal_finality_ms` setting
4. Review logs: `export RUST_LOG=ippan_consensus=debug`

## Post-Migration Checklist

- [ ] Configuration updated to DLC
- [ ] Validator bond posted (10 IPN)
- [ ] Node restarts successfully
- [ ] DLC mode confirmed in `/consensus/state`
- [ ] Shadow verifiers active
- [ ] Metrics reporting DLC data
- [ ] No BFT imports remain
- [ ] Tests passing
- [ ] Monitoring updated

## Support

For migration support:
- Discord: https://discord.gg/ippan
- GitHub Issues: https://github.com/dmrl789/IPPAN/issues
- Documentation: https://docs.ippan.network/dlc

## See Also

- [DLC_CONSENSUS.md](./DLC_CONSENSUS.md) - Full DLC specification
- [BEYOND_BFT_DETERMINISTIC_LEARNING_CONSENSUS.md](./BEYOND_BFT_DETERMINISTIC_LEARNING_CONSENSUS.md) - Whitepaper
- [API Reference](./API_REFERENCE.md)
