# Issues & Research Notes

The `docs/issues` module collects design explorations, scale plans, and open research topics that inform future milestones. These documents serve as living references until their proposals graduate into formal specifications or implementation tickets.

## Contents

- [`blockdag-scale-plan.md`](./blockdag-scale-plan.md) — Roadmap for scaling the BlockDAG, covering validator scheduling, data availability, and performance guardrails for 10M TPS targets.

## Using This Module

- Treat each document as an RFC in progress. When work is accepted or implemented, link the resulting PRs or specs and note the decision outcome at the top of the file.
- Reference relevant modules (e.g., consensus, economics) so readers can trace how proposals impact other domains.
- When proposing modifications, summarize assumptions, dependencies, and potential risks in the opening section to fast-track reviews.

## Maintenance Guidelines

- Prefix each document with a revision date (`Last updated: YYYY-MM-DD`) to make stale context obvious.
- Archive superseded plans by linking to the canonical replacement rather than deleting content; this preserves decision history.
- Ensure diagrams or metrics referenced here also live in `docs/diagrams` or `artifacts/` so they can be versioned alongside text.
