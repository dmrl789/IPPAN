# IPPAN Economics - DAG-Fair Emission Framework

Deterministic round-based emission and fair distribution for IPPAN BlockDAG.

## Overview

IPPAN's DAG-Fair Emission Framework transforms traditional block-based mining into **time-anchored micro-rewards**. Unlike linear blockchains where block intervals define issuance, IPPAN's BlockDAG creates thousands of micro-blocks per second within overlapping rounds. Rewards are computed **per round** and distributed proportionally to validators based on their participation.

This crate implements the core economics logic for the IPPAN blockchain, providing:

- **Deterministic Emission**: Round-based emission with Bitcoin-style halving schedule
- **Hard Cap Enforcement**: 21M IPN maximum supply with automatic clamping and burn
- **DAG-Fair Distribution**: Role-weighted proportional distribution across validators
- **Fee Management**: Configurable fee caps per round
- **Precision**: Uses micro-IPN (μIPN) for exact calculations without floating point
- **Parallel Simulation**: High-performance multi-core simulation using Rayon
- **Governance Controls**: On-chain parameter updates via validator voting

## Key Features

### Monetary Unit
- **1 IPN = 1,000,000 μIPN** (micro-IPN)
- All calculations use `u128` for micro-IPN to avoid floating point precision issues
- Constants and conversion helpers provided

### Emission Schedule
- **Initial Reward**: 0.0001 IPN per round (100 μIPN)
- **Halving Interval**: ~2 years (630,720,000 rounds at 10 rounds/second)
- **Hard Cap**: 21,000,000 IPN total supply
- **Formula**: `R(t) = R0 / 2^floor(t / T_h)` - Bitcoin-style deterministic halving

### Distribution Logic
- **Role Weights**: 
  - Proposer: 1.2× weight
  - Verifier: 1.0× weight
  - Observer: 0× weight (no rewards)
- **Proportional**: Based on number of micro-blocks contributed
- **Fee Cap**: Maximum 10% of round emission can come from fees
- **Fair**: All validators paid proportionally to their weighted contribution
- **Uptime Score**: Performance-based reward scaling

### Reward Composition

Each round's reward is distributed across four components:

- **Round Emission (60%)**: Base reward distributed per round
- **Transaction Fees (25%)**: Deterministic micro-fees per transaction
- **AI Service Commissions (10%)**: From inference and compute tasks
- **Network Reward Dividend (5%)**: Weekly redistribution by uptime × reputation

## Usage

### Basic Example

```rust
use ippan_economics::*;
use std::collections::HashMap;

// Create economics parameters
let params = EconomicsParams::default();

// Calculate emission for a round
let round = 1000;
let emission = emission_for_round(round, &params);

// Create participation set
let mut participation = HashMap::new();
participation.insert(
    ValidatorId("alice".to_string()),
    Participation { role: Role::Proposer, blocks: 5 },
);
participation.insert(
    ValidatorId("bob".to_string()),
    Participation { role: Role::Verifier, blocks: 10 },
);

// Distribute rewards
let fees = 50_000; // 0.05 IPN in μIPN
let (payouts, emission_paid, fees_paid) = distribute_round(
    emission,
    fees,
    &participation,
    &params,
)?;

// Process payouts
for (validator, amount) in payouts {
    println!("{}: {} μIPN", validator.0, amount);
}
```

### Integration with Consensus

```rust
use ippan_economics::*;

fn settle_round(
    round: u64,
    already_issued: u128,
    fees: u128,
    participation: ParticipationSet,
    params: &EconomicsParams,
) -> Result<(), EcoError> {
    // Compute capped emission
    let emission = emission_for_round_capped(round, already_issued, params)?;
    
    // Distribute rewards
    let (payouts, _, _) = distribute_round(emission, fees, &participation, params)?;
    
    // Apply payouts to validator balances
    for (validator, amount) in payouts {
        // credit_validator_balance(validator, amount);
    }
    
    Ok(())
}
```

### Epoch Verification

```rust
use ippan_economics::*;

// At end of epoch, verify total emission
let expected_emission = sum_emission_over_rounds(
    epoch_start, 
    epoch_end, 
    |r| emission_for_round(r, &params)
);

let actual_minted = get_total_minted_from_chain();
let burn_amount = epoch_auto_burn(expected_emission, actual_minted);

if burn_amount > 0 {
    // Auto-burn excess tokens
    burn_tokens(burn_amount);
}
```

## Core Components

### EmissionEngine
Calculates round rewards using the halving formula with supply cap enforcement:

```rust
use ippan_economics::prelude::*;

let mut engine = EmissionEngine::new();
let reward = engine.calculate_round_reward(1000)?;
```

### RoundRewards
Distributes rewards proportionally among validators based on role, blocks contributed, and uptime:

```rust
let round_rewards = RoundRewards::new(emission_params);
let distribution = round_rewards.distribute_round_rewards(
    round_index,
    round_reward,
    participations,
    fees_collected,
)?;
```

### SupplyTracker
Monitors total supply and enforces the 21M IPN cap:

