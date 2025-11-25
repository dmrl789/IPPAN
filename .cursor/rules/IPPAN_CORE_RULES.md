# IPPAN CORE RULES (AUTO-APPLIED IN CURSOR)

## Always

- Work only on branch `master`. Do not create or use other branches.

- Do not modify .github/workflows/* or toolchain files unless explicitly instructed.

- Do not introduce f32/f64 in any runtime/consensus Rust crates.

- Prefer deterministic iteration and stable ordering in consensus paths.

- List files touched + test steps in every summary.

## Rust Safety

- If a change touches crates/*, run a quick scan for floats before committing:

  - `rg -n "\\bf(32|64)\\b" crates` (or equivalent)

- Never add floats "temporarily".

