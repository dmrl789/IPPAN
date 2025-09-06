# 📋 IPPAN Canonical JSON Schema Specification

## Overview

This document defines the canonical JSON schema for IPPAN data structures, ensuring consistency across all implementations, APIs, and user interfaces. The schema is based on JSON Schema Draft 2020-12 and provides a complete specification for IPPAN blockchain data.

## Schema ID

- **Schema ID**: `https://ippan.org/schema/round-v1.json`
- **Version**: v1
- **Base Schema**: JSON Schema Draft 2020-12

## Complete Schema Definition

```json
{
  "$id": "https://ippan.org/schema/round-v1.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "IPPAN Round (v1)",
  "type": "object",
  "required": ["version", "round_id", "state", "time", "block_count", "zk_stark_proof", "merkle_root", "blocks"],
  "properties": {
    "version": { "const": "v1" },

    "round_id": { "type": "string", "pattern": "^[0-9]+$" },

    "state": { "type": "string", "enum": ["pending", "finalizing", "finalized", "rejected"] },

    "time": {
      "type": "object",
      "required": ["start_ns", "end_ns"],
      "properties": {
        "start_ns": { "type": "string", "pattern": "^[0-9]+$" },
        "end_ns":   { "type": "string", "pattern": "^[0-9]+$" }
      },
      "additionalProperties": false
    },

    "block_count": { "type": "integer", "minimum": 0 },

    "zk_stark_proof": { "type": "string", "pattern": "^[0-9a-fA-F]{64}$" },   // 32 bytes hex

    "merkle_root":   { "type": "string", "pattern": "^[0-9a-fA-F]{64}$" },   // 32 bytes hex

    "blocks": {
      "type": "array",
      "items": { "$ref": "#/$defs/block" }
    }
  },

  "$defs": {
    "hex16": { "type": "string", "pattern": "^[0-9a-fA-F]{32}$" },           // 16 bytes
    "hex32": { "type": "string", "pattern": "^[0-9a-fA-F]{64}$" },           // 32 bytes
    "uint":  { "type": "string", "pattern": "^[0-9]+$" },                    // stringified u64/u128
    "int":   { "type": "string", "pattern": "^-?[0-9]+$" },

    "hashtimer_v1": {
      "type": "object",
      "required": ["version", "time", "position", "node_id", "payload_digest", "hash_timer_digest"],
      "properties": {
        "version": { "const": "v1" },

        "time": {
          "type": "object",
          "required": ["t_ns", "precision_ns", "drift_ns"],
          "properties": {
            "t_ns":         { "$ref": "#/$defs/uint" },    // nanoseconds since epoch
            "precision_ns": { "type": "integer", "minimum": 1 },
            "drift_ns":     { "$ref": "#/$defs/int" }
          },
          "additionalProperties": false
        },

        "position": {
          "type": "object",
          "required": ["round", "seq", "kind"],
          "properties": {
            "round": { "$ref": "#/$defs/uint" },
            "seq":   { "type": "integer", "minimum": 1 },
            "kind":  { "type": "string", "enum": ["Tx", "Block", "Round"] }
          },
          "additionalProperties": false
        },

        "node_id":        { "$ref": "#/$defs/hex16" },     // 16-byte id in hex
        "payload_digest": { "$ref": "#/$defs/hex32" },     // SHA-256(payload w/o HashTimer)
        "hash_timer_digest": { "$ref": "#/$defs/hex32" }   // SHA-256(96-byte HT buffer)
      },
      "additionalProperties": false
    },

    "tx": {
      "type": "object",
      "required": ["tx_hash", "from", "to", "amount", "fee", "nonce", "signature", "hashtimer"],
      "properties": {
        "tx_hash":   { "$ref": "#/$defs/hex32" },
        "from":      { "$ref": "#/$defs/hex32" },          // 20 bytes typical → still allow 32 for flexibility
        "to":        { "$ref": "#/$defs/hex32" },
        "amount":    { "$ref": "#/$defs/uint" },
        "fee":       { "type": "integer", "minimum": 0 },
        "nonce":     { "type": "integer", "minimum": 0 },
        "memo":      { "type": "string" },
        "signature": { "type": "string", "pattern": "^[0-9a-fA-F]+$" }, // length depends on scheme
        "hashtimer": { "$ref": "#/$defs/hashtimer_v1" }
      },
      "additionalProperties": false
    },

    "block": {
      "type": "object",
      "required": ["block_id", "producer", "status", "tx_count", "hashtimer", "header_digest", "parents", "parent_rounds", "txs"],
      "properties": {
        "block_id":      { "type": "string" },              // e.g., "block-54" or hex id
        "producer": {
          "type": "object",
          "required": ["node_id", "label"],
          "properties": {
            "node_id": { "$ref": "#/$defs/hex16" },
            "label":   { "type": "string" }                 // e.g., "validator-1"
          },
          "additionalProperties": false
        },
        "status":   { "type": "string", "enum": ["pending", "finalized"] },
        "tx_count": { "type": "integer", "minimum": 0 },
        "header_digest": { "$ref": "#/$defs/hex32" },
        "hashtimer": { "$ref": "#/$defs/hashtimer_v1" },
        "parents": {
          "type": "array",
          "items": { "$ref": "#/$defs/hex32" },
          "minItems": 1,
          "maxItems": 8,
          "uniqueItems": true
        },
        "parent_rounds": {
          "type": "array",
          "items": { "$ref": "#/$defs/uint" },
          "minItems": 1,
          "maxItems": 8
        },
        "txs": {
          "type": "array",
          "items": { "$ref": "#/$defs/tx" }
        }
      },
      "additionalProperties": false
    }
  },

  "additionalProperties": false
}
```

