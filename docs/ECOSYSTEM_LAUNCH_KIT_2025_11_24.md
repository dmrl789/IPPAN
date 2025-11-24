# IPPAN Ecosystem Launch Kit — 2025-11-24

Use this checkpoint document to onboard every persona involved in the IPPAN launch. Each section links to the canonical runbooks and includes a concrete outcome checklist.

---

## Validator operators start here

**Key docs**

- [Production Validator Runbook](operators/production-validator-runbook.md)
- [Disaster Recovery Runbook](operators/disaster-recovery.md)
- [Production Readiness Checklist](../PRODUCTION_READINESS_CHECKLIST.md)
- [Storage & Snapshots](STORAGE_AND_SNAPSHOTS.md)
- [Performance Report (2025-11-24)](performance/PERFORMANCE_REPORT_2025_11_24.md)

**Runbook**

1. Provision hardware and harden the OS per the Validator Runbook.
2. Configure `IPPAN_RPC_HOST`, `IPPAN_RPC_ALLOWED_ORIGINS`, and data directories; run `ippan-node --check`.
3. Start the node (`systemctl start ippan`) and verify `/health` plus `/metrics`.
4. Schedule hourly snapshots (`ippan-node snapshot export --dir ... --height <tip>`) and sync them off-site.
5. Review the latest performance baseline and compare local `scripts/run-benchmarks.sh` output before declaring the node ready.
6. Complete every item in the Production Readiness Checklist, then log the sign-off in your change tracker.

---

## Gateway/explorer operators start here

**Key docs**

- [Gateway & Explorer Runbook](operators/gateway-explorer-runbook.md)
- [Handles & Addresses](users/handles-and-addresses.md)
- [Production Readiness Checklist](../PRODUCTION_READINESS_CHECKLIST.md) (Gateway section)
- [Security Guide](SECURITY_GUIDE.md)

**Runbook**

1. Deploy a reverse proxy (nginx/Caddy) with TLS certificates for explorer domains.
2. Configure the gateway/explorer services to bind to loopback and set `RPC_ALLOWED_ORIGINS` to explicit domains.
3. Point the explorer UI (`NEXT_PUBLIC_IPPAN_RPC_BASE`) at the gateway ingress and confirm handle resolution works.
4. Enable rate limiting/WAF rules and forward logs to your SIEM.
5. Scrape `/metrics` from a private network and configure alerts for health + latency thresholds.
6. Capture the current configuration in IaC and reference the Production Readiness Checklist before go-live.

---

## Application developers start here

**Key docs**

- [Developer Journey](dev/developer-journey.md)
- [Local Full-Stack Guide](dev/local-full-stack.md)
- [SDK Overview](dev/sdk-overview.md)
- [Handles & Addresses](users/handles-and-addresses.md)

**Runbook**

1. Clone the repo, install Rust/Node, and run `scripts/run-local-full-stack.sh` to boot a three-node devnet.
2. Generate a wallet + handle via `ippan-wallet` and fund it using the local faucet/devnet helper.
3. Integrate the Rust or TypeScript SDK following the samples; run the provided example programs to issue payments.
4. Inspect `/health`, `/tx`, and `/block` endpoints to understand the JSON contracts.
5. Extend your service/application logic and keep the localnet logs open to debug interactions.

---

## End users start here

**Key docs**

- [User Getting Started Guide](users/getting-started.md)
- [Handles & Addresses](users/handles-and-addresses.md)
- [Gateway & Explorer Runbook](operators/gateway-explorer-runbook.md) (for available endpoints)

**Runbook**

1. Download the latest `ippan-wallet` binary, generate a key, and store it securely.
2. Obtain devnet/testnet IPN from the faucet or operator and confirm the balance via RPC or explorer.
3. Register an `@handle.ipn` if desired and share it with other testers.
4. Send and receive payments using the CLI, explorer UI, or SDK-backed apps.
5. Report issues or feedback to the operator team before mainnet go-live.

---

## Link map

| Topic | Reference |
| --- | --- |
| Node configuration profiles | `config/*.toml` + [Production Validator Runbook](operators/production-validator-runbook.md) |
| Wallet CLI | [User Getting Started](users/getting-started.md) |
| Handles | [Handles & Addresses](users/handles-and-addresses.md) |
| Local full stack | [Local Full-Stack Guide](dev/local-full-stack.md) |
| SDKs | [SDK Overview](dev/sdk-overview.md) |
| Developer workflow | [Developer Journey](dev/developer-journey.md) |
| Disaster recovery | [Disaster Recovery Runbook](operators/disaster-recovery.md) |
| Gateway operations | [Gateway & Explorer Runbook](operators/gateway-explorer-runbook.md) |
| Performance references | [Performance Report 2025-11-24](performance/PERFORMANCE_REPORT_2025_11_24.md) |
