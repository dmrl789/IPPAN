# AI Roadmap (D-GBDT)

## TODO (vNext)

- [x] Define devnet training spec + model metadata template (`docs/ai/DGBDT_TRAINING_SPEC.md`, `docs/ai/models/_TEMPLATE.md`).
- [ ] Devnet training entrypoint + helper scripts (in progress):
  - `ai_training/train_ippan_d_gbdt_devnet.py`
  - `scripts/ai/train-dgbdt-devnet.sh`
  - `scripts/ai/train-and-verify-devnet.sh`
- [ ] Implement `train_ippan_d_gbdt_devnet.py` using `docs/ai/DGBDT_TRAINING_SPEC.md`.
- [ ] Add CI job for “train-then-verify hash” on a small sample dataset.
- [ ] Add CLI or script to promote a model: copy JSON → update `config/dlc.toml` → run hash verifier → `rollout-devnet.sh` verify-only.
- [ ] Document the first vNext promotion in `docs/ai/models/ippan_d_gbdt_devnet_v2.md`.


