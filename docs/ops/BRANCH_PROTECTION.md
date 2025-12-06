# Branch Protection Settings

This document describes the exact branch protection settings to apply on GitHub for the `master` branch.

## How to Apply

1. Navigate to: **Settings → Branches → Branch protection rules → Add rule**
2. **Branch name pattern:** `master`
3. Apply the settings below

## Required Settings

### Pull Request Requirements

- ✅ **Require a pull request before merging**
  - ✅ **Require approvals:** 1 (or 2 if preferred)
  - ✅ **Dismiss stale approvals on new commits**
  - ✅ **Require conversation resolution**

### Status Checks

- ✅ **Require status checks to pass before merging**
  - Select these checks (names must match your Actions):
    - `Build & Test (Rust)`
    - `AI Determinism & DLC Consensus` (when applicable)
    - `Fuzz — Smoke` (optional but recommended)
  - ✅ **Require branches to be up to date before merging**

### Branch Restrictions

- ✅ **Restrict who can push to matching branches** (only repository owner/admin)
- ✅ **Do not allow force pushes**
- ✅ **Do not allow deletions**

### Optional: Tag Protection

- Protect `v*` tags (Settings → Tags → Add rule)
  - Pattern: `v*`
  - Restrict who can create/delete tags

## Handling Workflows with Path Filters

Some workflows (e.g., `AI Determinism & DLC Consensus`) may be skipped due to path filters. In this case:

- If a workflow is skipped due to paths, it should **not block merge**
- If GitHub requires strict checks, either:
  - Disable "require" for that workflow in branch protection, OR
  - Split into a lightweight always-on check that always runs

## Notes

- Keep branch protection rules minimal but effective
- Focus on critical gates: fmt, clippy, tests, model hash verification
- Allow flexibility for optional checks (fuzz, soak) to be skipped without blocking
