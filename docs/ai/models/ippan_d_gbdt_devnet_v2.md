# Model: ippan_d_gbdt_devnet_v2

- model_id: ippan_d_gbdt_devnet_v2
- model_hash: 150167882b7f598dcb54df2d6853790202c9d6ee4a79ba6348576c093499179c
- trainer commit: ffeb0c2a10874ab82b5a7b1b51053ecf3a5b2441
- training datasets:
  - devnet_dataset_20251217T181610Z.csv.gz
  - devnet_dataset_20251218T001926Z.csv.gz
  - devnet_dataset_20251218T062318Z.csv.gz
- training date: 2025-12-18 (UTC)
- trainer script: ai_training/train_ippan_d_gbdt_devnet.py
- metrics:
  - validation MSE: 0.000000 (90/30 deterministic split)
  - notes: dataset appears highly regular; treat “0.0” as a **sanity signal** (pipeline working) not as evidence of generalization.
- determinism:
  - platforms tested:
    - laptop (Python trainer output)
    - laptop (Rust `compute_model_hash`)
    - devnet nodes (Rust `ippan-ai-core` canonical loader via consensus_dlc)
  - hashes match: yes (Python `model_hash` == Rust `compute_model_hash` == devnet `/status.ai.shadow_model_hash`)
- deployment:
  - promoted to: devnet (shadow only)
  - promotion date: 2025-12-18
  - rollout:
    - canary: 5.223.51.238
    - remaining: 188.245.97.41, 135.181.145.174, 178.156.219.107
  - runtime wiring:
    - `config/dlc.toml` keeps `[dgbdt.model]` as active
    - `[dgbdt.shadow_model]` points to `ai_assets/models/devnet/ippan_d_gbdt_devnet_v2.json`
    - nodes set `IPPAN_DLC_CONFIG_PATH=/opt/ippan/config/dlc.toml` so the shadow loader reads the repo-pinned config
    - verification signal: `GET /status` includes `.ai.shadow_loaded=true` and `.ai.shadow_model_hash=<hash>`
  - rollback plan:
    - remove or comment `[dgbdt.shadow_model]` (or set `IPPAN_DLC_CONFIG_PATH` back to the previous path)
    - restart `ippan-node` on devnet nodes

## Promotion gates (shadow → active)

Before promoting this model to active (decision-impacting) on devnet:

- **Shadow window**: ≥24h shadow scoring with stable hash across all 4 nodes.
- **Offline evaluation**: compare vs active model; record metrics + qualitative differences.
- **Determinism**: `verify_model_hash` must pass for both active + shadow; no drift across nodes.
- **Go/no-go**: if any instability or unexpected distribution shift, keep active model and record reasons.


