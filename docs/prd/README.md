# IPPAN Product Requirements Documents (PRDs)

This directory contains the complete set of Product Requirements Documents for the IPPAN blockchain project.

---

## Document Index

### 1. [IPPAN Vision 2025](./ippan-vision-2025.md) — **Master PRD**
*Comprehensive product vision and strategic objectives*

**Scope:** High-level product vision, business objectives, competitive positioning, and roadmap

**Key Topics:**
- Core objectives (10M TPS, deterministic AI, microsecond timing)
- HashTimer™ and IPPAN Time architecture
- Embedded AI architecture (L1 + L2)
- Tokenomics and monetary policy
- Governance framework
- Environmental and ethical principles
- Implementation roadmap (Q4 2025 → Q4 2026)
- Comparative landscape vs. Bitcoin, Ethereum, Solana, etc.

**Audience:** Executives, investors, partners, community

---

### 2. [IPPAN L1 Architecture](./ippan-l1-architecture.md) — **Technical Specification**
*Layer-1 blockchain architecture, consensus, and data allocation*

**Scope:** Technical design of the Layer-1 blockchain, parallel consensus, and L1/L2 separation

**Key Topics:**
- Global Layer-1 design principles
- BlockDAG + parallel block creation
- Round-based finalization (100–250 ms)
- L1 vs L2 data allocation strategy
- Confidentiality and compliance model
- DNS and human-readable identity (`@user.ipn`)
- Scalability targets (1–10M TPS)
- Developer reference types (Rust)

**Audience:** Core developers, protocol engineers, validator operators

---

### 3. [IPPAN Storage & Data Availability](./ippan-storage-da.md) — **Storage PRD**
*Data availability, pruning, fast sync, and confidential transactions*

**Scope:** Block storage model, erasure coding, retention policies, and privacy

**Key Topics:**
- Block data layout (headers vs. bodies)
- Data availability via Reed–Solomon erasure coding
- Retention model (validator, full node, archival node)
- Fast sync procedure with snapshot checkpoints
- Confidential transactions and mixed visibility
- ZK-STARK integration roadmap (Phases 0–3)
- Economic incentives (announcement fees, serving rewards, archival contracts)
- Networking/RPC interfaces

**Audience:** Storage engineers, infrastructure teams, privacy specialists

---

## Document Relationships

```
┌─────────────────────────────────────────────────┐
│   ippan-vision-2025.md                          │
│   (Master PRD — Product Strategy & Vision)      │
└────────────────┬────────────────────────────────┘
                 │
       ┌─────────┴──────────┐
       │                    │
       ▼                    ▼
┌─────────────────┐  ┌──────────────────────┐
│ ippan-l1-       │  │ ippan-storage-da.md  │
│ architecture.md │  │ (Storage & DA Spec)  │
│ (L1 Tech Spec)  │  └──────────────────────┘
└─────────────────┘
```

- **Vision PRD** defines the "what" and "why" — business goals, competitive positioning, and success criteria
- **L1 Architecture PRD** defines the "how" — consensus, parallel blocks, rounds, and deterministic ordering
- **Storage & DA PRD** defines the "where" and "when" — data retention, availability proofs, and confidentiality

---

## Usage Guidelines

### For New Contributors
Start with the **Vision PRD** to understand IPPAN's mission and unique value proposition, then dive into the technical PRDs for implementation details.

### For Implementing New Features
1. Check if the feature aligns with the **Vision PRD** objectives
2. Reference the **L1 Architecture PRD** for consensus and block structure
3. Consult the **Storage & DA PRD** for data handling and retention policies
4. If the feature requires a new PRD, follow the template in `AGENTS.md` section 10

### For Governance Proposals
- Protocol parameter changes → reference **L1 Architecture PRD**
- AI model updates → reference **Vision PRD** section 6.3
- Storage/retention policy changes → reference **Storage & DA PRD**

---

## Version History

| Document | Version | Date | Maintainer |
|----------|---------|------|------------|
| ippan-vision-2025.md | 2025 | 2025-10-22 | IPPAN Foundation / dmrl789 |
| ippan-l1-architecture.md | October 2025 | 2025-10 | IPPAN Core Team |
| ippan-storage-da.md | October 2025 | 2025-10 | IPPAN Storage Team |

---

## Contributing

When updating PRDs:
1. Maintain backward compatibility with existing implementations where possible
2. Update the version history table above
3. Cross-reference related PRDs to maintain consistency
4. If introducing breaking changes, create an ADR (Architecture Decision Record) in `docs/adr/`

---

**License:** CC-BY-SA-4.0  
**Last Updated:** 2025-10-22
