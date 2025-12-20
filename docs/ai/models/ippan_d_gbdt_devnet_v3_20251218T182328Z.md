# Model: ippan_d_gbdt_devnet_v3

- model_id: ippan_d_gbdt_devnet_v3
- model_hash: 994993e4ec1a6bb01dc5c9a3f1ce52cca7dad58f673385c7cf2c8b00d0c6c270
- training date: 2025-12-18 (UTC)
- trainer script: ai_training/train_ippan_d_gbdt_devnet.py

#### Clean devnet v3 shadow window

The shadow evaluation for this model uses a **clean 24h window** starting at:

- `WINDOW_START = 2025-12-19T12:58:16Z`

### Shadow evaluation (DevNet v3)

- Shadow model hash: `994993e4ec1a6bb01dc5c9a3f1ce52cca7dad58f673385c7cf2c8b00d0c6c270`
- Target clean-window: **2025-12-19T12:58:16Z → 2025-12-20T12:58:16Z**
- Observed logs window: **2025-12-19T12:58:16Z → 2025-12-20T11:26:11Z**
- Full analyzer output: `docs/devnet_v3_shadow_eval_FINAL_clean_24h.txt`
- Detailed methodology and node-by-node stats:
  - `docs/ai/shadow_eval/ippan_d_gbdt_devnet_v3_shadow_eval.md`
  - `docs/devnet_v3_shadow_eval_FINAL_clean_24h.txt`
- Result: **OK** for all sampled snapshots in the observed window: all report `shadow_loaded=true` and the expected hash on all 4 DevNet validators.

See the detailed shadow evaluation report for per-node first-good times and integrity statistics:

- `docs/ai/shadow_eval/ippan_d_gbdt_devnet_v3_shadow_eval.md`


