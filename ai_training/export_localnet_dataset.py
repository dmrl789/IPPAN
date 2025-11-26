#!/usr/bin/env python3
"""
IPPAN Localnet Dataset Exporter

Exports validator metrics from a running localnet to CSV format for GBDT training.
Supports two modes:
- RPC mode: Fetches metrics via HTTP endpoints
- LOGS mode: Parses Docker Compose logs (fallback)

OFFLINE ONLY: This script uses floats for label computation but the exported CSV
is compatible with the deterministic training pipeline.
"""

import argparse
import csv
import json
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional

try:
    import requests
except ImportError:
    print("ERROR: 'requests' library required. Install with: pip install requests")
    sys.exit(1)


# CSV column order (must match training script expectations)
CSV_COLUMNS = [
    "timestamp_utc",
    "round_id",
    "validator_id",
    "uptime_ratio_7d",
    "validated_blocks_7d",
    "missed_blocks_7d",
    "avg_latency_ms",
    "slashing_events_90d",
    "stake_normalized",
    "peer_reports_quality",
    "fairness_score",
]

# Scale factors
SCALE = 1_000_000
UPTIME_SCALE = 10_000  # ValidatorMetrics uses 0-10000 for percentages


def compute_fairness_score(
    uptime_ratio_7d: float,
    validated_blocks_7d: float,
    missed_blocks_7d: float,
    avg_latency_ms: float,
    slashing_events_90d: float,
    stake_normalized: float,
    peer_reports_quality: float,
) -> float:
    """
    Compute fairness score using the same policy as synthetic generator.
    All inputs should be in [0..1] range (normalized).
    """
    total_blocks = validated_blocks_7d + missed_blocks_7d
    miss_ratio = missed_blocks_7d / total_blocks if total_blocks > 0 else 0.0
    latency_norm = min(avg_latency_ms / 1000.0, 1.0)  # Assume 1000ms = 1.0

    score = (
        0.3
        + 0.3 * uptime_ratio_7d
        + 0.2 * stake_normalized
        + 0.2 * peer_reports_quality
        - 0.2 * miss_ratio
        - 0.2 * latency_norm
        - 0.4 * (min(slashing_events_90d, 3.0) / 3.0)
    )

    return max(0.0, min(1.0, score))


def normalize_from_10000(value: int, default: float = 0.5) -> float:
    """Convert 0-10000 scale to 0-1 range."""
    if value <= 0:
        return default
    return min(value / 10_000.0, 1.0)


def fetch_status_rpc(rpc_url: str) -> Optional[Dict[str, Any]]:
    """Fetch status from RPC endpoint."""
    try:
        url = f"{rpc_url.rstrip('/')}/status"
        response = requests.get(url, timeout=5)
        response.raise_for_status()
        return response.json()
    except Exception as e:
        print(f"WARNING: Failed to fetch /status: {e}", file=sys.stderr)
        return None


