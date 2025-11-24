# IPPAN Documentation Hub

This directory houses canonical references for the IPPAN network. Documentation is grouped by discipline so contributors can quickly locate specifications, runbooks, and product context without leaving the repository.

## Quick Start for Developers

- Read the [Developer Guide](./DEVELOPER_GUIDE.md) for environment setup, builds, and workflow expectations.
- Understand how wallet addresses and `@handles` relate via [`users/handles-and-addresses.md`](./users/handles-and-addresses.md).
- Learn the wallet & signing flows in [`dev/wallet-cli.md`](./dev/wallet-cli.md) before wiring payments or SDKs.
- Spin up the [local full-stack environment](./dev/local-full-stack.md) to run a node, gateway, and explorer with one command.
- Build the Rust workspace with `cargo check --workspace` before you branch into feature work.
- Run the unified UI locally (`apps/unified-ui`) with `npm install && npm run dev` to validate end-to-end flows when your change touches the frontend.
- Keep the Agent Charter ([`.cursor/AGENT_CHARTER.md`](../.cursor/AGENT_CHARTER.md)) in mind—determinism, reproducibility, and scope isolation apply to documentation updates too.

## Document Families

- **Protocol & Architecture** – Deep technical references such as [`AI_IMPLEMENTATION_GUIDE.md`](./AI_IMPLEMENTATION_GUIDE.md), [`CONSENSUS_RESEARCH_SUMMARY.md`](./CONSENSUS_RESEARCH_SUMMARY.md), and [`DAG_FAIR_EMISSION_SYSTEM.md`](./DAG_FAIR_EMISSION_SYSTEM.md).
- **Product Requirements** – Strategic and functional direction captured under [`prd/`](./prd/README.md).
- **Operational Runbooks** – Deployment and maintenance playbooks including [`automated-deployment-guide.md`](./automated-deployment-guide.md) and [`server-health-check.md`](./server-health-check.md).
- **Visual Resources** – Mermaid and SVG diagrams in [`diagrams/`](./diagrams/README.md) for architecture storytelling and reviews.
- **Research & Open Issues** – Exploratory work and design proposals under [`issues/`](./issues/README.md).

## Module Index

- [`consensus/`](./consensus/README.md) – Deterministic Learning Consensus validation flows and BlockDAG mechanics.
- [`prd/`](./prd/README.md) – Product requirements documents and their change management process.
- [`diagrams/`](./diagrams/README.md) – Authoritative diagram sources and export workflow.
- [`issues/`](./issues/README.md) – Active RFCs, scale plans, and research notes awaiting implementation.

## Maintenance Guidelines

- Link new documents from the closest relevant module README so other contributors can discover them quickly.
- When updating specifications, include version context or revision dates in the document heading.
- Keep examples deterministic and executable; avoid placeholder code that could mislead downstream teams.
- Run spell-check or lint tools in your editor where possible to keep the knowledge base clean.
