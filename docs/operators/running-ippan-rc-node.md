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

# (Recommended) Embed the git revision into the binary for operator visibility;
# this is picked up by `option_env!("GIT_COMMIT_HASH")` in the node binary.
export GIT_COMMIT_HASH="$(git rev-parse --short HEAD)"

# Build the RC node
cargo build --release -p node
```

## Configuration
- Example config path: `config/node.toml` (copy and adjust for your environment).
- Prefer the new profile-aware configs (`config/devnet.toml`, `config/testnet.toml`,
  `config/mainnet.toml`). See `docs/operators/node-configuration-profiles.md`
  for defaults and selection guidance.
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
- If you exported `GIT_COMMIT_HASH` during the build, the startup log will also
  show the embedded commit, which helps operators confirm exactly which RC
  binary is running. Example:

  ```text
  2025-11-20T10:00:15Z  INFO node::version: IPPAN v0.9.0-rc1 (git 1a2b3c4)
  ```

## Observability
- Logs: stdout/stderr by default; direct to files via your process supervisor.
- Health: RPC endpoints expose payment, handle, file, AI status, and operator health surfaces.
- Metrics: enable any metrics exporters configured in `config/node.toml` (if applicable).

## Safety warnings
- This RC is for **testnet/devnet experimentation only**; do not stake or store production funds.
- Configuration schemas may change before v1.0—revisit defaults after upgrades.
- Validate binaries you run and rebuild after pulling new commits or tags.