def extract_validators_from_status(status_data: Dict[str, Any]) -> Dict[str, Dict[str, Any]]:
    """
    Extract validator metrics from /status response.
    
    Expected structure (new format):
    {
        "consensus": {
            "round": <round_id>,
            "validator_ids": ["<id1>", "<id2>", ...],  // backward compatibility
            "validators": {                            // NEW: full metrics map
                "<validator_id>": {
                    "uptime": <0-10000>,
                    "latency": <0-10000>,
                    "honesty": <0-10000>,
                    "blocks_proposed": <u64>,
                    "blocks_verified": <u64>,
                    "stake": { "micro_ipn": "<u128-as-string>" },
                    "rounds_active": <u64>,
                    "slashing_events_90d": <u32>
                }
            }
        }
    }
    
    Old format (array only) will raise an error.
    """
    validators = {}
    
    if "consensus" not in status_data:
        return validators
    
    consensus = status_data["consensus"]
    round_id = consensus.get("round", 0)
    
    # Check for new format: validators is a dict/object
    validators_raw = consensus.get("validators")
    
    if validators_raw is None:
        # Check if old format (array) exists
        validator_ids = consensus.get("validator_ids", consensus.get("validators", []))
        if isinstance(validator_ids, list) and len(validator_ids) > 0:
            print(
                "ERROR: Status endpoint does not expose metrics yet.",
                "Update node to include consensus.validators map.",
                f"Found {len(validator_ids)} validator IDs but no metrics.",
                sep="\n",
                file=sys.stderr,
            )
            sys.exit(1)
        return validators
    
    if isinstance(validators_raw, list):
        # Old format: array of IDs only
        print(
            "ERROR: Status endpoint does not expose metrics yet.",
            "Update node to include consensus.validators map.",
            f"Found {len(validators_raw)} validator IDs but no metrics.",
            sep="\n",
            file=sys.stderr,
        )
        sys.exit(1)
    
    if isinstance(validators_raw, dict):
        for validator_id, metrics in validators_raw.items():
            if isinstance(metrics, dict):
                validators[validator_id] = {
                    "round_id": round_id,
                    "uptime": metrics.get("uptime", 10000),
                    "latency": metrics.get("latency", 0),
                    "honesty": metrics.get("honesty", 10000),
                    "blocks_proposed": metrics.get("blocks_proposed", 0),
                    "blocks_verified": metrics.get("blocks_verified", 0),
                    "stake": metrics.get("stake", {}),
                    "rounds_active": metrics.get("rounds_active", 0),
                    "slashing_events_90d": metrics.get("slashing_events_90d", 0),
                }
    
    return validators


def extract_stake_micro(stake_data: Any) -> int:
    """Extract stake in micro-IPN from various possible formats."""
    if isinstance(stake_data, dict):
        micro_ipn = stake_data.get("micro_ipn", stake_data.get("atomic", 0))
        # Handle string or int (JSON may serialize u128 as string)
        return int(micro_ipn) if micro_ipn is not None else 0
    elif isinstance(stake_data, (int, str)):
        return int(stake_data)
    return 0


def convert_validator_to_row(
    validator_id: str,
    metrics: Dict[str, Any],
    max_stake_micro: int,
    timestamp: str,
) -> Dict[str, Any]:
    """Convert validator metrics to CSV row format."""
    # Extract raw values
    uptime_scaled = metrics.get("uptime", 10000)
    latency_scaled = metrics.get("latency", 0)
    honesty_scaled = metrics.get("honesty", 10000)
    blocks_proposed = metrics.get("blocks_proposed", 0)
    blocks_verified = metrics.get("blocks_verified", 0)
    rounds_active = metrics.get("rounds_active", 0)
    stake_micro = extract_stake_micro(metrics.get("stake", 0))
    round_id = metrics.get("round_id", 0)
    
    # Normalize to [0..1] for feature computation
    uptime_ratio = normalize_from_10000(uptime_scaled)
    peer_quality = normalize_from_10000(honesty_scaled)
    
    # Compute normalized features (for fairness score)
    validated_blocks_norm = blocks_verified / max(rounds_active, 1) if rounds_active > 0 else 0.0
    missed_blocks_norm = max(0, rounds_active - blocks_proposed) / max(rounds_active, 1) if rounds_active > 0 else 0.0
    latency_ms_norm = (latency_scaled * 100) / 10_000.0  # Approximate: 10000 = 1000ms
    stake_norm = stake_micro / max(max_stake_micro, 1) if max_stake_micro > 0 else 1.0
    
    # Get slashing events (now available in metrics)
    slashing_events = metrics.get("slashing_events_90d", 0)
    slashing_norm = min(slashing_events, 3) / 3.0 if slashing_events > 0 else 0.0
    
    # Compute fairness score
    fairness_score = compute_fairness_score(
        uptime_ratio,
        validated_blocks_norm,
        missed_blocks_norm,
        latency_ms_norm / 1000.0,  # Convert to seconds for score
        slashing_norm,
        stake_norm,
        peer_quality,
    )
    
    # Convert to scaled integers for CSV (matching training format)
    return {
        "timestamp_utc": timestamp,
        "round_id": round_id,
        "validator_id": validator_id,
        "uptime_ratio_7d": int(uptime_ratio * SCALE),
        "validated_blocks_7d": int(blocks_verified * SCALE),
        "missed_blocks_7d": int(max(0, rounds_active - blocks_proposed) * SCALE),
        "avg_latency_ms": int(latency_ms_norm * SCALE),
        "slashing_events_90d": slashing_events,
        "stake_normalized": int(min(stake_norm, 1.0) * SCALE),
        "peer_reports_quality": int(peer_quality * SCALE),
        "fairness_score": fairness_score,
    }