## Data Type Definitions

### Primitive Types

- **`hex16`**: 16-byte value represented as 32 hex characters
- **`hex32`**: 32-byte value represented as 64 hex characters  
- **`uint`**: Stringified unsigned integer (u64/u128) to avoid JavaScript 53-bit limitations
- **`int`**: Stringified signed integer (i32/i64) to avoid JavaScript 53-bit limitations

### HashTimer v1 Structure

The HashTimer v1 is the core timing mechanism for IPPAN:

```json
{
  "version": "v1",
  "time": {
    "t_ns": "1756995008183000000",        // Nanoseconds since epoch
    "precision_ns": 100,                  // Precision quantum
    "drift_ns": "-116"                    // Clock drift in nanoseconds
  },
  "position": {
    "round": "8784975040",                // Consensus round
    "seq": 1,                             // Sequence within round
    "kind": "Tx"                          // Event type: Tx, Block, or Round
  },
  "node_id": "76687a7e80849ea0c7c0a0d9f4d0a3b1",  // 16-byte node ID
  "payload_digest": "a847df59a9e8a893351ba4d26508cdefa2b4c6d8e0f1a2b3c4d5e6f708192a3b",
  "hash_timer_digest": "1f5b7c3a2e0d4c9b8a76f5e4d3c2b1a0ffeeddccbbaa99887766554433221100"
}
```

## Canonical Ordering

When rendering or validating order, sort by this tuple:

```
(t_ns, round, seq, node_id, payload_digest)
```

- `t_ns` = `hashtimer.time.t_ns` (string → compare as integer)
- `round` = `hashtimer.position.round` (string → compare as integer)
- `seq` = `hashtimer.position.seq` (integer)
- `node_id` = hex lexicographic
- `payload_digest` = hex lexicographic

This yields a **total, deterministic order** across Tx and Blocks.

## Example Data

### Complete Round Example

