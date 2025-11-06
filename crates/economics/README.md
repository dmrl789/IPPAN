# IPPAN Economics

## Overview
- Encapsulates DAG-Fair emission schedules, validator reward distribution, and fee recycling.
- Supplies deterministic economic logic consumed by consensus, treasury, and governance modules.
- Designed to keep emission policy transparent and parameter driven.

## Key Modules
- `emission`: calculate round rewards, supply projections, and emission caps.
- `distribution`: split rewards across validator roles and sink destinations.
- `parameters`: governance-managed parameter proposals and validation.
- `types`: shared data models for economics calculations and results.
- `errors`: explicit error taxonomy for configuration and runtime failures.

## Integration Notes
- Reference `EmissionParams` when aligning governance proposals with runtime defaults.
- Call `distribute_round_reward` during consensus finalization to populate treasury sinks.
- Persist parameter updates through governance flows to keep economics changes auditable.
