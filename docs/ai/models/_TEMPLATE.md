# Model: ippan_d_gbdt_devnet_vX

- model_id: ippan_d_gbdt_devnet_vX
- model_hash: <BLAKE3 hex>
- trainer commit: <git SHA>
- training datasets:
  - devnet_dataset_YYYYMMDDT*.csv.gz, ...
- training date: YYYY-MM-DD (UTC)
- trainer script: ai_training/train_ippan_d_gbdt.py
- metrics:
  - AUC:
  - calibration:
  - other:
- determinism:
  - platforms tested:
  - hashes match: yes/no
- deployment:
  - promoted to: devnet / mainnet
  - promotion date:
  - rollback plan:


