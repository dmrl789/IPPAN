# IPPAN Economics

Deterministic round-based emission and fair distribution for IPPAN BlockDAG.

## Features

- **DAG-Fair Emission**: Bitcoin-style halving with hard cap enforcement
- **Role-weighted Distribution**: Proportional rewards for proposers and verifiers
- **Fee Cap Enforcement**: Automatic validation of fee limits
- **Supply Verification**: Deterministic supply tracking and projection
- **Parallel Simulation**: High-performance multi-core simulation using Rayon

## Architecture

The crate is organized into modular components:

- `emission`: Per-round emission calculation with halving schedule
- `distribution`: Fair reward distribution across validators
- `params`: Economic parameters and configuration
- `types`: Core types (ValidatorId, Participation, Payouts, etc.)
- `errors`: Error types for economic operations
- `verify`: Supply and distribution verification

## Usage

### As a Library

```rust
use ippan_economics::*;

// Configure economic parameters
let params = EconomicsParams::default();

// Calculate emission for a specific round
let round = 1000;
let emission = emission_for_round(round, &params);

// Set up validator participation
let mut participants = ParticipationSet::new();
participants.insert(
    ValidatorId("validator1".to_string()),
    Participation { role: Role::Proposer, blocks: 2 },
);
participants.insert(
    ValidatorId("validator2".to_string()),
    Participation { role: Role::Verifier, blocks: 1 },
);

// Distribute rewards
let fees_collected = 100;
let (payouts, emission_paid, fees_paid) = 
    distribute_round(emission, fees_collected, &participants, &params)?;
```

## Examples

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

## Economic Parameters

Default configuration (per PRD):
- **Hard cap**: 21M IPN (21,000,000,000,000 μIPN)
- **Initial emission**: 0.0001 IPN per round (100 μIPN)
- **Halving interval**: 630.72M rounds (~2 years at 10 rounds/sec)
- **Fee cap**: 10% of per-round emission
- **Proposer weight**: 1.2× (120%)
- **Verifier weight**: 1.0× (100%)

### Monetary Unit

- **1 IPN = 1,000,000 μIPN** (micro-IPN)
- Similar to Bitcoin's satoshi system
- All internal calculations use μIPN for precision

## CSV Output Format

```csv
round,emission_micro,total_supply_micro,halving_index
0,0,0,0
1,100,100,0
2,100,200,0
...
```

## Testing

```bash
# Run unit tests
cargo test --package ippan_economics

# Run integration tests
cargo test --package ippan_economics --test basic
```

## Notes

- All emission calculations are deterministic and reproducible
- The simulator uses per-thread RNG seeding for parallel execution
- Chart generation gracefully handles headless/CI environments
- CSV output is always generated regardless of plotting success
- Fairness ratio measures max/min validator rewards (closer to 1.0 is more fair)

## License

Apache-2.0