```rust
let mut tracker = SupplyTracker::new(2_100_000_000_000); // 21M IPN in micro-IPN
tracker.record_emission(round, amount)?;
let supply_info = tracker.get_supply_info();
```

### GovernanceParams
Enables on-chain parameter updates through validator voting:

```rust
let mut governance = GovernanceParams::new(emission_params);
let proposal_id = governance.create_proposal(
    proposer,
    new_params,
    voting_period,
    justification,
    current_round,
)?;
```

## Examples

### Basic Usage

Run the basic example demonstrating emission and distribution:

```bash
cargo run --example basic_usage -p ippan_economics
```

### Parallel Emission Simulator

Simulates **10,000 rounds** of emission and validator participation using all CPU cores:

```bash
# Basic simulation (CSV output only)
cargo run --package ippan_economics --example parallel_emission_sim

# With chart generation (may require fonts)
cargo run --package ippan_economics --example parallel_emission_sim --features plotters
```

**Output:**
- `emission_data.csv` - Per-round emission, supply, and halving data
- `emission_curve.png` - Visualization of emission curve (if plotters feature enabled)

**Example output:**

```
🚀 Starting IPPAN parallel emission simulation over 10000 rounds
✅ Simulation complete: issued=498750 μIPN (≈ 0.499 IPN), burned=0 μIPN
⚖️  Validator reward distribution:
   min=48234 μIPN, max=51023 μIPN, avg=49875.0 μIPN
   fairness ratio = 1.06× (max/min)
```

**CSV Output Format:**

```csv
round,emission_micro,total_supply_micro,halving_index
0,0,0,0
1,100,100,0
2,100,200,0
...
```

### DAG-Fair Emission Demo

Run the demo to see the framework in action:

```bash
cargo run --example dag_fair_emission_demo
```

## Configuration

The `EconomicsParams` struct allows configuration of:

| Parameter | Default Value | Description |
|-----------|---------------|-------------|
| **Hard Cap** | 21M IPN (2.1e12 μIPN) | Maximum total supply |
| **Initial Reward** | 0.0001 IPN (100 μIPN) | Base reward per round |
| **Halving Interval** | 630,720,000 rounds | ~2 years at 10 rounds/second |
| **Fee Cap** | 10% | Maximum fee percentage of emission |
| **Proposer Weight** | 1.2× | Proposer reward multiplier |
| **Verifier Weight** | 1.0× | Verifier reward multiplier |

These parameters should be stored on-chain and only modifiable through governance.

## Governance

All emission parameters can be updated through on-chain governance:

1. **Proposal Creation**: Validators create proposals with new parameters
2. **Voting Period**: Other validators vote (approve/reject/abstain)
3. **Threshold Check**: Requires 66% supermajority approval
4. **Execution**: Approved proposals update parameters immediately

## Error Handling

The crate provides specific error types:

- `EcoError::HardCapExceeded`: When emission would exceed total supply
- `EcoError::FeeCapExceeded`: When fees exceed allowed percentage
- `EcoError::NoBlocksInRound`: When no participation recorded

## Testing

Run the test suite:

```bash
cargo test -p ippan_economics
```

Run benchmarks to test performance:

```bash
cargo bench -p ippan_economics
```

## Architecture

The crate is organized into modular components:

- `emission`: Per-round emission calculation with halving schedule
- `distribution`: Fair reward distribution across validators
- `params`: Economic parameters and configuration
- `types`: Core types (ValidatorId, Participation, Payouts, etc.)
- `errors`: Error types for economic operations
- `verify`: Supply and distribution verification
- `governance`: On-chain parameter management

## Dependencies

- `serde`: Serialization support for on-chain storage
- `thiserror`: Error handling
- `ippan-types`: Core IPPAN types
- `ippan-time`: HashTimer-based round indexing

### Dev Dependencies (for examples)

- `rand`: Random number generation for simulations
- `rayon`: Parallel execution for multi-core simulations
- `csv`: CSV output generation
- `plotters`: Chart generation (optional)
- `image`: Image format support for charts
- `rust_decimal`: Decimal arithmetic

## Performance

The framework is optimized for high-throughput scenarios:

- Round reward calculation: ~100ns
- Reward distribution: ~1μs per validator
- Supply tracking: ~10ns per operation
- Governance voting: ~100ns per vote

## Security Considerations

- All calculations use deterministic arithmetic to prevent consensus forks
- Supply cap enforcement prevents inflation beyond 21M IPN
- Fee caps prevent economic centralization
- Comprehensive auditing ensures supply integrity
- Governance requires supermajority approval for parameter changes

## Notes

- All emission calculations are deterministic and reproducible
- The parallel simulator uses per-thread RNG seeding for reproducibility
- Chart generation gracefully handles headless/CI environments
- CSV output is always generated regardless of plotting success
- Fairness ratio measures max/min validator rewards (closer to 1.0 is more fair)

## Integration

This crate integrates with other IPPAN components:

- **`ippan_time`**: HashTimer-based round indexing
- **`ippan_types`**: Core data structures
- **`ippan_governance`**: Parameter update mechanisms
- **`ippan_consensus`**: Round settlement and validator management

## License

Apache-2.0
