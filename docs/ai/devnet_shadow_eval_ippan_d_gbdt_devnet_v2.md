# DevNet D-GBDT Shadow Model Evaluation (ippan_d_gbdt_devnet_v2)

## Goal

Run a 24–72h shadow period where the new model:

- `ippan_d_gbdt_devnet_v2`
- shadow hash: `150167882b7f598dcb54df2d6853790202c9d6ee4a79ba6348576c093499179c`

is loaded on all devnet nodes **without changing live decisions**, and verify:

- Shadow model is loaded and stable on all nodes
- Shadow model hash matches expected BLAKE3 on all nodes
- No systemic AI/runtime errors appear during the window

## Toolkit

Scripts:

- `scripts/ai/shadow_eval/collect_shadow_status_once.sh`
- `scripts/ai/shadow_eval/run_shadow_observation_loop.sh`
- `scripts/ai/shadow_eval/analyze_shadow_status.py`

Outputs:

- JSON snapshots under `logs/ai_shadow/`

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

3) Analyze health after collecting data:

```bash
export EXPECTED_SHADOW_HASH="150167882b7f598dcb54df2d6853790202c9d6ee4a79ba6348576c093499179c"
./scripts/ai/shadow_eval/analyze_shadow_status.py
```

## Promote / No-go criteria (online layer)

- **Promote candidate** if, over the observation window:
  - `shadow_loaded=true` for `>= 99%` of samples per node
  - All observed `shadow_model_hash` values match the expected hash
  - No repeated/systemic errors visible in `/ai/status` and node logs

- **No-go / investigate first** if:
  - Any node reports a different `shadow_model_hash`
  - `shadow_loaded` is `false` or missing in a significant subset of samples
  - Frequent AI subsystem errors are visible in logs or status endpoints

## Offline checks (recommended before active promotion)

Online shadow evaluation confirms **health & determinism**. Before promoting to active,
run offline comparisons on devnet datasets:

- Score distribution drift vs current active model
- Ranking stability for top validators
- Tail risk / impact on lowest-scored validators


