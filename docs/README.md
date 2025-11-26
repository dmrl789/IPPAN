# IPPAN Documentation Hub

This directory is the single entrypoint for IPPAN knowledge. Start here to navigate protocol design, operator guidance, deterministic AI/DLC research, audit evidence, and contributor workflows. The trunk branch is currently `master` and will be renamed to `main`; all docs reflect that future rename.

## Index

### Overview
- [Architecture overview](./architecture_overview.md)
- [High-level implementation milestones](./overview/IMPLEMENTATION_COMPLETE_SUMMARY.md)
- [Governance models](./overview/GOVERNANCE_MODELS.md)
- [Component summaries (IPNDHT, P2P, handles)](./overview/README.md)

### Protocol & Economics
- [Protocol specification](./spec/IPPAN_PROTOCOL_SPEC.md)
- [DAG fair emission and economics](./protocol/DAG_FAIR_EMISSION_IMPLEMENTATION_SUMMARY.md)
- [HashTimer ordering analysis](./protocol/HASHTIMER_IMPLEMENTATION_ANALYSIS.md)
- [Consensus improvements and validator rewards](./protocol/CONSENSUS_IMPROVEMENTS_SUMMARY.md)
- [Economics integration and tokenomics](./protocol/ECONOMICS_INTEGRATION_SUMMARY.md) and [Tokenomics](./protocol/TOKENOMICS.md)

### Operators (Deployment & Operations)
- [Localnet Quickstart](./LOCALNET_QUICKSTART.md) - Get localnet running on Windows in <10 minutes
- [Node operator guide](./operators/NODE_OPERATOR_GUIDE.md)
- [Deployment readiness and assessments](./operators/DEPLOYMENT_READY.md)
- [Explorer deployment guide](./operators/EXPLORER_DEPLOYMENT_GUIDE.md)
- [Production rollout and readiness checklists](./operators/PRODUCTION_READINESS_CHECKLIST.md)
- [Secrets and variables setup](./operators/SECRETS_AND_VARIABLES_SETUP_COMPLETE.md)

### AI & DLC
- [AI & DLC overview](./ai_dlc/AI_DLC_OVERVIEW.md)
- [Deterministic math implementation](./ai_dlc/AI_CORE_DETERMINISTIC_MATH_IMPLEMENTATION.md)
- [GBDT and DLC status](./ai_dlc/CONSENSUS_DLC_STATUS.md) and [DLC migration](./ai_dlc/DLC_MIGRATION_COMPLETE.md)
- [Feature set and registry improvements](./ai_dlc/AI_FEATURES_README.md) and [AI registry improvements](./ai_dlc/AI_REGISTRY_IMPROVEMENTS.md)
- Reports and simulations are preserved under [archive](./archive/2025_rc1/).

### Audit & Security
- [Audit index](./audit/AUDIT_INDEX.md)
- [Audit ready checklist](./audit/AUDIT_READY.md)
- [Audit package (RC1)](./audit/AUDIT_PACKAGE_V1_RC1_2025_11_24.md)
- [Dependency audits and resolutions](./audit/CARGO_DEPENDENCY_AUDIT.md)
- [Security threat model](./audit/SECURITY_THREAT_MODEL.md)

### Developer Workflow & CI
- [Trunk workflow (current `master`, future `main`)](./dev_workflow/MAIN_BRANCH_DEVELOPMENT.md)
- [CI/CD fixes and stabilization](./dev_workflow/CI_STABILIZATION_SUMMARY.md)
- [Conflict-resolution history](./dev_workflow/MERGE_CONFLICTS_RESOLUTION.md)
- [Agent protocols and readiness dashboards](./dev_workflow/META_AGENT_PROTOCOL.md) and [READINESS_DASHBOARD_IMPLEMENTATION.md](./dev_workflow/READINESS_DASHBOARD_IMPLEMENTATION.md)

### Archive / Historical Reports
- Historical, dated, or single-run reports are stored in [docs/archive](./archive/) (see [`2025_rc1`](./archive/2025_rc1/)).

## How to Contribute

- Update or add docs under the appropriate folder above.
- When renaming files, update links in this README and any referencing documents.
- Keep references to the trunk branch phrased as "trunk branch (currently `master`, will be `main`)" to stay future-proof.
