# IPPAN Internet Intelligent Layer (IIL) Protocol v0.1

## Overview

The Internet Intelligent Layer (IIL) defines a deterministic JSON API for resolving IPPAN resources. Version 0.1 focuses on identity records backed by the existing handle registry and ships minimal proof bundles binding each response to the node's finalized state.

* Protocol version: `0.1`
* Supported record kind: `identity`
* Not yet implemented: `file`, `service` (explicit 501 responses)
* Hashing: canonical JSON (`json-c14n-v1`) hashed with BLAKE3 (hex-encoded)
* Finality anchor: latest finalized round from local node storage

## Endpoints

### `GET /iil/status`
Returns a health snapshot for the IIL surface.

**Response**
```json
{
  "iil_version": "0.1",
  "finality": { "round_id": 42, "round_hash": "...", "ippan_time_us": 1234 },
  "status": "ok"
}
```

### `GET /iil/resolve/{handle}`
Resolves a URL-encoded handle (e.g. `%40alice.ipn`). Handles must start with `@`, end with `.ipn`, and be ≤64 characters.

**Success Response**
```json
{
  "iil_version": "0.1",
  "finality": { ... },
  "record": {
    "hashid": "<record_hash>",
    "kind": "identity",
    "payload": {
      "handle": "@alice.ipn",
      "owner": "ippan1...",
      "status": "active",
      "metadata": {"display": "Alice"},
      "expires_at": null
    }
  },
  "proof": {
    "finality": { ... },
    "integrity": {
      "record_hash": "<record_hash>",
      "canonicalization": "json-c14n-v1"
    }
  }
}
```

**Error Responses**
* 400 `invalid_handle` – malformed handle input
* 404 `handle_not_found` – registry miss
* 501 `not_implemented` – unsupported kinds (`file`, `service`)

### `GET /iil/get/{hashid}`
Fetches a record by its 64-hex-character `hashid` (canonical record hash). Only identity records are available in v0.1; non-existent hashes return 404.

**Errors**
* 400 `invalid_hashid`
* 404 `record_not_found`

### `POST /iil/query`
Performs multi-record lookup with deterministic ordering.

**Request**
```json
{
  "handles": ["@alice.ipn", "@bob.ipn"],
  "kinds": ["identity"],
  "limit": 50
}
```

Constraints:
* `limit` defaults to 50 and is capped at 200.
* Request bodies are limited to 1 MiB by default (override via `IPPAN_IIL_MAX_BODY_BYTES`).
* Unsupported kinds (`file`, `service`) return a 501 error.

**Response Ordering**
Results are sorted deterministically: primary key `score` descending (if present), then `hashid` ascending (lexicographic).

**Example Response**
```json
{
  "iil_version": "0.1",
  "finality": { ... },
  "results": [
    {
      "score": 100,
      "record": { "hashid": "...", "kind": "identity", "payload": { ... } },
      "proof": {
        "finality": { ... },
        "integrity": {
          "record_hash": "...",
          "canonicalization": "json-c14n-v1"
        }
      }
    }
  ]
}
```

## Proof Bundle (v0.1)
Every endpoint includes a minimal proof bundle:
* `finality` – derived from the latest finalized round (round id, round hash/state root, IPPAN time if available)
* `integrity.record_hash` – BLAKE3 hash of canonical JSON form of the record
* `integrity.canonicalization` – fixed string `json-c14n-v1`
* Optional future fields (not yet in v0.1): inclusion proofs, attestations

## Determinism
* JSON canonicalization sorts object keys recursively and preserves array order.
* Floating-point values are rejected; only integers/strings/booleans/arrays/objects are permitted.
* Query results are sorted by `score` (descending) then `hashid` (ascending) and truncated by `limit`.

## Error Model
* Structured JSON errors: `{ "code": "...", "message": "..." }`
* Common codes: `invalid_handle`, `invalid_hashid`, `not_implemented`, `record_not_found`, `body_too_large`, `invalid_json`

## Non-Goals in v0.1
* File/service record materialization
* Inclusion/attestation proofs
* Mutations or writes through the IIL surface