```json
{
  "version": "v1",
  "round_id": "8784975040",
  "state": "finalized",
  "time": { 
    "start_ns": "1756995008000000000", 
    "end_ns": "1756995008250000000" 
  },
  "block_count": 8,
  "zk_stark_proof": "b597133e7c45d8c0b3b0c9a2b1f0f9aa9c00aa11bb22cc33dd44ee55ff667788",
  "merkle_root": "c5d42a59e1ae68e1c2a9ff00bb11aa22cc33dd44ee55ff66778899aabbccddee",
  "blocks": [
    {
      "block_id": "block-54",
      "producer": {
        "node_id": "76687a7e80849ea0c7c0a0d9f4d0a3b1",
        "label": "validator-1"
      },
      "status": "finalized",
      "tx_count": 54,
      "header_digest": "3440434551535e626ca478b18893979da2dbb5bbbfc4cf07dd15eb23f9310209",
      "parents": [
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
      ],
      "parent_rounds": ["8784975037", "8784975039"],
      "hashtimer": {
        "version": "v1",
        "time": { 
          "t_ns": "1756995008183000000", 
          "precision_ns": 100, 
          "drift_ns": "-116" 
        },
        "position": { 
          "round": "8784975040", 
          "seq": 1, 
          "kind": "Block" 
        },
        "node_id": "76687a7e80849ea0c7c0a0d9f4d0a3b1",
        "payload_digest": "a847df59a9e8a893351ba4d26508cdefa2b4c6d8e0f1a2b3c4d5e6f708192a3b",
        "hash_timer_digest": "1f5b7c3a2e0d4c9b8a76f5e4d3c2b1a0ffeeddccbbaa99887766554433221100"
      },
      "txs": [
        {
          "tx_hash": "ee50e75a33a55f9709ccf833b5b46eddeffd08ef6011e2576918716e238c4a49",
          "from": "881d8fa336c49e2b4ac499a8a4881b460b14e36d",
          "to": "41b9df3438166094439c64bd8fc9e737bbb5e380",
          "amount": "565419",
          "fee": 188,
          "nonce": 1219,
          "memo": "optional",
          "signature": "f6c6a3a96ab7...bbb118",
          "hashtimer": {
            "version": "v1",
            "time": { 
              "t_ns": "1756993856229000000", 
              "precision_ns": 100, 
              "drift_ns": "-408" 
            },
            "position": { 
              "round": "8784969281", 
              "seq": 1, 
              "kind": "Tx" 
            },
            "node_id": "76687a7e80849ea0c7c0a0d9f4d0a3b1",
            "payload_digest": "a847df59a9e8a893351ba4d26508cdefa2b4c6d8e0f1a2b3c4d5e6f708192a3b",
            "hash_timer_digest": "3440434551535e626ca478b18893979da2dbb5bbbfc4cf07dd15eb23f9310209"
          }
        }
      ]
    }
  ]
}
```

## Implementation Notes

### Big Integer Handling

- **Big integers** (`t_ns`, `round`, `amount`) are **strings** in JSON
- Compare numerically in code using `BigInt()` or equivalent
- This avoids JavaScript's 53-bit integer limitation

### Node ID Format

- **`node_id`** is a **16-byte hex** (32 hex chars)
- Keep friendly labels separately in the `producer.label` field
- Example: `"node_id": "76687a7e80849ea0c7c0a0d9f4d0a3b1"` with `"label": "validator-1"`

### Digest Format

- **Digests** are **32-byte hex** (64 hex chars)
- Always include the full HashTimer tuple **and** the `hash_timer_digest`
- The digest is the proof; the tuple is for ordering
- For block/round proofs, expose **full 32-byte** values in APIs

### Validation Rules

1. **Stringified integers** must match the `uint` or `int` patterns
2. **Hex values** must be exactly 32 or 64 characters
3. **HashTimer v1** must include all required fields
4. **Canonical ordering** must be maintained for deterministic processing

## Schema Validation

Use any JSON Schema Draft 2020-12 validator to validate IPPAN data:

```javascript
import Ajv from 'ajv';
import addFormats from 'ajv-formats';

const ajv = new Ajv();
addFormats(ajv);

const validate = ajv.compile(ippanRoundSchema);
const isValid = validate(roundData);
```

## Versioning

- **Current Version**: v1
- **Schema ID**: `https://ippan.org/schema/round-v1.json`
- **Backward Compatibility**: Maintained through version tags
- **Forward Compatibility**: New fields will be optional in future versions

## References

- [JSON Schema Draft 2020-12](https://json-schema.org/draft/2020-12/schema)
- [IPPAN HashTimer v1 Specification](./IPPAN_PRD.md#hashTimer-canonical-v1)
- [IPPAN Product Requirements Document](./IPPAN_PRD.md)
