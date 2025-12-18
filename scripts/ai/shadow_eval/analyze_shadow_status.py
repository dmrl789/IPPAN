#!/usr/bin/env python3
import json
import os
import sys
from collections import Counter, defaultdict

LOG_DIR = "logs/ai_shadow"
EXPECTED_HASH = os.environ.get("EXPECTED_SHADOW_HASH", "").strip()


def main() -> None:
    if not os.path.isdir(LOG_DIR):
        print(f"Log dir {LOG_DIR} not found", file=sys.stderr)
        sys.exit(1)

    ai_files = [
        f for f in os.listdir(LOG_DIR) if f.startswith("ai_status_") and f.endswith(".json")
    ]
    if not ai_files:
        print("No ai_status_*.json files found; run the collector first.")
        return

    per_node_samples: dict[str, int] = defaultdict(int)
    per_node_loaded_true: dict[str, int] = defaultdict(int)
    per_node_hashes: dict[str, Counter[str]] = defaultdict(Counter)

    for fname in sorted(ai_files):
        path = os.path.join(LOG_DIR, fname)
        try:
            with open(path, "r", encoding="utf-8") as f:
                data = json.load(f)
        except Exception as e:
            print(f"WARN: Failed to parse {fname}: {e}", file=sys.stderr)
            continue

        # filename looks like: ai_status_root_5.223..._YYYY...
        parts = fname.split("_")
        node_id = parts[2] if len(parts) >= 3 else "unknown"

        per_node_samples[node_id] += 1

        shadow_loaded = data.get("shadow_loaded")
        shadow_hash = data.get("shadow_model_hash")

        if shadow_loaded is True:
            per_node_loaded_true[node_id] += 1

        if shadow_hash:
            per_node_hashes[node_id][shadow_hash] += 1

    print("=== Shadow model health summary ===")
    print(f"Expected shadow model hash: {EXPECTED_HASH or '(not set)'}\n")

    for node, total in sorted(per_node_samples.items()):
        loaded = per_node_loaded_true[node]
        rate = (loaded / total * 100.0) if total else 0.0
        print(f"Node: {node}")
        print(f"  Samples: {total}")
        print(f"  shadow_loaded=true: {loaded} ({rate:.1f}% of samples)")

        hashes = per_node_hashes[node]
        if not hashes:
            print("  Shadow hash: (none observed)")
        else:
            print("  Shadow hashes observed:")
            for h, count in hashes.most_common():
                flag = ""
                if EXPECTED_HASH and h != EXPECTED_HASH:
                    flag = "  <== MISMATCH vs expected!"
                print(f"    {h}  (count={count}){flag}")
        print()

    if EXPECTED_HASH:
        bad_nodes = [
            node
            for node, hashes in per_node_hashes.items()
            if any(h != EXPECTED_HASH for h in hashes.keys())
        ]
        if bad_nodes:
            print("WARN: Hash mismatches found on nodes:", ", ".join(bad_nodes))
        else:
            print("OK: All observed shadow hashes match EXPECTED_SHADOW_HASH.")
    else:
        print("NOTE: EXPECTED_SHADOW_HASH not set; only reporting raw hashes.")


if __name__ == "__main__":
    main()


