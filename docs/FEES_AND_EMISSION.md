# Fees and Emission System

This document describes the fee caps and DAG-Fair emission system implemented in IPPAN.

## Overview

The IPPAN blockchain implements a comprehensive fee and emission system designed to:
- Prevent fee spam through hard caps
- Ensure fair reward distribution
- Provide predictable token economics
- Enable sustainable network operation

## Fee System

### Fee Caps

Transaction fees are capped based on transaction type to prevent spam and ensure accessibility:

| Transaction Type | Fee Cap (micro-IPN) | Description |
|------------------|---------------------|-------------|
| Transfer | 1,000 | Simple token transfers |
| AI Call | 100 | AI model inference calls |
| Governance | 10,000 | Governance proposals and votes |
| Validator Registration | 100,000 | New validator registration |
| Contract Deployment | 50,000 | Smart contract deployment |
| Contract Execution | 5,000 | Smart contract function calls |

### Fee Calculation

Fees are calculated as:
```
fee = base_fee + size_fee
```

Where:
- `base_fee`: Type-specific base fee
- `size_fee`: 1 micro-IPN per 100 bytes

### Fee Validation

All transactions are validated against fee caps before inclusion in blocks:

```rust
use ippan_consensus::fees::*;

let caps = FeeCaps::default();
validate_transaction_fee(TransactionType::Transfer, 500, &caps)?;
```

## DAG-Fair Emission

### Overview

The DAG-Fair emission system distributes rewards based on round participation rather than block production, ensuring fair compensation for all network contributors.

### Emission Parameters

- **Base Reward (R0)**: 10,000 micro-IPN per round
- **Halving Interval**: 2,102,400 rounds (~2 years)
- **Proposer Bonus**: 20% of round reward
- **Verifier Reward**: 80% of round reward
- **Minimum Reward**: 1 micro-IPN per round

### Reward Calculation

#### Round Reward

The reward for round `t` is calculated as:

```
R(t) = R0 >> halvings
```

Where `halvings = t / HALVING_ROUNDS`

#### Total Supply

The total supply at round `t` is:

```
S(t) = Σ(i=0 to t) R(i)
```

### Reward Distribution

#### Per Block Distribution

For each block in a round:
- **Proposer**: 20% of block reward
- **Verifiers**: 80% of block reward (split equally)

#### Example

Round with 2 blocks, 1000 micro-IPN total reward:
- Block 1: Proposer gets 100, Verifiers get 400 (200 each)
- Block 2: Proposer gets 100, Verifiers get 400 (200 each)

## Fee Recycling

### Overview

Collected fees are periodically recycled back to the reward pool to maintain token supply and prevent deflation.

### Recycling Parameters

- **Interval**: 1,008 rounds (~1 week)
- **Percentage**: 80% of collected fees
- **Minimum**: 1,000,000 micro-IPN to trigger recycling

### Recycling Process

1. Collect fees from transactions
2. Check if recycling interval has passed
3. If minimum amount reached, recycle percentage
4. Add recycled amount to reward pool

## Implementation

### Fee Manager

```rust
use ippan_consensus::fees::*;

let mut fee_manager = FeeManager::new(
    FeeCaps::default(),
    FeeRecyclingConfig::default()
);

// Validate transaction fee
fee_manager.validate_fee(TransactionType::Transfer, 500)?;

// Collect fee
fee_manager.collect_fee(500);

// Process recycling
let recycled = fee_manager.process_recycling(current_round)?;
```

### Emission Calculator

```rust
use ippan_consensus::emission::*;

let params = EmissionParams::default();
let calculator = EmissionCalculator::new(params);

// Calculate round reward
let reward = calculator.calculate_round_reward(1000);

// Distribute rewards
let context = RoundContext { /* ... */ };
let distribution = calculator.distribute_rewards(&context)?;
```

## Economic Model

### Token Supply

- **Maximum Supply**: 21,000,000 IPN
- **Decimals**: 8 (1 IPN = 100,000,000 micro-IPN)
- **Halving**: Every ~2 years
- **Final Reward**: 1 micro-IPN per round

### Inflation Schedule

| Period | Rounds | Reward per Round | Annual Inflation |
|--------|--------|------------------|------------------|
| Year 1-2 | 0-2,102,400 | 10,000 | ~5.26% |
| Year 3-4 | 2,102,401-4,204,800 | 5,000 | ~2.63% |
| Year 5-6 | 4,204,801-6,307,200 | 2,500 | ~1.32% |
| ... | ... | ... | ... |

### Fee Economics

- **Base Fee**: Covers transaction processing costs
- **Size Fee**: Prevents spam through large transactions
- **Type-based**: Different costs for different operations
- **Recycling**: Maintains supply and prevents deflation

## Security Considerations

### Fee Spam Prevention

- Hard caps prevent excessive fees
- Size-based fees prevent large transaction spam
- Type-based caps prevent specific attack vectors

### Economic Security

- Predictable emission schedule
- Fair distribution mechanism
- Recycling prevents deflation
- Minimum rewards ensure network participation

### Governance

- Fee caps can be updated through governance
- Recycling parameters are configurable
- Emission parameters can be modified
- All changes require stakeholder approval

## Configuration

### Default Settings

```rust
// Fee Caps
FeeCaps {
    transfer: 1_000,
    ai_call: 100,
    governance: 10_000,
    validator_registration: 100_000,
    contract_deployment: 50_000,
    contract_execution: 5_000,
}

// Emission Parameters
EmissionParams {
    r0: 10_000,
    halving_rounds: 2_102_400,
    proposer_bonus: 0.20,
    verifier_reward: 0.80,
    min_reward: 1,
}

// Recycling Configuration
FeeRecyclingConfig {
    recycling_interval: 1008,
    recycling_percentage: 0.8,
    min_recycling_amount: 1_000_000,
}
```

### Updating Parameters

Parameters can be updated through governance:

1. Submit parameter change proposal
2. Stakeholders vote on the change
3. If approved, parameters are updated
4. Changes take effect at specified round

## Monitoring

### Metrics

- Total fees collected
- Recycling amounts
- Round rewards distributed
- Validator reward distribution
- Fee cap utilization

### Alerts

- Fee cap violations
- Recycling failures
- Unusual fee patterns
- Reward distribution issues

## Best Practices

### For Validators

- Monitor fee collection
- Ensure proper reward distribution
- Participate in governance
- Report anomalies

### For Users

- Use appropriate transaction types
- Consider fee caps when designing transactions
- Monitor network fees
- Participate in governance

### For Developers

- Implement proper fee validation
- Use appropriate transaction types
- Consider fee implications
- Test with different fee scenarios

## Troubleshooting

### Common Issues

1. **Fee Cap Exceeded**: Reduce transaction size or use different type
2. **Recycling Not Working**: Check minimum amount and interval
3. **Reward Distribution Issues**: Verify round context and parameters
4. **Parameter Update Failed**: Check governance requirements

### Debugging

- Enable detailed logging
- Check parameter values
- Verify transaction validation
- Monitor network metrics