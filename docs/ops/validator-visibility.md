## Validator visibility (Explorer + RPC)

Explorer should **not guess** validator counts. Use the node RPC as the source of truth.

### Endpoints

- **`GET /status`** (recommended for Explorer)
  - `node_id`
  - `peer_count`
  - `validator_count`
  - `consensus.validator_ids` (list)
  - `consensus.round`
  - `mempool_size`
  - `rpc_queue_depth` / `rpc_queue_capacity` (if present)
- **`GET /consensus/view`** (debug/ops)
  - includes the same consensus view plus queue + mempool snapshots

### Where `validator_count` comes from

`validator_count` is computed from the same **in-memory consensus view** returned under `consensus.validator_ids`.

### DevNet / multi-node: where validator_count comes from

`validator_count` is derived from **live cluster state** via P2P peer announcements:

- nodes periodically send `PeerInfo` messages
- each includes `validator_id_hex` and `node_id`
- `/status` counts **distinct active validator ids** observed within a recent window (plus self)

This avoids env-var/config “guessing” and makes Explorer evidence-driven.


