#!/usr/bin/env python3
"""
Analyze shadow status snapshots within a specific time window.

This is intended for devnet shadow evaluation where logs/ai_shadow/ may include
older snapshots from previous hash rollouts. We filter by timestamp parsed from
the filename, e.g.:
  logs/ai_shadow/ai_status_root_188.245.97.41_2025-12-19T12-58-16Z.json
"""

from __future__ import annotations

import argparse
import glob
import json
import os
from collections import Counter, defaultdict
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from typing import Any, Dict, Iterable, List, Optional, Tuple


@dataclass(frozen=True)
class Snapshot:
    ts: datetime  # UTC
    node: str
    path: str


def parse_ts_from_filename(path: str) -> datetime:
    base = os.path.basename(path)
    if not base.endswith(".json"):
        raise ValueError("not json")
    stem = base[: -len(".json")]
    ts_raw = stem.rsplit("_", 1)[-1]
    ts = datetime.strptime(ts_raw, "%Y-%m-%dT%H-%M-%SZ")
    return ts.replace(tzinfo=timezone.utc)


def parse_node_from_filename(path: str) -> str:
    base = os.path.basename(path)
    if not base.startswith("ai_status_"):
        raise ValueError("not ai_status")
    stem = base[: -len(".json")] if base.endswith(".json") else base
    remainder = stem[len("ai_status_") :]
    node = remainder.rsplit("_", 1)[0]
    return node


def parse_iso_z(s: str) -> datetime:
    # Accept: 2025-12-19T12:58:16Z
    if not s.endswith("Z"):
        raise ValueError("expected Z suffix")
    dt = datetime.fromisoformat(s[:-1])
    return dt.replace(tzinfo=timezone.utc)


def iso_z(dt: datetime) -> str:
    return dt.astimezone(timezone.utc).replace(tzinfo=None).isoformat() + "Z"


def iter_snapshots(log_dir: str) -> Iterable[Snapshot]:
    pattern = os.path.join(log_dir, "ai_status_root_*_*.json")
    for path in glob.glob(pattern):
        try:
            ts = parse_ts_from_filename(path)
            node = parse_node_from_filename(path)
        except Exception:
            continue
        yield Snapshot(ts=ts, node=node, path=path)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--log-dir", default="logs/ai_shadow")
    ap.add_argument("--expected-hash", required=True)
    ap.add_argument("--since", required=True, help="Inclusive start, e.g. 2025-12-19T12:58:16Z")
    ap.add_argument("--hours", type=float, default=24.0)
    ap.add_argument(
        "--nodes",
        nargs="+",
        default=[
            "root_188.245.97.41",
            "root_135.181.145.174",
            "root_178.156.219.107",
            "root_5.223.51.238",
        ],
        help="Node keys (derived from filenames).",
    )
    args = ap.parse_args()

    since = parse_iso_z(args.since)
    until = since + timedelta(hours=float(args.hours))
    expected_hash = args.expected_hash
    expected_nodes: List[str] = list(args.nodes)

    snaps = sorted(iter_snapshots(args.log_dir), key=lambda s: (s.ts, s.node))
    if not snaps:
        print(f"NO_SNAPSHOTS under {args.log_dir}")
        return 0

    # Collect per-node stats
    files_total = 0
    parse_errors = 0
    per_node_samples = Counter()
    per_node_loaded_true = Counter()
    per_node_hash_counts: Dict[str, Counter] = defaultdict(Counter)

    # Window bookkeeping
    ts_min: Optional[datetime] = None
    ts_max: Optional[datetime] = None

    for s in snaps:
        if s.ts < since or s.ts >= until:
            continue
        if s.node not in expected_nodes:
            continue

        files_total += 1
        ts_min = s.ts if ts_min is None else min(ts_min, s.ts)
        ts_max = s.ts if ts_max is None else max(ts_max, s.ts)

        try:
            with open(s.path, "r", encoding="utf-8") as fh:
                data: Dict[str, Any] = json.load(fh)
        except Exception:
            parse_errors += 1
            continue

        per_node_samples[s.node] += 1
        loaded = data.get("shadow_loaded")
        if loaded is True:
            per_node_loaded_true[s.node] += 1

        h = data.get("shadow_model_hash") or data.get("shadow_hash")
        if h is None:
            h = "MISSING"
        per_node_hash_counts[s.node][str(h)] += 1

    print("### Shadow status window analysis ###")
    print(f"LOG_DIR: {args.log_dir}")
    print(f"EXPECTED_HASH: {expected_hash}")
    print(f"WINDOW_SINCE: {iso_z(since)}")
    print(f"WINDOW_UNTIL: {iso_z(until)} (hours={args.hours})")
    print(f"FILES_TOTAL_IN_WINDOW (name-filtered): {files_total}")
    print(f"PARSE_ERRORS_IN_WINDOW: {parse_errors}")
    print(f"OBSERVED_WINDOW_TS: {iso_z(ts_min) if ts_min else None} -> {iso_z(ts_max) if ts_max else None}")
    print("")

    mismatching_nodes: List[str] = []
    not_loaded_nodes: List[str] = []
    missing_nodes: List[str] = []

    for node in expected_nodes:
        samples = per_node_samples[node]
        if samples == 0:
            missing_nodes.append(node)
            continue

        loaded_true = per_node_loaded_true[node]
        loaded_pct = (loaded_true / samples) * 100.0
        hashes = per_node_hash_counts[node]

        print(f"Node: {node}")
        print(f"  Samples parsed: {samples}")
        print(f"  shadow_loaded=true: {loaded_true} ({loaded_pct:.1f}%)")
        print("  Shadow hashes observed:")
        for h, c in hashes.most_common():
            flag = ""
            if h != expected_hash:
                flag = "  <== MISMATCH vs expected!"
            print(f"    {h}  (count={c}){flag}")
        print("")

        if loaded_true != samples:
            not_loaded_nodes.append(node)
        if any(h != expected_hash for h in hashes.keys()):
            mismatching_nodes.append(node)

    ok = not missing_nodes and not mismatching_nodes and not not_loaded_nodes and files_total > 0
    if ok:
        print("OK: All samples in window match EXPECTED_HASH and shadow_loaded=true on all nodes.")
        return 0

    if missing_nodes:
        print(f"WARN: No parsed samples in window for nodes: {', '.join(missing_nodes)}")
    if not_loaded_nodes:
        print(f"WARN: shadow_loaded was not always true for nodes: {', '.join(not_loaded_nodes)}")
    if mismatching_nodes:
        print(f"WARN: Hash mismatches found in window on nodes: {', '.join(sorted(set(mismatching_nodes)))}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())


