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

### DevNet / multi-node: configuring a real validator set (without changing consensus rules)

By default, nodes only know **their own** validator id unless additional ids are provided.

Set a comma-separated validator set on each node:

```bash
export IPPAN_VALIDATOR_IDS="<64hex_id_1>,<64hex_id_2>,<64hex_id_3>,<64hex_id_4>"
```

After restart, `/status.validator_count` should reflect the full set and Explorer should display it from `/status.validator_count`.


