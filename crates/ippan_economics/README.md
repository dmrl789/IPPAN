# IPPAN Economics

Comprehensive economic modeling and simulation for the IPPAN blockchain.

## Features

- **Emission schedules** with Bitcoin-style halving
- **Validator reward distribution** (proposer/verifier split)
- **Fee recycling** and burning mechanisms
- **Supply projection** and fairness analysis
- **High-performance parallel simulation** using Rayon

## Usage

### As a Library

```rust
use ippan_economics::*;

let params = EconomicsParams::default();
let emission = emission_for_round(1000, &params);

let mut participants = ParticipationSet::new();
participants.insert(
    ValidatorId("validator1".to_string()),
    Participation { role: Role::Proposer, blocks: 2 },
);

let (payouts, distributed, _) = 
    distribute_round(100_000, 0, &participants, &params)?;
```

## Examples

### Parallel Emission Simulator

Simulates **10,000 rounds** of emission and validator participation using all CPU cores:

```bash
# Basic simulation (CSV output only)
cargo run --example parallel_emission_sim

# With chart generation (requires fonts in environment)
cargo run --example parallel_emission_sim --features plotters
```

**Output:**
- `emission_data.csv` - Per-round emission, supply, and halving data
- `emission_curve.png` - Visualization of emission curve (if plotters feature enabled)

**Example output:**

```
🚀 Starting IPPAN parallel emission simulation over 10000 rounds
✅ Simulation complete: issued=1925687261 μIPN (≈ 1925.687 IPN), burned=1131273 μIPN
⚖️  Validator reward distribution:
   min=19110875 μIPN, max=19562739 μIPN, avg=19256872.7 μIPN
   fairness ratio = 1.02× (max/min)
```

## Economic Parameters

Default configuration:
- **Initial emission**: 0.5 IPN per round (50M μIPN)
- **Halving interval**: 315M rounds (~2 years at 200ms rounds)
- **Supply cap**: 21M IPN
- **Proposer share**: 20%
- **Verifier share**: 80%
- **Burn rate**: 100% of unused emission

## CSV Output Format

```csv
round,emission_micro,total_supply_micro,halving_index
0,0,0,0
1,50000000,50000000,0
2,49999999,99999999,0
...
```

## Notes

- The simulator uses deterministic per-thread RNG for reproducibility
- Chart generation requires font support (may fail in headless/CI environments)
- CSV output is always generated regardless of plotting success
- Fairness ratio measures max/min validator rewards (lower is more fair)

## License

Apache-2.0
