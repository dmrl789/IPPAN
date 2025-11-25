# OFFLINE ONLY: This training script is for bootstrap model creation and is not used in IPPAN runtime nodes.

from __future__ import annotations

import json
from typing import Any, Dict

import lightgbm as lgb
import numpy as np
import pandas as pd
from sklearn.metrics import mean_squared_error
from sklearn.model_selection import train_test_split


RANDOM_SEED = 42
SCALE = 1_000_000
CSV_PATH = "data/ippan_gbdt_training.csv"
FEATURE_COLS = [
    "uptime_ratio_7d",
    "validated_blocks_7d",
    "missed_blocks_7d",
    "avg_latency_ms",
    "slashing_events_90d",
    "stake_normalized",
    "peer_reports_quality",
]
TARGET_COL = "fairness_score"


# LightGBM training is not bit-for-bit reproducible across all hardware, but the exported
# JSON artifact is what matters. IPPAN runtime will only use the quantized integer model
# for deterministic inference; no training occurs on validators.
def quantize_node(node: Dict[str, Any]) -> None:
    if "leaf_value" in node:
        node["leaf_value"] = int(round(node["leaf_value"] * SCALE))
    else:
        if "left_child" in node:
            quantize_node(node["left_child"])
        if "right_child" in node:
            quantize_node(node["right_child"])


def main() -> None:
    df = pd.read_csv(CSV_PATH)
    X = df[FEATURE_COLS]
    y = df[TARGET_COL]

    X_train, X_val, y_train, y_val = train_test_split(
        X, y, test_size=0.2, random_state=RANDOM_SEED, shuffle=True
    )

    model = lgb.LGBMRegressor(
        n_estimators=300,
        learning_rate=0.05,
        max_depth=3,
        subsample=1.0,
        colsample_bytree=1.0,
        objective="regression",
        random_state=RANDOM_SEED,
        n_jobs=1,
        deterministic=True,
        force_col_wise=True,
    )

    model.fit(X_train, y_train)
    preds = model.predict(X_val)
    mse = mean_squared_error(y_val, preds)
    print(f"Validation MSE: {mse:.6f}")

    raw = model.booster_.dump_model()
    for tree in raw.get("tree_info", []):
        quantize_node(tree["tree_structure"])

    export = {
        "type": "ippan_d_gbdt_v1",
        "scale": SCALE,
        "feature_cols": FEATURE_COLS,
        "target_col": TARGET_COL,
        "lightgbm_format_version": raw.get("version", "unknown"),
        "tree_ensemble": raw.get("tree_info", []),
    }

    output_path = "ai_training/ippan_d_gbdt_v1.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(export, f, sort_keys=True, separators=(",", ":"))

    print(f"Saved canonical model to {output_path}")


if __name__ == "__main__":
    main()
