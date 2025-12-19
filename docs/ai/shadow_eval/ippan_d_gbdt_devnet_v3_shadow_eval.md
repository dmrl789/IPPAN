# DevNet D-GBDT Shadow Model Evaluation (ippan_d_gbdt_devnet_v3)

## Goal

Run a shadow period where the new model:

- model_id: `ippan_d_gbdt_devnet_v3`
- shadow hash: `994993e4ec1a6bb01dc5c9a3f1ce52cca7dad58f673385c7cf2c8b00d0c6c270`

is loaded on all devnet nodes **without changing live decisions**, and verify:

- Shadow model is loaded and stable on all nodes
- Shadow model hash matches expected BLAKE3 on all nodes (during the **clean** window)
- No systemic AI/runtime errors appear during the window

## Toolkit

Scripts:

- `scripts/ai/shadow_eval/collect_shadow_status_once.sh`
- `scripts/ai/shadow_eval/run_shadow_observation_loop.sh`
- `scripts/ai/shadow_eval/analyze_shadow_status.py`
- `scripts/ai/shadow_eval/run_devnet_v3_shadow_analysis.sh`

Outputs:

- JSON snapshots under `logs/ai_shadow/`
- Analyzer log: `scripts/ai/shadow_eval/devnet_v3_shadow_eval_current.log`

## Clean devnet v3 shadow observation window

The `logs/ai_shadow/` directory contains historical snapshots where the shadow hash
was still transitioning (older snapshots include the devnet v2 hash
`150167882b7f598dcb54df2d6853790202c9d6ee4a79ba6348576c093499179c`).

For devnet v3 integrity analysis, we define a **clean** window starting from the
earliest snapshot timestamp where **all 4 nodes** reported the expected devnet v3
shadow hash.

- **Model hash**: `994993e4ec1a6bb01dc5c9a3f1ce52cca7dad58f673385c7cf2c8b00d0c6c270`
- **Source logs**: `logs/ai_shadow/`
- **Analyzer log**: `scripts/ai/shadow_eval/devnet_v3_shadow_eval_current.log`
- **First-good per node (EXPECTED hash)**:
  - `root_188.245.97.41`: `2025-12-19T09:59:20Z`
  - `root_135.181.145.174`: `2025-12-19T12:58:16Z`
  - `root_178.156.219.107`: `2025-12-19T09:59:20Z`
  - `root_5.223.51.238`: `2025-12-19T09:59:20Z`
- **Clean 24h window start**:
  - `WINDOW_START = 2025-12-19T12:58:16Z`

The integrity analysis for devnet v3 should treat this timestamp as the start of
the “clean” period in which all four nodes were running the expected shadow model.

## Online checks (24–72h)

From repo root:

1) One-shot snapshot:

```bash
./scripts/ai/shadow_eval/collect_shadow_status_once.sh
```

2) Observation loop (example: ~24h at one sample / 10 minutes):

```bash
./scripts/ai/shadow_eval/run_shadow_observation_loop.sh 144 600
```

3) Mid-run analysis (can be repeated as counts rise):

```bash
export EXPECTED_SHADOW_HASH="994993e4ec1a6bb01dc5c9a3f1ce52cca7dad58f673385c7cf2c8b00d0c6c270"
export LOG_DIR="logs/ai_shadow"
./scripts/ai/shadow_eval/run_devnet_v3_shadow_analysis.sh
```

## Promote / No-go criteria (online layer)

- **Promote candidate** if, over the clean window:
  - `shadow_loaded=true` for `>= 99%` of samples per node
  - All observed `shadow_model_hash` values match the expected hash
  - No repeated/systemic errors visible in `/ai/status` and node logs

- **No-go / investigate first** if:
  - Any node reports a different `shadow_model_hash` during the clean window
  - `shadow_loaded` is `false` or missing in a significant subset of samples
  - Frequent AI subsystem errors are visible in logs or status endpoints


