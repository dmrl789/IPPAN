# OFFLINE ONLY: This training script is for bootstrap model creation and is not used in IPPAN runtime nodes.

from __future__ import annotations

import json
from typing import Any, Dict, List

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
FEATURE_SCALE = {feature: SCALE for feature in FEATURE_COLS}


def quantize_tree_structure(tree: Dict[str, Any]) -> List[Dict[str, Any]]:
    """Convert a LightGBM tree structure into deterministic integer nodes."""

    nodes: List[Dict[str, Any]] = []

    def visit(node: Dict[str, Any]) -> int:
        node_id = len(nodes)

        if "leaf_value" in node:
            leaf_value = int(round(float(node["leaf_value"]) * SCALE))
            nodes.append(
                {
                    "id": node_id,
                    "left": -1,
                    "right": -1,
                    "feature_idx": -1,
                    "threshold": 0,
                    "leaf": leaf_value,
                }
            )
            return node_id

        feature_idx = int(node["split_feature"])
        feature_name = FEATURE_COLS[feature_idx]
        threshold_scale = FEATURE_SCALE[feature_name]
        threshold = int(round(float(node["threshold"]) * threshold_scale))

        nodes.append(
            {
                "id": node_id,
                "left": -1,
                "right": -1,
                "feature_idx": feature_idx,
                "threshold": threshold,
                "leaf": None,
            }
        )

        left_id = visit(node["left_child"])
        right_id = visit(node["right_child"])
        nodes[node_id]["left"] = left_id
        nodes[node_id]["right"] = right_id
        return node_id

    visit(tree)
    return nodes


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

    deterministic_trees = []
    for tree in raw.get("tree_info", []):
        nodes = quantize_tree_structure(tree["tree_structure"])
        weight = int(round(float(tree.get("shrinkage", 1.0)) * SCALE))
        deterministic_trees.append({"nodes": nodes, "weight": weight})

    bias = int(round(float(raw.get("average_output", 0.0)) * SCALE))

    export = {
        "version": 1,
        "scale": SCALE,
        "trees": deterministic_trees,
        "bias": bias,
        "post_scale": SCALE,
        "features": FEATURE_COLS,
        "feature_scale": FEATURE_SCALE,
        "target": TARGET_COL,
        "lightgbm_format_version": raw.get("version", "unknown"),
    }

    output_path = "ai_training/ippan_d_gbdt_v1.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(export, f, sort_keys=True, separators=(",", ":"))

    print(f"Saved canonical model to {output_path}")


if __name__ == "__main__":
    main()
