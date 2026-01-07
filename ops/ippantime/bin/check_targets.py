#!/usr/bin/env python3
"""
Check which /status endpoints are live and expose ippan_time_us (or configured field).

Usage:
  python3 ops/ippantime/bin/check_targets.py ops/ippantime/inventory.yaml

Notes:
- Read-only: does not modify chain/node state.
- By default, checks the *expected 40-port layout* derived from servers.node_index_range
  and http.base_port (+ node_index).
- If you want to check only explicit_targets (the 4-node fallback), use --mode explicit.
"""

import argparse
import json
import sys
from urllib.request import urlopen, Request


def http_get_json(url: str, timeout: float) -> dict:
    req = Request(url, headers={"User-Agent": "ippantime-check/1.0"})
    with urlopen(req, timeout=timeout) as r:
        return json.loads(r.read())


def load_inventory(path: str) -> dict:
    import yaml  # PyYAML required

    with open(path, "r", encoding="utf-8") as f:
        return yaml.safe_load(f)


def expected_targets(inv: dict) -> list[dict]:
    base_port = int(inv["http"]["base_port"])
    targets = []
    for s in inv.get("servers", []):
        host = s["host"]
        a, b = s["node_index_range"]
        for i in range(int(a), int(b) + 1):
            targets.append({"node": f"node-{i:02d}", "url": f"http://{host}:{base_port + i}"})
    return targets


def explicit_targets(inv: dict) -> list[dict]:
    return [{"node": t["node"], "url": t["url"]} for t in inv.get("explicit_targets", [])]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("inventory", help="Path to inventory.yaml")
    ap.add_argument(
        "--mode",
        choices=["expected", "explicit", "auto"],
        default="expected",
        help="Which target set to check: expected=range-based, explicit=explicit_targets only, auto=explicit if present else expected",
    )
    ap.add_argument("--timeout", type=float, default=None, help="HTTP timeout seconds (defaults to inventory http.timeout_sec)")
    args = ap.parse_args()

    inv = load_inventory(args.inventory)
    status_path = inv["http"]["status_path"]
    field = inv["http"]["ippan_time_field"]
    timeout = float(args.timeout if args.timeout is not None else inv["http"]["timeout_sec"])

    if args.mode == "expected":
        targets = expected_targets(inv)
    elif args.mode == "explicit":
        targets = explicit_targets(inv)
    else:
        targets = explicit_targets(inv) if inv.get("explicit_targets") else expected_targets(inv)

    if not targets:
        print("No targets to check (empty target list).", file=sys.stderr)
        return 2

    ok = []
    miss = []
    for t in targets:
        url = t["url"].rstrip("/") + status_path
        try:
            payload = http_get_json(url, timeout=timeout)
            if payload.get(field, None) is None:
                miss.append((t["node"], url, "missing_field"))
            else:
                ok.append((t["node"], url))
        except Exception:
            miss.append((t["node"], url, "unreachable"))

    print(f"Checked {len(targets)} targets (mode={args.mode})")
    print(f"OK:   {len(ok)}")
    print(f"MISS: {len(miss)}")
    if ok:
        print("\n== OK ==")
        for node, url in ok:
            print(f"{node} {url}")
    if miss:
        print("\n== MISSING ==")
        for node, url, why in miss:
            print(f"{node} {url} [{why}]")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

