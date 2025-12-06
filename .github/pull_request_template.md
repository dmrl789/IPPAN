- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` (if applicable)
- [ ] `cargo test -p ippan-consensus-dlc --test fairness_invariants -- --nocapture` (if touched)
- [ ] `cargo run -p ippan-ai-core --bin verify_model_hash -- config/dlc.toml` (if config/model touched)
- [ ] Docs updated (if user-facing)

