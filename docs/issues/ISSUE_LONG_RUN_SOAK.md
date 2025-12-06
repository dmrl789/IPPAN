# P0: Long-run determinism + DLC soak tests (scheduled/manual)

## Priority: P0 (Release Blocker)

## Description
Add extended determinism and DLC stability tests that run for hours (not minutes) to validate long-term stability and detect any determinism drift.

## Requirements
- New workflow file or extended job in existing workflow
- Use `workflow_dispatch` + `schedule` (weekly) for manual and scheduled runs
- Separate job that runs extended tests without timeout
- Store determinism logs, DLC state snapshots, and metrics as artifacts

## Acceptance Criteria
- [ ] Workflow can run 4+ hour soak tests
- [ ] No determinism drift detected over extended runs
- [ ] DLC consensus remains stable (no forks, no validator selection anomalies)
- [ ] Results archived and accessible

## Related
See `docs/READINESS_100_PLAN.md` section B.1 for full details.