def export_rpc_mode(
    rpc_url: str,
    samples: int,
    interval: float,
    output_path: Path,
) -> None:
    """Export dataset using RPC mode."""
    print(f"RPC Mode: Fetching {samples} samples from {rpc_url} (interval: {interval}s)")
    
    rows = []
    
    for i in range(samples):
        print(f"  Sample {i+1}/{samples}...", end=" ", flush=True)
        
        status_data = fetch_status_rpc(rpc_url)
        if not status_data:
            print("FAILED (no data)")
            time.sleep(interval)
            continue
        
        validators = extract_validators_from_status(status_data)
        if not validators:
            print("FAILED (no validators)")
            time.sleep(interval)
            continue
        
        # Compute max stake for normalization
        max_stake_micro = 0
        for metrics in validators.values():
            stake_micro = extract_stake_micro(metrics.get("stake", 0))
            max_stake_micro = max(max_stake_micro, stake_micro)
        
        # Generate timestamp
        timestamp = datetime.now(timezone.utc).isoformat()
        
        # Convert each validator to a row
        for validator_id, metrics in validators.items():
            row = convert_validator_to_row(validator_id, metrics, max_stake_micro, timestamp)
            rows.append(row)
        
        print(f"OK ({len(validators)} validators)")
        
        if i < samples - 1:
            time.sleep(interval)
    
    # Sort by timestamp then validator_id
    rows.sort(key=lambda r: (r["timestamp_utc"], r["validator_id"]))
    
    # Write CSV
    print(f"\nWriting {len(rows)} rows to {output_path}...")
    with open(output_path, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=CSV_COLUMNS)
        writer.writeheader()
        writer.writerows(rows)
    
    print(f"✓ Exported {len(rows)} rows to {output_path}")


def export_logs_mode(
    compose_file: str,
    project: str,
    output_path: Path,
) -> None:
    """Export dataset using LOGS mode (fallback)."""
    print(f"LOGS Mode: Parsing logs from {compose_file} (project: {project})")
    print("WARNING: LOGS mode is not yet implemented.")
    print("Please use RPC mode or ensure metrics are logged in JSON format.")
    print("\nTo use RPC mode:")
    print(f"  python {sys.argv[0]} --mode rpc --rpc http://localhost:8080 --samples 120 --interval 5 --out {output_path}")
    sys.exit(1)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Export validator metrics from localnet to CSV for GBDT training"
    )
    parser.add_argument(
        "--mode",
        choices=["rpc", "logs"],
        default="rpc",
        help="Export mode: rpc (HTTP endpoints) or logs (Docker logs)",
    )
    parser.add_argument(
        "--rpc",
        default="http://localhost:8080",
        help="RPC base URL (default: http://localhost:8080)",
    )
    parser.add_argument(
        "--samples",
        type=int,
        default=120,
        help="Number of samples to collect (default: 120)",
    )
    parser.add_argument(
        "--interval",
        type=float,
        default=5.0,
        help="Interval between samples in seconds (default: 5.0)",
    )
    parser.add_argument(
        "--compose-file",
        default="localnet/docker-compose.full-stack.yaml",
        help="Docker Compose file path (LOGS mode only)",
    )
    parser.add_argument(
        "--project",
        default="ippan-local",
        help="Docker Compose project name (LOGS mode only)",
    )
    parser.add_argument(
        "--out",
        default="ai_training/localnet_training.csv",
        help="Output CSV file path (default: ai_training/localnet_training.csv)",
    )
    
    args = parser.parse_args()
    
    output_path = Path(args.out)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    
    if args.mode == "rpc":
        export_rpc_mode(args.rpc, args.samples, args.interval, output_path)
    else:
        export_logs_mode(args.compose_file, args.project, output_path)


if __name__ == "__main__":
    main()

