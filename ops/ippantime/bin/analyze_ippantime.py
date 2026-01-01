#!/usr/bin/env python3
import csv, glob, os, math, statistics

def percentile(vals, p):
    if not vals:
        return None
    s = sorted(vals)
    k = (len(s)-1) * (p/100.0)
    f = int(k)
    c = min(f+1, len(s)-1)
    if f == c:
        return s[f]
    return s[f] + (s[c]-s[f]) * (k-f)

def safe_int(x):
    try:
        return int(x)
    except Exception:
        return None

def safe_float(x):
    try:
        return float(x)
    except Exception:
        return None

def main():
    in_glob = "ops/ippantime/out/ippantime_*.csv"
    out_md = "ops/ippantime/out/REPORT.md"

    files = sorted(glob.glob(in_glob))
    if not files:
        raise SystemExit(f"No inputs matching {in_glob}")

    # Data: probe -> node -> lists
    data = {}
    for fp in files:
        with open(fp, "r", encoding="utf-8") as f:
            r = csv.DictReader(f)
            for row in r:
                probe = row["probe"]
                node = row["node"]
                ok = row["ok"] == "1"
                if probe not in data:
                    data[probe] = {}
                if node not in data[probe]:
                    data[probe][node] = {"offset": [], "jitter": [], "rtt": [], "back": 0, "total": 0, "ok": 0}
                d = data[probe][node]
                d["total"] += 1
                if ok:
                    d["ok"] += 1
                    off = safe_int(row["offset_us"])
                    jit = safe_int(row["jitter_us"])
                    rtt = safe_float(row["rtt_ms"])
                    if off is not None: d["offset"].append(off)
                    if jit is not None: d["jitter"].append(abs(jit))
                    if rtt is not None: d["rtt"].append(rtt)
                if row.get("monotonic_backwards") == "1":
                    d["back"] += 1

    # Build report
    lines = []
    lines.append("# IPPAN Time Mesh Report\n")
    lines.append(f"Inputs: {len(files)} CSV files\n")
    lines.append("\n## Probes included\n")
    for p in sorted(data.keys()):
        lines.append(f"- `{p}`\n")

    # Per-probe ranking tables
    for probe in sorted(data.keys()):
        lines.append(f"\n## Probe: `{probe}`\n")
        rows = []
        for node, d in data[probe].items():
            if not d["offset"]:
                rows.append((node, d["ok"], d["total"], None, None, None, None, None, None, d["back"]))
                continue
            p50 = int(percentile(d["offset"], 50))
            p95 = int(percentile(d["offset"], 95))
            p99 = int(percentile(d["offset"], 99))
            j95 = int(percentile(d["jitter"], 95)) if d["jitter"] else 0
            j99 = int(percentile(d["jitter"], 99)) if d["jitter"] else 0
            r95 = percentile(d["rtt"], 95) if d["rtt"] else None
            r99 = percentile(d["rtt"], 99) if d["rtt"] else None
            # RTT-normalized offset helps distinguish geography/network from true clock skew:
            # offset_norm_p99_us = offset_p99_us - (rtt_p99_ms * 1000 / 2)
            off_norm_p99 = None
            if r99 is not None:
                off_norm_p99 = int(p99 - (r99 * 1000.0 / 2.0))
            rows.append((node, d["ok"], d["total"], p50, p95, p99, off_norm_p99, j95, j99, r99, d["back"]))

        # sort by worst p99 abs offset then jitter then rtt
        def key(x):
            p99 = x[5]
            j99 = x[8]
            r99 = x[9]
            return (abs(p99) if p99 is not None else 10**18, j99 if j99 is not None else 10**18, r99 if r99 is not None else 10**9)
        rows.sort(key=key, reverse=True)

        lines.append("| node | ok/total | off_us p50 | off_us p95 | off_us p99 | off_norm_us p99 | jitter_us p95 | jitter_us p99 | rtt_ms p99 | backwards |\n")
        lines.append("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n")
        for (node, okc, tot, p50, p95, p99, off_norm_p99, j95, j99, r99, back) in rows:
            lines.append(
                f"| {node} | {okc}/{tot} | {p50 if p50 is not None else '—'} | {p95 if p95 is not None else '—'} | {p99 if p99 is not None else '—'} | {off_norm_p99 if off_norm_p99 is not None else '—'} | {j95 if j95 is not None else '—'} | {j99 if j99 is not None else '—'} | {f'{r99:.3f}' if r99 is not None else '—'} | {back} |\n"
            )

    # Cross-probe comparison (TR-A vs TR-B if present)
    if "tr-a" in data and "tr-b" in data:
        lines.append("\n## Cross-probe consistency (tr-a vs tr-b)\n")
        nodes = sorted(set(data["tr-a"].keys()).intersection(set(data["tr-b"].keys())))
        lines.append("| node | tr-a off_p99 | tr-b off_p99 | delta_p99_abs |\n")
        lines.append("|---|---:|---:|---:|\n")
        for n in nodes:
            a = data["tr-a"][n]["offset"]
            b = data["tr-b"][n]["offset"]
            if not a or not b:
                continue
            ap99 = int(percentile(a, 99))
            bp99 = int(percentile(b, 99))
            lines.append(f"| {n} | {ap99} | {bp99} | {abs(ap99 - bp99)} |\n")

    os.makedirs(os.path.dirname(out_md), exist_ok=True)
    with open(out_md, "w", encoding="utf-8") as f:
        f.write("".join(lines))

    print(f"Wrote {out_md}")

if __name__ == "__main__":
    main()
