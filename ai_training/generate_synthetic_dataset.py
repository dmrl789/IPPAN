"""Synthetic bootstrap dataset generator for IPPAN validator fairness modeling."""

from __future__ import annotations

import os

import numpy as np
import pandas as pd


SEED = 42
N_ROWS = 100_000
CSV_PATH = os.path.join("data", "ippan_gbdt_training.csv")


def main() -> None:
    rng = np.random.default_rng(SEED)

    uptime_ratio_7d = rng.beta(8.0, 2.0, size=N_ROWS)
    validated_blocks_7d = rng.integers(50, 1001, size=N_ROWS)
    missed_blocks_7d = rng.integers(0, 201, size=N_ROWS)
    avg_latency_ms = rng.uniform(20.0, 800.0, size=N_ROWS)
    slashing_events_90d = rng.integers(0, 6, size=N_ROWS)
    stake_normalized = rng.beta(2.0, 5.0, size=N_ROWS)
    peer_reports_quality = rng.beta(3.0, 2.0, size=N_ROWS)

    total_blocks = validated_blocks_7d + missed_blocks_7d
    miss_ratio = np.divide(
        missed_blocks_7d,
        total_blocks,
        out=np.zeros_like(missed_blocks_7d, dtype=float),
        where=total_blocks != 0,
    )
    latency_norm = np.clip(avg_latency_ms / 1000.0, 0.0, 1.0)

    score = (
        0.3
        + 0.3 * uptime_ratio_7d
        + 0.2 * stake_normalized
        + 0.2 * peer_reports_quality
        - 0.2 * miss_ratio
        - 0.2 * latency_norm
        - 0.4 * (np.minimum(slashing_events_90d, 3) / 3.0)
    )
    fairness_score = np.clip(score, 0.0, 1.0)

    df = pd.DataFrame(
        {
            "validator_id": np.arange(N_ROWS, dtype=np.int64),
            "round_id": np.zeros(N_ROWS, dtype=np.int64),
            "uptime_ratio_7d": uptime_ratio_7d,
            "validated_blocks_7d": validated_blocks_7d,
            "missed_blocks_7d": missed_blocks_7d,
            "avg_latency_ms": avg_latency_ms,
            "slashing_events_90d": slashing_events_90d,
            "stake_normalized": stake_normalized,
            "peer_reports_quality": peer_reports_quality,
            "fairness_score": fairness_score,
        }
    )

    os.makedirs(os.path.dirname(CSV_PATH), exist_ok=True)
    df.to_csv(CSV_PATH, index=False)
    print(f"Wrote synthetic dataset to {CSV_PATH}, rows={len(df)}")


if __name__ == "__main__":
    main()
