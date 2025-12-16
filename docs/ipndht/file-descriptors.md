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

### On-chain/DHT footprint

File descriptors are **not** anchored on the IPPAN blockchain; they live in the
File DHT (stub or libp2p) as small JSON blobs. Using the example values above:

- Fixed fields: `id` (32 bytes), `content_hash` (32 bytes), `owner` (32 bytes),
  `size_bytes` (8 bytes), `created_at_us` (8 bytes), and `dht_published`
  (boolean).
- Optional fields: `mime_type` (short UTF-8 string) and `tags` (small string
  array).

Even with all optional fields present the serialized descriptor remains only a
few hundred bytes in the DHT, and **zero** bytes are written to consensus
storage because the file payload itself never touches the chain.

## Example publish flow

Below is a concrete example of the JSON payload sent to `/files/publish` and
the response returned by the node. Values use valid lengths for the
`content_hash` (64 hex chars) and `owner` (Base58Check IPN address beginning
with `i`; the RPC also accepts the equivalent `0x`-prefixed hex form):

**Request**

```json
{
  "owner": "ippan1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqdsk9t",
  "content_hash": "5a8b5d6d4c3e2f1a0b9c8d7e6f5a4b3c2d1e0f9a8b7c6d5e4f3a2b1c0d9e8f7",
  "size_bytes": 24576,
  "mime_type": "application/pdf",
  "tags": ["whitepaper", "v1"]
}
```

**Response**

```json
{
  "id": "a93cf5d4e1b0f3c4d2a1e6f7c8b9d0e1f2a3b4c5d6e7f8091a2b3c4d5e6f708",
  "content_hash": "5a8b5d6d4c3e2f1a0b9c8d7e6f5a4b3c2d1e0f9a8b7c6d5e4f3a2b1c0d9e8f7",
  "owner": "ippan1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqdsk9t",
  "size_bytes": 24576,
  "created_at_us": 1700000123456789,
  "mime_type": "application/pdf",
  "tags": ["whitepaper", "v1"],
  "dht_published": true
}
```

The `id` is deterministically derived from the provided `content_hash` and
`owner`, so identical inputs always generate the same identifier.

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

---

See also: [End-to-End IPPAN Dev Demo](../demo_end_to_end_ippan.md)
