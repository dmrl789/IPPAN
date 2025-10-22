# Fee Caps and Emission Schedule

This document specifies the IPPAN protocol's fee caps, emission schedule, and reward distribution.

## Fee Caps

### Overview

IPPAN enforces **hard fee caps** at the protocol level to prevent fee market manipulation and ensure predictable costs. Transactions exceeding the cap are rejected during mempool admission and block assembly.

### Cap Values (µIPN)

| Transaction Type | Cap (µIPN) | Cap (IPN) | USD Equivalent* |
|------------------|------------|-----------|-----------------|
| Transfer         | 1,000      | 0.00001   | ~$0.0001        |
| AI Call          | 100        | 0.000001  | ~$0.00001       |
| Contract Deploy  | 100,000    | 0.001     | ~$0.01          |
| Contract Call    | 10,000     | 0.0001    | ~$0.001         |
| Governance       | 10,000     | 0.0001    | ~$0.001         |
| Validator Ops    | 10,000     | 0.0001    | ~$0.001         |

*Assuming $10/IPN, illustrative only.

### Enforcement

```rust
// In mempool admission
if tx.fee > fee_cap_for_type(tx.type) {
    return Err(FeeError::FeeAboveCap);
}

// In block assembly
for tx in block.transactions {
    validate_fee(&tx, tx.fee, &fee_config)?;
}
```

### Fee Recycling

Collected fees are accumulated and recycled to the reward pool weekly:

- **Collection Period**: 1 week (~3,024,000 rounds at 200ms/round)
- **Recycle Percentage**: 100% (configurable via governance)
- **Distribution**: Added to next week's reward pool, split per usual proposer/verifier allocation

## Emission Schedule

### Parameters

| Parameter           | Value                      | Notes                                    |
|---------------------|----------------------------|------------------------------------------|
| **R0**              | 10,000 µIPN                | Initial reward per round                 |
| **Halving Rounds**  | 315,000,000                | ~2 years at 200ms/round                  |
| **Supply Cap**      | 21,000,000 IPN             | 2.1e15 µIPN                              |
| **Proposer Share**  | 20% (2000 bps)             | Reward for block proposer                |
| **Verifier Share**  | 80% (8000 bps)             | Split equally among verifiers            |

### Halving Schedule

Reward per round:

```
R(round) = R0 >> (round / halving_rounds)
```

Example:

| Period      | Rounds            | Reward/Round | Notes          |
|-------------|-------------------|--------------|----------------|
| 0-315M      | 0 - 315,000,000   | 10,000 µIPN  | First 2 years  |
| 315M-630M   | 315M - 630M       | 5,000 µIPN   | Years 2-4      |
| 630M-945M   | 630M - 945M       | 2,500 µIPN   | Years 4-6      |
| ...         | ...               | ...          | ...            |
| After ~128y | > 20.16B rounds   | 0 µIPN       | Tail emission  |

### Distribution

For each round with `N` blocks and `V` verifiers:

1. **Total Reward**: `R = R(round)`
2. **Per Block**:
   - Proposer: `R * 0.20 / N`
   - Verifier Pool: `R * 0.80 / N`
   - Per Verifier: `(R * 0.80 / N) / V`

Example (1 block, 4 verifiers, round 1):

- Total: 10,000 µIPN
- Proposer: 2,000 µIPN
- Verifier Pool: 8,000 µIPN
- Each Verifier: 2,000 µIPN

### Supply Curve

```python
def projected_supply(rounds):
    total = 0
    halvings = 0
    while halvings < 64:
        start = halvings * 315_000_000 + 1
        end = min((halvings + 1) * 315_000_000, rounds)
        if start > rounds:
            break
        reward = 10_000 >> halvings
        total += reward * (end - start + 1)
        halvings += 1
    return min(total, 21_000_000_00000000)  # µIPN
```

| Milestone        | Rounds      | Supply (IPN)    | % of Cap |
|------------------|-------------|-----------------|----------|
| 1 year           | 157.5M      | ~1,575,000      | 7.5%     |
| 2 years          | 315M        | ~3,150,000      | 15%      |
| 4 years          | 630M        | ~4,725,000      | 22.5%    |
| 10 years         | 1.575B      | ~9,140,625      | 43.5%    |
| 20 years         | 3.15B       | ~14,765,625     | 70.3%    |
| 50 years         | 7.875B      | ~19,531,250     | 93%      |
| ∞ (tail)         | ∞           | 21,000,000      | 100%     |

## Reward Accounting

### On-Chain State

```rust
struct RewardPool {
    accumulated_fees: u128,        // µIPN from fees
    last_recycle_round: u64,
    total_emitted: u128,           // Total minted
    total_recycled: u128,          // Total recycled fees
}
```

### Per-Round Distribution

1. Calculate `R(round)`
2. Add recycled fees (if recycle round)
3. Distribute:
   ```rust
   for block in round.blocks {
       proposer_reward = (total_reward * 2000) / 10000;
       verifier_reward = (total_reward * 8000) / 10000 / verifier_count;
       
       mint(block.proposer, proposer_reward);
       for verifier in verifiers {
           mint(verifier, verifier_reward);
       }
   }
   ```

## Examples

### Scenario 1: High Activity Round

- Round: 1,000,000
- Blocks: 10
- Verifiers: 50
- Fees Collected (this round): 50,000 µIPN
- Reward: 10,000 µIPN (before halving)

Distribution:

- Total Pool: 10,000 µIPN
- Per Block: 1,000 µIPN
- Proposer (each): 200 µIPN (20% of 1,000)
- Verifier Pool (total): 8,000 µIPN
- Per Verifier: 160 µIPN (8,000 / 50)

### Scenario 2: Fee Recycling Round

- Round: 3,024,000 (1 week)
- Accumulated Fees: 10,000,000 µIPN
- Emission Reward: 10,000 µIPN
- Recycled: 10,000,000 µIPN (100%)

Total Pool: 10,010,000 µIPN

### Scenario 3: Post-Halving

- Round: 315,000,001
- Emission Reward: 5,000 µIPN (first halving)
- Rest same as Scenario 1

## Governance Parameters

Adjustable via on-chain governance:

| Parameter                 | Default       | Range           | Requires       |
|---------------------------|---------------|-----------------|----------------|
| `recycle_bps`             | 10000 (100%)  | 0 - 10000       | Majority vote  |
| `proposer_bps`            | 2000 (20%)    | 1000 - 5000     | Supermajority  |
| `verifier_bps`            | 8000 (80%)    | 5000 - 9000     | Supermajority  |
| Fee caps (individual)     | See table     | -50% to +200%   | Majority vote  |

**Note**: Halving schedule and R0 are immutable.

## Security Considerations

### Fee Cap Bypass Attempts

Prevented by:

1. Mempool validation
2. Block assembly validation
3. Block verification by all nodes
4. Slashing for proposing over-cap txs

### Inflation Attack

Not possible: emission schedule is deterministic and capped.

### Fee Recycling Manipulation

Mitigated:

- Weekly recycle (hard-coded interval)
- Recycle percentage controlled by governance
- All fees tracked on-chain with auditable history

## Implementation References

- [Emission Module](../crates/consensus/src/emission.rs)
- [Fee Caps Module](../crates/consensus/src/fees.rs)
- [Consensus Integration](../crates/consensus/src/lib.rs)

## Changelog

- **2025-10-22**: Initial emission and fee specification
