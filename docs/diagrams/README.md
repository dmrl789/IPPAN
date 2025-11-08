# Diagram Library

This folder stores the source-of-truth diagrams that illustrate IPPAN economics, consensus flows, and deterministic learning mechanics. Each Mermaid (`.mmd`) file renders to a corresponding SVG for inclusion in documentation and presentations.

## Available Diagrams

- `dag_fair_emission_curve.mmd` / `.svg` — Emission schedule curve for the DAG Fair Emission system.
- `dag_fair_round_reward_flow.mmd` / `.svg` — Reward distribution pipeline per consensus round.
- `deterministic_learning_consensus.mmd` — State machine for Deterministic Learning Consensus interactions (SVG generated on demand).

## Editing & Export Workflow

1. Install the Mermaid CLI (once per machine):
   ```bash
   npm install --global @mermaid-js/mermaid-cli
   ```
2. Regenerate an SVG after updating a Mermaid source:
   ```bash
   mmdc -i dag_fair_emission_curve.mmd -o dag_fair_emission_curve.svg
   ```
   Repeat for each diagram to keep the rendered assets in sync with the source.
3. Commit both the `.mmd` and `.svg` versions so downstream consumers do not need to regenerate assets.

## Contribution Guidelines

- Favor reusable components (subgraphs, class definitions) to keep diagrams maintainable.
- Keep labels deterministic and version agnostic; when required, add revision dates in the footer rather than the title.
- Reference the diagram from related documentation (for example, link to emission diagrams in `DAG_FAIR_EMISSION_SYSTEM.md`) so editors know when updates are needed.
