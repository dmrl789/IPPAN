# Audit Pack Workflow

The **Audit Pack** workflow is a manual, one-click workflow that runs all P0 readiness gates and generates audit artifacts (SBOM, dependency reports, logs) for external auditors and release preparation.

## How to Run

1. Navigate to **Actions** → **Audit Pack — P0 Gates + SBOM**
2. Click **Run workflow**
3. (Optional) Configure inputs:
   - **rust_toolchain**: Rust toolchain version (default: `stable`)
   - **clippy_all_features**: Run clippy with `--all-features` (default: `true`)
4. Click **Run workflow**

The workflow runs for approximately 10-30 minutes depending on cache state.

## What You Get

After the workflow completes, download the **audit-pack-artifacts** artifact from the workflow run. It contains:

### Artifacts

- **`sbom.json`** - Software Bill of Materials (CycloneDX format)
  - Complete dependency tree with versions
  - License information per dependency
  - Used for security audits and compliance

- **`cargo-deny.txt`** - Dependency policy report
  - Security advisories check
  - License compliance verification
  - Banned crate detection
  - Source registry validation

- **`gates.log`** - P0 gate execution logs
  - Format check results
  - Clippy lint output
  - Test execution summary

- **`verify_model_hash.log`** - AI model hash verification
  - Confirms D-GBDT model matches expected hash
  - Validates deterministic AI model integrity

## What "Pass" Means

The workflow **passes** when:

✅ **Format check**: All Rust code is properly formatted  
✅ **Clippy**: No warnings (all warnings denied)  
✅ **Tests**: All workspace tests pass  
✅ **Model hash**: D-GBDT model hash matches `config/dlc.toml`  
✅ **SBOM**: Generated successfully (CycloneDX JSON)  
✅ **cargo-deny**: No security advisories, license violations, or banned crates

If any step fails, the workflow fails and artifacts are still uploaded for analysis.

## Auditor Checklist

When running an audit, use this workflow to:

1. **Generate baseline artifacts** before code review
2. **Verify dependency policy** compliance (see `cargo-deny.txt`)
3. **Review SBOM** for unexpected dependencies or license issues
4. **Confirm P0 gates** are passing (see `gates.log`)
5. **Validate model integrity** (see `verify_model_hash.log`)

## Related Documentation

- **Readiness Plan**: [docs/READINESS_100_PLAN.md](../READINESS_100_PLAN.md) - Complete P0/P1 tracking
- **Documentation Index**: [docs/INDEX.md](../INDEX.md) - Single entry point to all docs
- **Audit Index**: [docs/audit/AUDIT_INDEX.md](AUDIT_INDEX.md) - Audit entry point
- **Dependency Policy**: [deny.toml](../../deny.toml) - Cargo-deny configuration

## Workflow Details

### P0 Gates Executed

1. **Format check**: `cargo fmt --all -- --check`
2. **Clippy lint**: `cargo clippy --all --all-targets --all-features -- -D warnings`
3. **Tests**: `cargo test --all --all-targets --all-features`
4. **Model hash verification**: `cargo run -p ippan-ai-core --bin verify_model_hash -- config/dlc.toml`

### Artifact Generation

- **SBOM**: Generated via `cargo-cyclonedx` (CycloneDX JSON format)
- **Dependency report**: Generated via `cargo-deny` (advisories, bans, licenses, sources)

### Disk Space Management

The workflow uses `jlumbroso/free-disk-space@v1.3.1` to free runner disk space and does **not** cache the `target/` directory to minimize disk usage.

### Retention

Artifacts are retained for **21 days** after workflow completion.

---

**Questions?** See [docs/INDEX.md](../INDEX.md) for the complete documentation index.

