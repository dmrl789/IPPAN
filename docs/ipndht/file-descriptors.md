# IPNDHT File Descriptor Runtime Notes

The file descriptor pipeline uses the `ippan-files` crate for metadata (ID
creation, validation, and storage) and selects a DHT backend at runtime via
environment variables:

## Runtime DHT modes

| Mode   | Description | Configuration |
|--------|-------------|----------------|
| `stub` | Default for tests/dev. File descriptors are validated and stored
locally, but DHT publish/find calls short-circuit to in-memory stubs. | `IPPAN_FILE_DHT_MODE=stub` |
| `libp2p` | Uses the libp2p Kademlia network to publish descriptors and query
providers. The node starts its own libp2p swarm dedicated to the File DHT. | `IPPAN_FILE_DHT_MODE=libp2p` |

Additional libp2p settings:

- `FILE_DHT_LIBP2P_LISTEN` &mdash; comma-separated multiaddrs (default:
  `/ip4/0.0.0.0/tcp/9100`).
- `FILE_DHT_LIBP2P_BOOTSTRAP` &mdash; optional comma-separated bootstrap multiaddrs.

When `libp2p` mode is enabled, the node instantiates `Libp2pFileDhtService`,
which serializes descriptors into Kademlia records (`file_id` as the key) and
queries providers via `libp2p::kad::Behaviour`. The RPC handlers in
`crates/rpc/src/files.rs` transparently await the configured implementation.

The `/files/publish` and `/files/{id}` endpoints always use the configured DHT
service through `AppState.file_dht`, so switching modes does not require API
changes.

## Handle records

Handle registrations now reuse the same IPNDHT infrastructure through a
dedicated `HandleDhtService` (stub and libp2p-backed implementations). The
consensus handle pipeline publishes each accepted registration into the DHT so
handles can be discovered by other nodes without querying the local registry.

- Keys: `handle:` + `blake3(@handle.ipn)`, ensuring namespace separation from
  file IDs.
- Values: JSON-serialized `HandleDhtRecord { handle, owner, expires_at }`.
- Runtime toggle: `IPPAN_HANDLE_DHT_MODE=stub|libp2p` (defaults to `stub`).

When `libp2p` is selected the node shares a single `IpnDhtService` instance for
both file and handle records so the Kademlia swarm only needs to boot once.
