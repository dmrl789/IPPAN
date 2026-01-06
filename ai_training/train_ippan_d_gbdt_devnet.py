# OFFLINE ONLY: Devnet entrypoint for D-GBDT fairness model training.
#
# This is a minimal wrapper around `ai_training/train_ippan_d_gbdt.py` with:
# - devnet default dataset discovery (devnet_dataset_*.csv[.gz])
# - devnet default output layout (ai_assets/models/devnet/)
# - required key=value summary lines for automation:
#     model_id=...
#     model_path=...
#     model_hash=...
#
# Runtime nodes NEVER train; they only load a pinned hash-verified JSON model.

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional

import lightgbm as lgb
import numpy as np
import pandas as pd
from sklearn.metrics import mean_squared_error

try:
    from blake3 import blake3  # type: ignore

    _HAS_PY_BLAKE3 = True
except Exception:
    # We intentionally allow a fallback path for environments where `pip install blake3`
    # is not available (e.g., missing wheels / Rust build issues). In that case we
    # compute the canonical hash using the existing Rust helper binary.
    _HAS_PY_BLAKE3 = False


RANDOM_SEED = 42
SCALE = 1_000_000

DEFAULT_DATA_DIR = "ai_assets/datasets/devnet"
DEFAULT_OUTPUT_DIR = "ai_assets/models/devnet"
DEFAULT_MODEL_ID = "ippan_d_gbdt_devnet_v2"

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


def _pick_latest_dataset_file(data_dir: Path) -> Path:
    """
    Pick the newest devnet dataset file from a directory.

    Preferred order:
    - devnet_dataset_*.csv.gz
    - devnet_dataset_*.csv
    """
    if not data_dir.exists():
        raise FileNotFoundError(f"Dataset dir not found: {data_dir}")

    candidates: List[Path] = []
    candidates.extend(sorted(data_dir.glob("devnet_dataset_*.csv.gz")))
    candidates.extend(sorted(data_dir.glob("devnet_dataset_*.csv")))

    if not candidates:
        raise FileNotFoundError(
            f"No devnet datasets found in {data_dir}. Expected devnet_dataset_*.csv.gz or devnet_dataset_*.csv"
        )

    # Name sorting is sufficient because filenames include UTC timestamps
    return candidates[-1]


def _compute_model_hash_b3(model_path: Path) -> str:
    """Compute BLAKE3 hash of the model's canonical JSON representation.
    
    IMPORTANT: The hash is computed over the canonical JSON that includes ONLY
    the fields recognized by the Rust Model struct: version, scale, trees, bias,
    post_scale. This ensures hash compatibility between Python and Rust.
    """
    # Load the full model JSON
    with open(model_path, "r", encoding="utf-8") as f:
        full_model = json.load(f)
    
    # Extract only the fields that Rust's Model struct includes in its hash
    # (see crates/ai_core/src/gbdt/model.rs)
    canonical_model = {
        "bias": full_model["bias"],
        "post_scale": full_model["post_scale"],
        "scale": full_model["scale"],
        "trees": full_model["trees"],
        "version": full_model["version"],
    }
    
    # Serialize to canonical JSON: sorted keys, no whitespace
    # This matches Rust's serde_canon::to_canonical_json
    canonical_json = json.dumps(canonical_model, sort_keys=True, separators=(",", ":"))
    
    if _HAS_PY_BLAKE3:
        return blake3(canonical_json.encode("utf-8")).hexdigest()

    # Fallback: use Rust canonical hasher (matches verifier expectations).
    # Requires: `cargo` available and workspace builds.
    out = subprocess.check_output(
        [
            "cargo",
            "run",
            "-q",
            "-p",
            "ippan-ai-core",
            "--bin",
            "compute_model_hash",
            "--",
            str(model_path),
        ],
        text=True,
    ).strip()
    if not out or len(out) != 64:
        raise RuntimeError(f"Unexpected compute_model_hash output: {out!r}")
    return out


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
        description="Train IPPAN D-GBDT fairness model (devnet entrypoint)"
    )
    parser.add_argument(
        "--data-dir",
        default=DEFAULT_DATA_DIR,
        help=f"Directory containing devnet_dataset_*.csv(.gz) (default: {DEFAULT_DATA_DIR})",
    )
    parser.add_argument(
        "--output-dir",
        default=DEFAULT_OUTPUT_DIR,
        help=f"Output directory for model JSON (default: {DEFAULT_OUTPUT_DIR})",
    )
    parser.add_argument(
        "--model-id",
        default=DEFAULT_MODEL_ID,
        help=f"Model id used for output filename (default: {DEFAULT_MODEL_ID})",
    )
    parser.add_argument(
        "--csv",
        default=None,
        help="Optional explicit input CSV/CSV.GZ path; overrides --data-dir discovery",
    )
    parser.add_argument(
        "--out",
        default=None,
        help="Optional explicit output model path; overrides --output-dir/--model-id",
    )
    args = parser.parse_args()

    data_dir = Path(args.data_dir)
    out_dir = Path(args.output_dir)
    model_id = str(args.model_id)

    if args.csv:
        csv_path = Path(args.csv)
    else:
        csv_path = _pick_latest_dataset_file(data_dir)

    if args.out:
        output_path = Path(args.out)
    else:
        out_dir.mkdir(parents=True, exist_ok=True)
        output_path = out_dir / f"{model_id}.json"

    print(f"Loading dataset from: {csv_path}")
    df = pd.read_csv(csv_path)

    # Handle target column: accept either fairness_score_scaled OR fairness_score
    if TARGET_COL in df.columns:
        print(f"Using existing {TARGET_COL} column")
        y_raw = df[TARGET_COL].astype("float64")
    elif "fairness_score" in df.columns:
        print(f"Converting fairness_score to {TARGET_COL} (multiply by {SCALE})")
        y_raw = (
            (df["fairness_score"].astype("float64") * SCALE)
            .round()
            .astype("int64")
            .astype("float64")
        )
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

    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w", encoding="utf-8", newline="") as f:
        json.dump(export, f, sort_keys=True, separators=(",", ":"))

    model_hash = _compute_model_hash_b3(output_path)

    # Required automation-friendly summary lines
    print(f"model_id={model_id}")
    print(f"model_path={output_path.as_posix()}")
    print(f"model_hash={model_hash}")


if __name__ == "__main__":
    main()


