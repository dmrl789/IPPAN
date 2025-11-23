# Storage Layout, Snapshots, and Recovery

This guide captures how IPPAN persists state on disk, how to take/export a
snapshot, how to import that snapshot on a new node, and what manual crash
recovery looks like today.

## Storage layout inventory

The sled-backed `ippan-storage` crate owns all durable data that consensus,
RPC, and operators rely on. Each logical entity has a deterministic keyspace
(or sled tree) so operators know exactly what is persisted between restarts.

| Tree / key prefix          | Description |
|---------------------------|-------------|
| `blocks`                  | Canonical blocks keyed by `block.hash()`. All block metadata and bundled transactions are serialized as JSON. |
| `transactions`            | Confirmed transactions (payments) keyed by transaction hash. Used for history lookups and manifest payment counts. |
| `accounts`                | Account balances/nonces keyed by raw address bytes. |
| `metadata:latest_height`  | Highest block height seen locally. Updated whenever a block is stored. |
| `metadata:latest_finalized_round` | Last finalized round identifier. Written alongside `round_finalizations`. |
| `metadata:chain_state`    | Economic chain state (issuance totals, last round) serialized as JSON. |
| `metadata:network_id`     | Identifier of the network this database belongs to (e.g. `ippan-devnet`, `ippan-mainnet`). Enforced during snapshot import/export. |
| `metadata:ai_model_hash`  | Optional active AI/GBDT model hash advertised by the AI registry. Only the active hash is stored today (no history). |
| `l2_networks`, `l2_commits`, `l2_exits` | Layer-2 metadata, commits, and exit records. |
| `round_certificates`      | Stored certificates keyed by `round_id` for auditability. |
| `round_finalizations`     | Finalization records keyed by `round_id`. Exported/imported to preserve last round info. |
| `validator_telemetry`     | Validator telemetry snapshots keyed by validator ID. |
| `file_descriptors`        | Off-chain file descriptor metadata keyed by descriptor ID. |
| `file_owner_index`        | Secondary index mapping owner address → descriptor IDs for fast lookups. |

### Known gaps

* **Handles** (`crates/l2_handle_registry`) currently live in memory only. The
  snapshot pipeline exports an empty `handles.jsonl` placeholder so operators
  are reminded to back up handle registrations manually until disk-backed
  handle storage lands.
* **AI registry history** stores only the active model hash (`metadata:ai_model_hash`).
  Historical models are not tracked yet; future work will add them.

## Snapshot manifest

Snapshots are described by a JSON manifest stored at
`<snapshot_dir>/manifest.json`:

```json
{
  "version": 1,
  "network_id": "ippan-devnet",
  "height": 420,
  "last_round_id": "419",
  "timestamp_us": 1731500000000,
  "accounts_count": 12,
  "payments_count": 64,
  "blocks_count": 421,
  "handles_count": 0,
  "files_count": 5,
  "ai_model_hash": "b3f3..."
}
```

`SnapshotManifest::new_from_storage` derives this data directly from storage
and `validate_against_storage` re-counts everything to make sure a manifest
matches what is on disk.

## Snapshot directory layout

```
<snapshot_root>/
├── manifest.json
├── blocks.jsonl
├── payments.jsonl
├── accounts.jsonl
├── handles.jsonl        # currently empty until handle persistence lands
├── files.jsonl
├── rounds.jsonl
└── chain_state.json
```

All list files use deterministic JSONL ordering so two nodes with identical
state export byte-identical snapshots:

* Blocks are sorted by round then block hash.
* Transactions/payments are sorted by transaction hash.
* Accounts are sorted by address bytes.
* File descriptors are sorted by descriptor ID.
* Round finalization records are sorted by round.

## CLI: export and import

Two maintenance subcommands were added to the node binary. They only touch the
disk database—no networking or consensus services are started.

```bash
# Export a snapshot
IPPAN_NETWORK_ID=ippan-devnet ippan-node \
  --config deployments/testnet/configs/testnet-node-1.toml \
  snapshot export --dir /var/backups/ippan-2025-11-15

# Import a snapshot into a fresh data directory
IPPAN_NETWORK_ID=ippan-devnet ippan-node \
  --config deployments/testnet/configs/testnet-node-1.toml \
  snapshot import --dir /var/backups/ippan-2025-11-15
```

Notes:

* The snapshot directory must be empty before exporting and must already
  exist before importing.
* Importing requires an empty database (no blocks, accounts, or transactions).
  Start from a clean data directory or delete the old `db/` folder first.
* Network IDs must match. Importing a `ippan-testnet` snapshot into a
  `ippan-mainnet` node will be rejected before any data is written.
* `handles.jsonl` is informational until the handle registry becomes
  persistent.

## Manual crash/restart sanity check

1. Start a node normally: `ippan-node --config <path> ...`.
2. Submit at least one state-changing action (e.g. send a payment via the CLI
   or register a handle).
3. Abruptly stop the node (Ctrl+C or `kill -9 <pid>`).
4. Restart the node with the same data directory.
5. Verify state survived:
   * Query the account balance via RPC (`GET /account/<address>`).
   * List file descriptors or other records to confirm they match the pre-crash
     state.
6. Optionally run `ippan-node snapshot export --dir /tmp/after-crash` to double
   check the manifest counts.

This flow ensures sled flushes + replay logic are healthy: the restarted node
should immediately expose the same accounts and files without needing to
re-ingest blocks.

## Test coverage checkpoints

* `crates/storage/tests/persistence_conflicts.rs` runs sled-backed restart
  tests that reopen the database after clean shutdowns and crash-like drops.
  Canonical state (chain state + balances) and side branches are all
  reloaded, ensuring forks remain queryable for audit while the canonical head
  survives a restart.
* The same suite models DAG conflicts and explicit reorgs with the in-memory
  backend to prove state updates are not double-applied when consensus changes
  the canonical branch.

## Operator workflow summary

1. **Before maintenance/migration**: run `ippan-node snapshot export --dir ...`
   and copy the resulting directory to durable storage.
2. **To migrate**: provision a clean node, copy the snapshot directory over,
   run `ippan-node snapshot import --dir ...`, then start the node normally.
3. **To audit**: use `manifest.json` alongside `SnapshotManifest::validate` to
   prove a data directory matches the manifest counts.

Keep this document close whenever you need to reason about what is persisted,
how to back it up, and how to prove a restored node matches the original.
