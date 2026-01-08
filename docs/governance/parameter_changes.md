# Fee Schedule Governance

This document describes the governance process for updating fee schedules and registry prices in IPPAN.

## Overview

Fee schedules are governance-controlled parameters that can be updated through on-chain proposals. The system enforces:

- **Timelock**: Minimum 2 epochs (14 days) notice before activation
- **Rate limits**: Maximum ±20% change per epoch
- **Minimum floors**: Base fees must be >= 1 kambei
- **Quorum + Supermajority**: Voting thresholds for approval

## Proposal Types

### 1. Fee Schedule Proposal

Updates the transaction fee computation parameters:

```rust
pub struct FeeScheduleProposal {
    pub proposal_id: [u8; 32],
    pub proposer: [u8; 32],
    pub new_schedule: FeeScheduleV1,
    pub current_schedule: FeeScheduleV1,
    pub created_epoch: Epoch,
    pub activation_epoch: Epoch,
    pub reason_hash: [u8; 32],
    pub status: ProposalStatus,
    pub votes: VoteTally,
}
```

### 2. Registry Price Proposal

Updates handle/domain registration prices:

```rust
pub struct RegistryPriceProposal {
    pub proposal_id: [u8; 32],
    pub proposer: [u8; 32],
    pub new_schedule: RegistryPriceScheduleV1,
    pub current_schedule: RegistryPriceScheduleV1,
    pub created_epoch: Epoch,
    pub activation_epoch: Epoch,
    pub reason_hash: [u8; 32],
    pub status: ProposalStatus,
    pub votes: VoteTally,
}
```

## Governance Constants

| Constant | Value | Description |
|----------|-------|-------------|
| NOTICE_EPOCHS | 2 | Minimum epochs (typically 2 weeks) between creation and activation |
| QUORUM_BPS | 6000 | **60%** of voting power must participate |
| APPROVAL_BPS | 6667 | **2/3 (66.67%)** of decisive votes must be "yes" |
| MAX_CONCURRENT_PROPOSALS | 5 | Maximum pending proposals |

## Timelock Requirement

Proposals must respect a minimum notice period:

```rust
// Validation check
if activation_epoch < created_epoch + NOTICE_EPOCHS {
    return Err("Insufficient notice period");
}
```

This ensures:
- **2 epochs (typically 2 weeks)** for network participants to review
- Time for validators to prepare
- Protection against rushed malicious changes

## Rate Limits

Each fee field can only change by ±20% per epoch:

```rust
fn check_rate_limit(old: u64, new: u64) -> Result<()> {
    let upper_bound = old * 12 / 10;  // +20%
    let lower_bound = old * 8 / 10;   // -20%
    
    if new > upper_bound || new < lower_bound {
        return Err("Rate limit exceeded");
    }
    Ok(())
}
```

This prevents:
- Sudden dramatic fee increases
- Economic shock to users
- Governance attacks via rapid parameter changes

## Minimum Floors

Fee schedules must meet minimum requirements:

| Parameter | Minimum |
|-----------|---------|
| tx_base_fee | 1 kambei |
| tx_min_fee | 1 kambei |
| handle_register_fee | 1 kambei |
| domain_register_fee | 1 kambei |

## Voting Process

### Vote Choices

```rust
pub enum VoteChoice {
    Yes,
    No,
    Abstain,
}
```

### Quorum Check

```rust
pub fn has_quorum(&self, total_voting_power: u64) -> bool {
    let participation = total_participated() * 10000 / total_voting_power;
    participation >= QUORUM_BPS
}
```

### Approval Check (2/3 Supermajority)

```rust
pub fn has_approval(&self) -> bool {
    let decisive = yes_power + no_power;
    if decisive == 0 { return false; }
    let yes_bps = yes_power * 10000 / decisive;
    yes_bps >= APPROVAL_BPS  // 6667 = 2/3
}
```

## Proposal Lifecycle

```
┌────────────┐   Submit   ┌─────────┐   Pass    ┌────────┐   Activate  ┌───────────┐
│   Draft    │ ─────────> │ Voting  │ ────────> │ Passed │ ──────────> │ Activated │
└────────────┘            └─────────┘           └────────┘             └───────────┘
                               │                    │
                               │ Reject             │ Expire
                               v                    v
                          ┌──────────┐        ┌─────────┐
                          │ Rejected │        │ Expired │
                          └──────────┘        └─────────┘
```

