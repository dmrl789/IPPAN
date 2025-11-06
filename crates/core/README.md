# IPPAN Core

## Overview
- Houses BlockDAG primitives, synchronization services, and Stark proof helpers.
- Supplies the shared data structures consumed by consensus, mempool, and network crates.
- Focused on deterministic ordering and conflict resolution within DAG-based consensus.

## Key Modules
- `block` and `dag`: define DAG nodes, headers, and persistence helpers.
- `dag_sync`, `sync_manager`, and `dag_operations`: manage peer synchronization and conflict handling.
- `order`: deterministic ordering for finalized rounds.
- `zk_stark`: generate and verify Stark proofs for DAG state commitments.

## Integration Notes
- Instantiate `BlockDAG` for in-memory DAG operations; wire storage through external crates.
- Use `start_dag_sync` to bridge the network stack and DAG updates.
- Stark helpers are optional but keep consensus proofs reproducible when enabled.
