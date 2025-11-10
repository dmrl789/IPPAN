# Consensus Documentation Module

The `docs/consensus` module captures the deterministic consensus research that underpins IPPAN's BlockDAG architecture and validator workflow.

## Contents

- [`ippan_block_creation_validation_consensus.md`](./ippan_block_creation_validation_consensus.md) — Detailed walkthrough of block creation, validation sequencing, and how the Deterministic Learning Consensus (DLC) interacts with HashTimer scheduling.

## How to Use These Notes

- Start with the overview diagrams in `docs/diagrams` and return here for the narrative that explains validator responsibilities and quorum thresholds.
- Cross-reference implementation details in `crates/consensus` to ensure the code matches the documented state-machine transitions.
- Use this document as the source of truth when aligning validator APIs, telemetry metrics, or economics with the consensus pipeline.

## Maintenance Guidelines

- Update the consensus narrative whenever new validator states, penalties, or message types are introduced.
- Include round numbers, timing guarantees, and deterministic ordering rules for any new flow to keep downstream teams aligned.
- Link any supporting research or simulations from `docs/issues` or external repositories so future reviewers can trace design decisions.
