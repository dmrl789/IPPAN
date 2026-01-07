#!/usr/bin/env python3
import csv
import glob
import os
import statistics


def percentile(vals, p):
    if not vals:
        return None
    s = sorted(vals)
    k = (len(s) - 1) * (p / 100.0)
    f = int(k)
    c = min(f + 1, len(s) - 1)
    if f == c:
        return s[f]
    return s[f] + (s[c] - s[f]) * (k - f)


def safe_int(x):
    try:
        return int(x)
    except Exception:
        return None


def main():
    files = sorted(glob.glob("ops/ippantime/out/ippantime_*.csv"))
    if not files:
        raise SystemExit("No ops/ippantime/out/ippantime_*.csv found")

    # probe -> node -> jitter list
    data = {}
    for fp in files:
        with open(fp, "r", encoding="utf-8") as f:
            r = csv.DictReader(f)
            for row in r:
                probe = row.get("probe")
                node = row.get("node")
                ok = row.get("ok") == "1"
                if not (probe and node and ok):
                    continue
                j = safe_int(row.get("jitter_us", ""))
                if j is None:
                    continue
                j = abs(j)
                data.setdefault(probe, {}).setdefault(node, []).append(j)

    os.makedirs("ops/ippantime/out", exist_ok=True)
    out = "ops/ippantime/out/JITTER_HOTSPOTS.md"
    lines = ["# Jitter hotspots\n\n", f"Inputs: {len(files)} CSV files\n\n"]

    for probe in sorted(data.keys()):
        rows = []
        for node, js in data[probe].items():
            p95 = percentile(js, 95)
            p99 = percentile(js, 99)
            mean = statistics.mean(js) if js else None
            if p95 is None or p99 is None or mean is None:
                continue
            rows.append((int(p99), int(p95), int(mean), node, len(js)))
        rows.sort(reverse=True)

        lines.append(f"## Probe `{probe}`\n\n")
        lines.append("| node | samples | jitter_us mean | jitter_us p95 | jitter_us p99 |\n")
        lines.append("|---|---:|---:|---:|---:|\n")
        for p99, p95, mean, node, n in rows[:20]:
            lines.append(f"| {node} | {n} | {mean} | {p95} | {p99} |\n")
        lines.append("\n")

    with open(out, "w", encoding="utf-8") as f:
        f.write("".join(lines))
    print(f"Wrote {out}")


if __name__ == "__main__":
    main()

