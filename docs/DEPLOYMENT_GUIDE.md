# IPPAN Deployment Guide

Operational runbook for releasing, deploying, and validating the IPPAN network. Use this alongside `docs/automated-deployment-guide.md` for infrastructure specifics.

## Release & Versioning Strategy

- **Semantic versioning**: Production artifacts use `vMAJOR.MINOR.PATCH`. Increment MINOR for backward-compatible features, PATCH for hotfixes, and MAJOR for breaking changes.
- **Branch cadence**: Feature branches merge into `develop`; once CI is green, `develop` is promoted into `main` via a reviewed PR. `main` is always deployable.
- **Release workflow**: Trigger `.github/workflows/release.yml` with the target tag (or push a `v*` tag). The workflow generates the changelog, creates the GitHub release, and publishes Docker images to GHCR.
- **Hotfix path**: Urgent fixes land on `hotfix/*`, merge into `main`, tag a patch release, then back-merge to `develop` to keep streams aligned.
- **Artifact retention**: GHCR keeps both `:latest` and immutable `:vX.Y.Z` tags. Deployments must pin the immutable tag in compose files for reproducibility.

## CI/CD Workflow Summary

| Workflow | Path | Trigger | Key Stages |
|----------|------|---------|------------|
| CI | `.github/workflows/ci.yml` | Push/PR to `main` and `develop` | Rust fmt/check/build/clippy/test, AI core determinism, DLC consensus tests, Gateway lint/build, Unified UI lint/type-check/build |
| Build Matrix | `.github/workflows/build.yml` | Nightly schedule, manual | Compiles workspace across toolchains/targets, caches artifacts for deployment |
| Test Matrix | `.github/workflows/test.yml` | Nightly schedule, manual | Runs long-form integration and stress tests before release tagging |
| Deploy (staging/prod) | `.github/workflows/deploy.yml`, `deploy-ippan-full-stack.yml`, `prod-deploy.yml` | Push to `main`, manual promotion | Builds Docker images, publishes to GHCR, SSH deploys to servers, executes health probes |
| Release | `.github/workflows/release.yml` | Tag push or manual dispatch | Creates changelog, publishes release notes, pushes production images |

**Quality gates**

- Every workflow above must succeed for the same commit before production deployments are approved.
- Release promotion requires a successful CI run on the tagged commit and completion of the applicable deploy workflow.
- Security (`security.yml`) and governance (`metaagent-governance.yml`) workflows run in parallel; any failure blocks release until resolved.

## Deployment Promotion Flow

1. **Merge to `main`** after CI success and reviewer approval.
2. **Automated deploy** (`deploy.yml`/`deploy-ippan-full-stack.yml`) builds and ships new containers to staging and production servers.
3. **Release tag**: Dispatch `Release` workflow with immutable `vX.Y.Z` tag once staging validation passes.
4. **Operational validation**: Run the scripts in the next section. Capture their JSON/terminal output and attach to deployment notes.
5. **QA checklist**: Execute startup, validator rotation, and recovery tests before closing the change request.

## Operational Validation Runbooks

| Script | Purpose | Command | Success Criteria |
|--------|---------|---------|------------------|
| `deploy/check-nodes.sh` | Snapshot health endpoints and peer connectivity for a target host list. Outputs JSON summary for dashboards. | `HOST=188.245.97.41 API_BASE=http://188.245.97.41:8080 ./deploy/check-nodes.sh` | Exit code `0`, HTTP 200 for `/health`, `/status`, `/peers`, and `peer_count ≥ 1`. Non-zero exit indicates rollout regression. |
| `deploy/health-check.sh` | Deep health probe for production nodes including consensus drift analysis. | `./deploy/health-check.sh` | All critical checks print `✓`; warnings only for transient conditions. Block height difference must stay ≤ 5, each node reachable, `status = ok`, and peer count > 0. |
| `deploy/verify-deployment.sh` | Smoke test UI, gateway, RPC, and P2P ports immediately after deploy. | `./deploy/verify-deployment.sh` | Each endpoint reports `✅`; script exits `0`. Any ❌ requires investigation before sign-off. |
| `deploy/verify-production.sh` | Comprehensive post-deploy scorecard with PASS/WARN/FAIL tallies. | `./deploy/verify-production.sh` | Totals show no FAIL items; WARN items must be documented with owner and follow-up. |
| `deploy/monitor-production.sh` | Continuous monitor logging node vitals every 30s and writing alerts. | `sudo ./deploy/monitor-production.sh` | Keeps writing `INFO` logs, no `ALERT` entries in `/var/log/ippan/monitor.log`. Script is long-running; stop with `Ctrl+C` once monitoring window completes. |

> **Tooling requirements**: The validation scripts expect `curl`, `jq`, `docker`, and SSH access to the deployment hosts. Ensure they are available on the operator workstation or bastion.

## QA Checklist

### Startup Validation

- **Fresh deploy**: After containers restart, run `deploy/health-check.sh` to confirm all HTTP endpoints respond with 200.
- **Log inspection**: Tail `docker compose logs -f ippan-node-1` and `ippan-node-2`; ensure HashTimer initialization and consensus startup complete without panics.
- **Peer formation**: Execute `curl http://<node>:8080/peers` and confirm every node lists the others with `state: Connected`.
- **Block production**: Call `curl http://<node>:8080/status | jq '.consensus.latest_block_height'` twice over 60s; height must increase on at least one node.

### Validator Rotation

- **Baseline snapshot**: Run `cargo run --bin ippan-check-nodes -- --api http://<node>:8080 --json` to capture current proposer/validator data (included in the `status` payload when exposed) and archive the output.
- **Rotation trigger**: Allow consensus to progress for ≥ 3 rounds (watch the `consensus.current_round` field in the checker output) or submit a low-value transaction to advance slots.
- **Validation**: Ensure the reported proposer or validator ID changes after the rotation window. Cross-check `deploy/health-check.sh` output to confirm `consensus.latest_block_height` advances and both nodes remain within the allowed block-height drift.
- **Audit log**: Note the validator IDs before/after rotation in deployment notes together with timestamps from the checker or node logs (look for `Consensus state => ... proposer: ...` entries).

### Recovery & Failure Handling

- **Single node restart**: Restart one node (`docker compose restart ippan-node-1`) and verify `uptime_secs` resets while peer count recovers to previous level within 60s.
- **Network partition simulation**: Temporarily block P2P port on one host (`sudo ufw deny 9000`) and confirm `deploy/check-nodes.sh` exits non-zero, then remove the rule and rerun until it returns success.
- **Data consistency**: After recovery, re-run `cargo run --bin ippan-check-nodes -- --api http://<node>:8080,http://<other-node>:8080 --require-peers <count> --json` and verify `consensus.latest_block_height` (when present) or block hashes reported by each node are aligned.
- **Alerting path**: Ensure `deploy/monitor-production.sh` writes an `ALERT` entry during simulated outage and resolves after connectivity is restored.

Document outcomes and attach command outputs to the change record. Do not mark a deployment complete until every checklist item is satisfied or explicitly waived by a maintainer.
