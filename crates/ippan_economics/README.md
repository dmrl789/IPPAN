# IPPAN Economics - DAG-Fair Emission Framework

This crate implements the deterministic round-based token economics for IPPAN, providing fair emission distribution across the BlockDAG structure.

## Overview

IPPAN's DAG-Fair Emission Framework transforms traditional block-based mining into **time-anchored micro-rewards**. Unlike linear blockchains where block intervals define issuance, IPPAN's BlockDAG creates thousands of micro-blocks per second within overlapping rounds. Rewards are computed **per round** and distributed proportionally to validators based on their participation.

## Key Features

- **Round-based emission**: Rewards calculated per round, not per block
- **DAG-Fair distribution**: Proportional rewards based on validator participation
- **Deterministic halving**: Bitcoin-style halving schedule with round precision
- **Supply integrity**: Hard-capped 21M IPN with automatic burn of excess
- **Governance controls**: On-chain parameter updates via validator voting
- **Comprehensive auditing**: Supply verification and integrity checks

## Core Components

### EmissionEngine
Calculates round rewards using the halving formula: `R(t) = R₀ / 2^(⌊t/Tₕ⌋)`

```rust
use ippan_economics::prelude::*;

let mut engine = EmissionEngine::new();
let reward = engine.calculate_round_reward(1000)?;
```

### RoundRewards
Distributes rewards proportionally among validators based on:
- Role (Proposer: 1.2×, Verifier: 1.0×, Observer: 0×)
- Blocks contributed
- Uptime score

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

## Usage Example

```rust
use ippan_economics::prelude::*;
use rust_decimal::Decimal;

// Create emission engine
let mut emission_engine = EmissionEngine::new();

// Calculate reward for a round
let round_reward = emission_engine.calculate_round_reward(1000)?;

// Create validator participations
let participations = vec![
    ValidatorParticipation {
        validator_id: "validator_1".to_string(),
        role: ValidatorRole::Proposer,
        blocks_contributed: 15,
        uptime_score: Decimal::new(95, 2), // 0.95
    },
    ValidatorParticipation {
        validator_id: "validator_2".to_string(),
        role: ValidatorRole::Verifier,
        blocks_contributed: 12,
        uptime_score: Decimal::new(98, 2), // 0.98
    },
];

// Distribute rewards
let round_rewards = RoundRewards::new(emission_engine.params().clone());
let distribution = round_rewards.distribute_round_rewards(
    1000,
    round_reward,
    participations,
    500, // fees
)?;

// Track supply
let mut supply_tracker = SupplyTracker::new(emission_engine.params().total_supply_cap);
supply_tracker.record_emission(1000, round_reward)?;
```

## Emission Parameters

| Parameter | Default Value | Description |
|-----------|---------------|-------------|
| `initial_round_reward` | 10,000 micro-IPN | Base reward per round (0.0001 IPN) |
| `halving_interval` | 630,000,000 rounds | Halving schedule (~2 years at 10 rounds/second) |
| `total_supply_cap` | 2,100,000,000,000 micro-IPN | Hard cap (21M IPN) |
| `fee_cap_fraction` | 0.1 | Maximum fees as fraction of round reward (10%) |

## Reward Composition

Each round's reward is distributed across four components:

- **Round Emission (60%)**: Base reward distributed per round
- **Transaction Fees (25%)**: Deterministic micro-fees per transaction
- **AI Service Commissions (10%)**: From inference and compute tasks
- **Network Reward Dividend (5%)**: Weekly redistribution by uptime × reputation

## Governance

All emission parameters can be updated through on-chain governance:

1. **Proposal Creation**: Validators create proposals with new parameters
2. **Voting Period**: Other validators vote (approve/reject/abstain)
3. **Threshold Check**: Requires 66% supermajority approval
4. **Execution**: Approved proposals update parameters immediately

## Testing

Run the comprehensive test suite:

```bash
cargo test
```

Run the demo to see the framework in action:

```bash
cargo run --example dag_fair_emission_demo
```

Run benchmarks to test performance:

```bash
cargo bench
```

## Integration

This crate integrates with other IPPAN components:

- **`ippan_time`**: HashTimer-based round indexing
- **`ippan_types`**: Core data structures
- **`ippan_governance`**: Parameter update mechanisms

## Security Considerations

- All calculations use deterministic arithmetic to prevent consensus forks
- Supply cap enforcement prevents inflation beyond 21M IPN
- Fee caps prevent economic centralization
- Comprehensive auditing ensures supply integrity
- Governance requires supermajority approval for parameter changes

## Performance

The framework is optimized for high-throughput scenarios:

- Round reward calculation: ~100ns
- Reward distribution: ~1μs per validator
- Supply tracking: ~10ns per operation
- Governance voting: ~100ns per vote

## License

This crate is part of the IPPAN project and is licensed under Apache-2.0.
