# IPPAN Economics

Deterministic round-based emission and fair distribution for IPPAN BlockDAG.

## Overview

This crate implements the core economics logic for the IPPAN blockchain, providing:

- **Deterministic Emission**: Round-based emission with halving schedule
- **Hard Cap Enforcement**: 21M IPN maximum supply with automatic clamping
- **Fair Distribution**: Role-weighted proportional distribution across validators
- **Fee Management**: Configurable fee caps per round
- **Precision**: Uses micro-IPN (μIPN) for exact calculations without floating point

## Key Features

### Monetary Unit
- **1 IPN = 1,000,000 μIPN** (micro-IPN)
- All calculations use `u128` for micro-IPN to avoid floating point precision issues
- Constants and conversion helpers provided

### Emission Schedule
- **Initial Reward**: 0.0001 IPN per round (100 μIPN)
- **Halving Interval**: ~2 years (630,720,000 rounds at 10 rounds/second)
- **Hard Cap**: 21,000,000 IPN total supply
- **Formula**: `R(t) = R0 / 2^floor(t / T_h)`

### Distribution Logic
- **Role Weights**: Proposers get 1.2x weight vs Verifiers (1.0x)
- **Proportional**: Based on number of micro-blocks contributed
- **Fee Cap**: Maximum 10% of round emission can come from fees
- **Fair**: All validators paid proportionally to their weighted contribution

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

## Configuration

The `EconomicsParams` struct allows configuration of:

- **Hard Cap**: Maximum total supply (default: 21M IPN)
- **Initial Reward**: Base reward per round (default: 0.0001 IPN)
- **Halving Interval**: Rounds between halvings (default: ~2 years)
- **Fee Cap**: Maximum fee percentage (default: 10%)
- **Role Weights**: Proposer vs Verifier weights (default: 1.2x vs 1.0x)

These parameters should be stored on-chain and only modifiable through governance.

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

Run the example:

```bash
cargo run --example basic_usage -p ippan_economics
```

## Dependencies

- `serde`: Serialization support for on-chain storage
- `thiserror`: Error handling

## License

Apache-2.0