# AI Status API

Deterministic GBDT (D-GBDT) scoring is now observable through a lightweight RPC
endpoint. Operators, explorers, and automation can query the node to learn
exactly which AI model is active and whether the deterministic stub is being
used.

## Endpoint

- **Method:** `GET`
- **Path:** `/ai/status`
- **Auth:** None (inherits existing RPC security controls)

## Response Schema

```jsonc
{
  "enabled": true,
  "using_stub": false,
  "model_hash": "6f39...c1b2",
  "model_version": "1"
}
```

| Field          | Type            | Description                                                                 |
| -------------- | --------------- | --------------------------------------------------------------------------- |
| `enabled`      | `bool`          | `true` when the node is compiled/configured with D-GBDT fairness support.   |
| `using_stub`   | `bool`          | `true` if the deterministic stub model is active (e.g., registry missing).  |
| `model_hash`   | `string/null`   | Hex-encoded BLAKE3 hash of the active model, if one has been loaded.        |
| `model_version`| `string/null`   | Optional version string reported by the model (defaults to `"1"`).          |

## Semantics

- When `enabled = false`, the node either lacks D-GBDT support or AI status has
  not been wired into the running consensus.
- `using_stub = true` signals deterministic fallback scoring (typically only
  used in tests or when `IPPAN_DGBDT_ALLOW_STUB=1`).
- `model_hash` and `model_version` are sourced directly from
  `consensus_dlc::DlcConsensus::ai_status()` and therefore always reflect the
  hash/metadata used for validator selection.

## Example Usage

```bash
curl http://localhost:8080/ai/status | jq
```

```json
{
  "enabled": true,
  "using_stub": false,
  "model_hash": "3c72e0f6d0d8869c8b0f5e62c3d1f4e0f4a9b387a1b1564c91da2cf9e2437d1f",
  "model_version": "1"
}
```

With this endpoint, dashboards and explorers can show the live AI model hash,
verify whether a node is running the production registry model, and alert if a
validator falls back to the stub.
