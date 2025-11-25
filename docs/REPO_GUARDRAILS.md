# Repository Guardrails

## Cursor AI Rules

Cursor IDE automatically applies rules from `.cursor/rules/*.md`:
- `IPPAN_CORE_RULES.md`: Core development rules (master-only, no floats in runtime)
- `IPPAN_AI_TRAINING_RULES.md`: AI training policy (offline only, LFS for large data)

## No Floats in Runtime Policy

Runtime and consensus Rust crates (`crates/*`) must never use `f32` or `f64`.

**Enforcement**: Opt-in pre-commit hook at `.githooks/pre-commit` scans `crates/` for floats (excluding tests/examples/benches).

**To enable the hook**:
```bash
./.githooks/install.sh
```

This sets `git config core.hooksPath .githooks` so Git uses the versioned hooks.

## Large Files Policy

GitHub rejects normal pushes for files >100MB. Large artifacts must be:

1. **Placed only in `ai_assets/`** (dedicated folder)
2. **Tracked with Git LFS**

**Setup Git LFS** (one-time):
```bash
git lfs install
```

**Usage**: Keep large datasets, model artifacts, and archives under `ai_assets/`. The `.gitattributes` file configures LFS tracking for common large file types in that folder.

**Note**: If Git LFS is not installed, you must install it on your machine before pushing large files.

