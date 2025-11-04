# IPPAN Treasury

## Overview
- Manages validator reward sinks, fee collection, and treasury accounting.
- Serves as the execution layer for economics outputs from consensus rounds.
- Ensures deterministic accounting paths for on-chain distribution.

## Key Modules
- `reward_pool`: holds and allocates per-round rewards to validators and sinks.
- `fee_collector`: aggregates transaction fees, enforces caps, and routes proceeds.
- `account_ledger`: maintains balances and history for treasury-managed accounts.

## Integration Notes
- Feed outputs from `ippan_economics` into `RewardPool` to finalize payouts.
- Keep treasury storage backed by `ippan_storage` implementations for persistence.
- Use the ledger helpers when exposing treasury state through RPC or governance dashboards.
