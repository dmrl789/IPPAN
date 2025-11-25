Checked the conflicts page — PR #282 currently has merge conflicts with base `main` (mergeable_state: dirty).

Proposed minimal resolutions (keep both `main` and this PR’s additions):

1) Cargo.toml — append new workspace members:
```diff
diff --git a/Cargo.toml b/Cargo.toml
@@
   "crates/core",
   "crates/time",
+  "crates/ai_core",
+  "crates/ai_registry",
   "node",
 ]
```

2) crates/consensus/Cargo.toml — add dependency:
```diff
@@
 ippan-mempool = { path = "../mempool" }
+ippan-ai-core = { path = "../ai_core" }
 anyhow = { workspace = true }
```

3) .github/workflows/ci.yml — add AI Core determinism tests step after the build-only test:
```diff
@@
-      - name: Cargo test (build only)
-        run: cargo test --workspace --all-features --no-run
+      - name: Cargo test (build only)
+        run: cargo test --workspace --all-features --no-run
+
+      - name: AI Core determinism tests
+        run: |
+          cargo test -p ippan-ai-core -- --nocapture
```

After resolving, please:
- run `cargo update -w` to refresh `Cargo.lock`
- run `cargo fmt && cargo clippy -D warnings && cargo test`

If you’d like, I can rebase this branch onto `main` and push the resolved state.