# Running an IPPAN v0.9.0-rc1 Node (Release Candidate)

> **Release Candidate only** – not for mainnet/production funds. Expect breaking changes before v1.0.

## Prerequisites
- Rust toolchain: `rustup` with stable 1.78+ (matching `rust-toolchain.toml`).
- OS: Linux x86_64 or aarch64 (tested in CI across both).
- Build essentials: `clang`, `cmake`, `pkg-config`, and OpenSSL headers available via your package manager.

## Build steps
```bash
git clone https://github.com/dmrl789/IPPAN.git
cd IPPAN
cargo build --release -p node
```

## Configuration
- Example config path: `config/node.toml` (copy and adjust for your environment).
- Key settings to review:
  - `node_id`: unique identifier for your validator/observer.
  - `rpc.addr` / `rpc.port`: external RPC endpoint for operators.
  - `network.listen_addr`: advertised listening address for peers.
  - `storage.path`: persistent data directory.
  - `time.hash_timer` and networking toggles for IPPAN Time coordination.
  - `security`: rate limits and IP allowlists.

## Running the node
```bash
./target/release/node --config config/node.toml
```
- The binary prints the IPPAN version at startup (`v0.9.0-rc1`).

## Observability
- Logs: stdout/stderr by default; direct to files via your process supervisor.
- Health: RPC endpoints expose payment, handle, file, AI status, and operator health surfaces.
- Metrics: enable any metrics exporters configured in `config/node.toml` (if applicable).

## Safety warnings
- This RC is for **testnet/devnet experimentation only**; do not stake or store production funds.
- Configuration schemas may change before v1.0—revisit defaults after upgrades.
- Validate binaries you run and rebuild after pulling new commits or tags.
