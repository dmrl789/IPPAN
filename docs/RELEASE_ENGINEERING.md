# Release Engineering Guide

## Semantic versioning

IPPAN crates share a workspace version (currently `0.1.0`). We follow semantic
versioning:

- **MAJOR** — protocol-breaking changes, storage migrations, new consensus rules.
- **MINOR** — backward compatible features (new RPC endpoints, ops tooling).
- **PATCH** — bug fixes or documentation-only changes.

Each release tag is `vX.Y.Z` and propagates to the `ippan-node`, `ippan-cli`, and
`ippan-rpc` crates via the shared workspace package metadata.

## Release flow

1. Bump `workspace.package.version` in `Cargo.toml`.
2. Run targeted checks:
   ```bash
   cargo check -p ippan-node
   cargo check -p ippan-rpc
   cargo test -p ippan-node -- --nocapture
   ```
3. Build the production binary with the hardened release profile:
   ```bash
   cargo build --release --bin ippan-node --features production
   ```
4. Stage the release artifacts:
   - `target/release/ippan-node`
   - `release/config-template.toml`
   - `release/ippan-node.service`
   - `release/README_RELEASE.md`

## Package tarball

Bundle the binary and configs into a portable archive:

```
tar -czf ippan-node-vX.Y.Z-linux-x64.tar.gz \
    ippan-node \
    release/config-template.toml \
    release/ippan-node.service \
    release/README_RELEASE.md
```

The archive should be produced from within `target/release` (copy the `release/`
files next to the binary before running `tar`).

## Operator installation via systemd

1. Copy the tarball to the server and extract it under `/tmp/ippan-release/`.
2. Install the binary and supporting files:
   ```bash
   sudo install -m 0755 ippan-node /usr/local/bin/ippan-node
   sudo install -m 0644 release/ippan-node.service /etc/systemd/system/ippan-node.service
   sudo mkdir -p /etc/ippan && sudo cp release/config-template.toml /etc/ippan/config.toml
   sudo chown -R ippan:ippan /var/lib/ippan
   ```
3. Edit `/etc/ippan/config.toml` with site-specific values, then validate:
   ```bash
   ippan-node --check --config /etc/ippan/config.toml
   ```
4. Enable and monitor the service:
   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable --now ippan-node
   journalctl -fu ippan-node
   ```

This process yields reproducible release artifacts and a clear hand-off from
developers to operators.