### Status Values

| Status | Description |
|--------|-------------|
| Voting | Open for votes |
| Passed | Approved, awaiting activation |
| Rejected | Failed to reach supermajority |
| Activated | Successfully applied |
| Cancelled | Withdrawn by proposer |
| Expired | Failed to reach quorum |

## Activation

At epoch boundary, the `FeeScheduleManager` processes proposals:

```rust
pub fn process_epoch(&mut self, current_epoch: Epoch, total_voting_power: u64) {
    // 1. Finalize voting for proposals whose period ended
    // 2. Activate passed proposals at activation_epoch
    // 3. Update active schedules
    // 4. Record in history for audit
}
```

### Deterministic Activation

- Schedules activate exactly at epoch start
- All nodes see same schedule at same time
- No ambiguity about active parameters

## Example: Increasing Base Fee

### Step 1: Create Proposal

```rust
let current = manager.fee_schedule().clone();
let mut new = current;
new.version = 2;
new.tx_base_fee = current.tx_base_fee * 115 / 100;  // +15%

let proposal = FeeScheduleProposal::new(
    proposal_id,
    proposer_address,
    new,
    current,
    current_epoch,
    current_epoch + 3,  // Activate in 3 epochs (21+ days)
    reason_hash,
)?;

manager.submit_fee_proposal(proposal)?;
```

### Step 2: Validators Vote

```rust
manager.vote_fee_proposal(
    &proposal_id,
    validator_address,
    VoteChoice::Yes,
    validator_voting_power,
)?;
```

### Step 3: Automatic Activation

At `activation_epoch`, if quorum + supermajority met:
- New schedule becomes active
- All new transactions use updated fees
- Recorded in `fee_schedule_history`

## Auditability

### Schedule History

```rust
pub fee_schedule_history: Vec<(Epoch, FeeScheduleV1)>
```

Query historical schedules:

```rust
pub fn fee_schedule_at_epoch(&self, epoch: Epoch) -> FeeScheduleV1 {
    self.fee_schedule_history
        .iter()
        .rev()
        .find(|(e, _)| *e <= epoch)
        .map(|(_, s)| *s)
        .unwrap_or_default()
}
```

### Vote Records

Each proposal maintains complete vote history:

```rust
pub struct VoteTally {
    pub yes_power: u64,
    pub no_power: u64,
    pub abstain_power: u64,
    pub voters: HashMap<[u8; 32], VoteChoice>,
}
```

## API Endpoints

### RPC Queries

| Endpoint | Description |
|----------|-------------|
| `GET /fee_schedule` | Current active fee schedule |
| `GET /fee_schedule?version={n}` | Specific schedule version |
| `GET /registry_prices` | Current registry prices |
| `GET /proposals/fee_schedule` | Pending fee proposals |
| `GET /proposals/registry_prices` | Pending registry proposals |

### Response Format

```json
{
  "schedule": {
    "version": 1,
    "tx_base_fee": 100,
    "tx_byte_fee": 1,
    "tx_min_fee": 100,
    "tx_max_fee": 1000000000
  },
  "current_epoch": 42,
  "activated_at_epoch": 0
}
```

## Implementation

- **Types**: `crates/types/src/fee_policy.rs`
- **Governance**: `crates/governance/src/fee_schedule.rs`
- **Manager**: `FeeScheduleManager` in governance crate

## Security Considerations

1. **Timelock prevents flash attacks**: 2-epoch minimum notice (typically 2 weeks)
2. **Rate limits prevent shock**: Max ±20% per change
3. **Quorum prevents minority rule**: **60%** must participate
4. **2/3 Supermajority prevents contentious changes**: **66.67%** approval needed
5. **Version tracking ensures forward progress**: Cannot revert to old versions

## Related Documentation

- [Fee Policy](../economics/fees.md) - Fee computation details
- [Units](../economics/units.md) - Kambei precision
- [Governance Overview](../overview/GOVERNANCE_MODELS.md) - General governance
