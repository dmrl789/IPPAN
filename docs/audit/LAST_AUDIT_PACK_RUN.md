# Last Audit Pack Run (Evidence)

**Date (UTC):** 2025-12-06 07:02:46 UTC

**Commit SHA:** f0427351743146f0f3f5c67c61165caa7f094a4a

**Workflow Run ID:** 19984918333

**Run URL:** https://github.com/dmrl789/IPPAN/actions/runs/19984918333

**Artifacts:** audit-pack-artifacts
- `sbom.json` - Software Bill of Materials (CycloneDX format)
- `cargo-deny.txt` - Dependency security and license checks
- `gates.log` - Format, clippy, and test results
- `verify_model_hash.log` - Model hash verification output

## Results

### Gate Status

- ✅ **fmt:** PASS
- ✅ **clippy:** PASS
- ✅ **tests:** PASS
- ✅ **verify_model_hash:** PASS
- ✅ **cargo-deny:** PASS
- ✅ **SBOM generated:** PASS

### Summary

All P0 gates passed successfully. The audit pack workflow completed in 7m55s with no failures.

**Job Status:** ✓ Success

**Duration:** 7m55s

## Notes

- **Retention Policy:** Artifacts are retained for 21 days
- **Workflow:** Triggered via `workflow_dispatch` on `master` branch
- **No warnings or errors detected**

---

*This document records evidence of the latest audit-pack workflow execution for audit readiness verification.*

