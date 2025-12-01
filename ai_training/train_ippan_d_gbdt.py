# OFFLINE ONLY: This training script is for bootstrap model creation and is not used in IPPAN runtime nodes.

from __future__ import annotations

import argparse
import hashlib
import json
from typing import Any, Dict, List

import lightgbm as lgb
import numpy as np
import pandas as pd
from sklearn.metrics import mean_squared_error


RANDOM_SEED = 42
SCALE = 1_000_000
DEFAULT_CSV_PATH = "ai_training/data/ippan_training.csv"
DEFAULT_OUTPUT_PATH = "ai_training/ippan_d_gbdt_v1.json"
FEATURE_COLS = [
    "uptime_ratio_7d",
    "validated_blocks_7d",
    "missed_blocks_7d",
    "avg_latency_ms",
    "slashing_events_90d",
    "stake_normalized",
    "peer_reports_quality",
]
TARGET_COL = "fairness_score_scaled"
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
    parser = argparse.ArgumentParser(
        description="Train IPPAN D-GBDT fairness model from CSV dataset"
    )
    parser.add_argument(
        "--csv",
        default=DEFAULT_CSV_PATH,
        help=f"Input CSV file path (default: {DEFAULT_CSV_PATH})",
    )
    parser.add_argument(
        "--out",
        default=DEFAULT_OUTPUT_PATH,
        help=f"Output JSON model file path (default: {DEFAULT_OUTPUT_PATH})",
    )
    args = parser.parse_args()

    csv_path = args.csv
    output_path = args.out

    print(f"Loading dataset from: {csv_path}")
    df = pd.read_csv(csv_path)
    
    # Handle target column: accept either fairness_score_scaled OR fairness_score
    if TARGET_COL in df.columns:
        print(f"Using existing {TARGET_COL} column")
        y_raw = df[TARGET_COL].astype("float64")
    elif "fairness_score" in df.columns:
        print(f"Converting fairness_score to {TARGET_COL} (multiply by {SCALE})")
        y_raw = (df["fairness_score"].astype("float64") * SCALE).round().astype("int64").astype("float64")
        # Store back to dataframe for consistency
        df[TARGET_COL] = y_raw
    else:
        available_cols = ", ".join(df.columns.tolist())
        raise ValueError(
            f"CSV must contain either '{TARGET_COL}' or 'fairness_score' column.\n"
            f"Available columns: {available_cols}"
        )
    
    # Verify all feature columns exist
    missing_features = [col for col in FEATURE_COLS if col not in df.columns]
    if missing_features:
        available_cols = ", ".join(df.columns.tolist())
        raise ValueError(
            f"Missing required feature columns: {missing_features}\n"
            f"Available columns: {available_cols}"
        )
    
    X = df[FEATURE_COLS]
    # Convert scaled integer target back to float for training (divide by SCALE)
    y = y_raw / SCALE

    print(f"Dataset size: {len(df)} rows")
    print(f"Features: {len(FEATURE_COLS)}")
    print(f"Target: {TARGET_COL} (scaled from {y_raw.min():.0f} to {y_raw.max():.0f})")

    # Deterministic hash-based split (stable across sklearn versions)
    def row_key(i: int) -> str:
        cols = df.columns
        vid = df.at[i, "validator_id"] if "validator_id" in cols else "na"
        rid = df.at[i, "round_id"] if "round_id" in cols else "na"

        # Prefer per-row identifier (your CSV has this)
        if "timestamp_utc" in cols:
            return f"{vid}|{rid}|{df.at[i,'timestamp_utc']}"

        # Keep other options for other CSV schemas
        if "hashtimer" in cols:
            return f"{vid}|{rid}|{df.at[i,'hashtimer']}"
        if "timestamp" in cols:
            return f"{vid}|{rid}|{df.at[i,'timestamp']}"
        if "ts" in cols:
            return f"{vid}|{rid}|{df.at[i,'ts']}"

        # Fallback (least desirable)
        return f"{vid}|{rid}"

    def is_val(i: int) -> bool:
        h = hashlib.sha256(row_key(i).encode("utf-8")).digest()
        bucket = (h[0] << 8) | h[1]  # 0..65535
        return bucket < int(0.2 * 65536)  # 20% val

    val_mask = np.array([is_val(i) for i in range(len(df))], dtype=bool)

    X_train, X_val = X[~val_mask], X[val_mask]
    y_train, y_val = y[~val_mask], y[val_mask]

    print(f"Split: train={len(X_train)} val={len(X_val)}")

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
        feature_fraction_seed=RANDOM_SEED,
        bagging_seed=RANDOM_SEED,
        data_random_seed=RANDOM_SEED,
    )

    print("Training model...")
    model.fit(X_train, y_train)
    
    # Evaluate on both train and validation sets for honest assessment
    train_preds = model.predict(X_train)
    val_preds = model.predict(X_val)
    
    train_mse = mean_squared_error(y_train, train_preds)
    val_mse = mean_squared_error(y_val, val_preds)
    
    print(f"Training MSE: {train_mse:.6f}")
    print(f"Validation MSE: {val_mse:.6f}")
    
    # Warn if validation MSE is suspiciously low (possible data leakage or overfitting)
    if val_mse < 1e-10:
        print("WARNING: Validation MSE is extremely low. Possible causes:")
        print("  - Data leakage (validation set too similar to training)")
        print("  - Overfitting to training data")
        print("  - Deterministic/synthetic data with no variance")

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

    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(export, f, sort_keys=True, separators=(",", ":"))

    print(f"Saved canonical model to {output_path}")


if __name__ == "__main__":
    main()
