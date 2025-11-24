# Production Readiness Checklist

Use this checklist before promoting a validator, gateway, or explorer stack to production. Items are grouped by persona; complete every line unless marked optional.

## Validators

- [ ] Hardware meets spec (8+ cores, 32 GB RAM, NVMe SSD ≥1 TB, redundant PSU/network).
- [ ] OS patched and hardened (SSH keys only, firewall restricts RPC/P2P ports, time sync enabled).
- [ ] `ippan-node --config <profile> --check` passes with no warnings.
- [ ] `IPPAN_RPC_HOST=127.0.0.1` (or private IP) and `IPPAN_RPC_ALLOWED_ORIGINS` set to explicit domains.
- [ ] `IPPAN_DATA_DIR` stored on dedicated volume; regular filesystem snapshots enabled if supported.
- [ ] Hourly `ippan-node snapshot export --dir /var/backups/ippan-<ts> --height <tip>` job configured and synced off-host.
- [ ] Prometheus `/metrics` scraped from a private network; alerts for `node_health`, `consensus_round`, and `mempool_size` configured.
- [ ] Validator keys stored with `600` permissions, backed up offline, and rotation documented.
- [ ] Systemd service with restart limits; log forwarding to central SIEM/Loki.

## Gateway & Explorer

- [ ] Reverse proxy terminates TLS (`nginx`, `Caddy`, or cloud LB) with automated certificate renewal.
- [ ] Gateway RPC binds to loopback/private interfaces; firewall blocks direct public access.
- [ ] `RPC_ALLOWED_ORIGINS` enumerates explorer/control domains (no wildcard in production). Config documented in IaC.
- [ ] Rate limits/WAF rules deployed at the proxy (`limit_req`, CDN rules, or cloud WAF).
- [ ] Explorer UI built from trusted source and served as static assets; CSP headers enforced.
- [ ] Monitoring covers latency, 4xx/5xx ratios, and proxy health; `/health` + `/metrics` endpoints polled.
- [ ] Logs (proxy + gateway) forwarded to centralized storage with retention ≥30 days.

## Backups & Disaster Recovery

- [ ] Operators can locate the latest snapshot manifest and off-site copy (S3/GCS/object store) for each validator.
- [ ] [Disaster Recovery Runbook](./docs/operators/disaster-recovery.md) printed / stored with access instructions.
- [ ] Quarterly restore drill completed: import snapshot into staging node, verify height/hash/balances.
- [ ] Configuration and secrets (env files, TLS certs, systemd units) stored in encrypted secrets manager or version-controlled repo.

## Security & Compliance

- [ ] Reverse proxies and RPC hosts run with minimum privileges (`ippan` user, `ProtectSystem=full` where applicable).
- [ ] Secrets (validator keys, RPC auth tokens) never committed to repos; stored in Vault/KMS/HSM.
- [ ] Audit trails enabled: journald persisted, log shipping confirmed, change-management tickets filed for upgrades.
- [ ] `RPC_ALLOWED_ORIGINS` wildcard rejected in production; documented exception process for temporary relaxations.
- [ ] Prometheus endpoints reachable only from monitoring networks; metrics handle disabled if unused.
- [ ] Access control lists for bastion hosts/SSH validated quarterly.

## Documentation Links

- [Validator Runbook](./docs/operators/production-validator-runbook.md)
- [Gateway & Explorer Runbook](./docs/operators/gateway-explorer-runbook.md)
- [Disaster Recovery Runbook](./docs/operators/disaster-recovery.md)

Sign-off:

| Role | Name | Date | Notes |
| --- | --- | --- | --- |
| Validator Lead | | | |
| Infra/SRE Lead | | | |
| Security Lead | | | |
