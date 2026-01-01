#!/usr/bin/env python3
import argparse, json, time, statistics, sys, csv
from urllib.request import urlopen, Request

def http_get_json(url: str, timeout: float):
    req = Request(url, headers={"User-Agent": "ippantime-probe/2.0"})
    with urlopen(req, timeout=timeout) as r:
        return json.loads(r.read())

def pick_int(payload: dict, key: str):
    v = payload.get(key, None)
    if isinstance(v, bool) or v is None:
        return None
    if isinstance(v, int):
        return v
    if isinstance(v, float):
        return int(v)
    if isinstance(v, str) and v.isdigit():
        return int(v)
    return None

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

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--targets-json", required=True, help="JSON list: [{node,url},...]")
    ap.add_argument("--status-path", default="/status")
    ap.add_argument("--ippan-field", default="ippan_time_us")
    ap.add_argument("--samples", type=int, default=600)
    ap.add_argument("--interval-ms", type=int, default=25)
    ap.add_argument("--timeout", type=float, default=2.0)
    ap.add_argument("--out", required=True, help="Output CSV path")
    ap.add_argument("--probe-name", required=True, help="Name of this probe vantage point (tr-a, tr-b, srv1, etc.)")
    args = ap.parse_args()

    targets = json.loads(args.targets_json)
    if not isinstance(targets, list) or not targets:
        print("targets-json must be a non-empty JSON list", file=sys.stderr)
        sys.exit(2)

    offsets = {t["node"]: [] for t in targets}
    jitters = {t["node"]: [] for t in targets}
    rtts = {t["node"]: [] for t in targets}
    backwards = {t["node"]: 0 for t in targets}
    last_ippan = {t["node"]: None for t in targets}
    last_offset = {t["node"]: None for t in targets}

    with open(args.out, "w", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerow([
            "probe","sample_idx","node","url","ok",
            "t0_wall_ns","t1_wall_ns","rtt_ms","mid_wall_us",
            "ippan_time_us","offset_us","jitter_us","monotonic_backwards"
        ])

        for si in range(args.samples):
            for t in targets:
                node = t["node"]
                url = t["url"].rstrip("/") + args.status_path

                t0 = time.time_ns()
                ok = 1
                t1 = None
                rtt_ms = None
                mid_wall_us = None
                ippan_us = None
                offset_us = None
                jitter_us = None
                mono_back = 0

                try:
                    payload = http_get_json(url, args.timeout)
                    t1 = time.time_ns()
                    rtt_ms = (t1 - t0) / 1_000_000.0
                    mid_wall_us = int(((t0 + t1) // 2) // 1_000)
                    ippan_us = pick_int(payload, args.ippan_field)
                    if ippan_us is None:
                        ok = 0
                    else:
                        offset_us = int(ippan_us - mid_wall_us)
                        offsets[node].append(offset_us)
                        rtts[node].append(rtt_ms)

                        # jitter = delta offset from last sample for that node
                        if last_offset[node] is not None:
                            jitter_us = int(offset_us - last_offset[node])
                            jitters[node].append(jitter_us)
                        last_offset[node] = offset_us

                        # monotonic check
                        if last_ippan[node] is not None and ippan_us < last_ippan[node]:
                            backwards[node] += 1
                            mono_back = 1
                        last_ippan[node] = ippan_us

                except Exception:
                    t1 = time.time_ns()
                    rtt_ms = (t1 - t0) / 1_000_000.0
                    ok = 0

                w.writerow([
                    args.probe_name, si, node, url, ok,
                    t0, t1 if t1 is not None else "",
                    f"{rtt_ms:.3f}" if rtt_ms is not None else "",
                    mid_wall_us if mid_wall_us is not None else "",
                    ippan_us if ippan_us is not None else "",
                    offset_us if offset_us is not None else "",
                    jitter_us if jitter_us is not None else "",
                    mono_back
                ])

            time.sleep(args.interval_ms / 1000.0)

    # Print small summary (stdout)
    print(f"\n=== PROBE SUMMARY [{args.probe_name}] ===")
    for t in targets:
        node = t["node"]
        off = offsets[node]
        jit = jitters[node]
        rt = rtts[node]
        if not off:
            print(f"{node}: NO VALID SAMPLES (missing field '{args.ippan_field}'?)")
            continue
        def p(v, q): return int(percentile(v, q)) if v else 0
        print(
            f"{node}  off_us p50={p(off,50)} p95={p(off,95)} p99={p(off,99)}"
            f" | jit_us p95={p([abs(x) for x in jit],95)} p99={p([abs(x) for x in jit],99)}"
            f" | rtt_ms p95={percentile(rt,95):.3f} p99={percentile(rt,99):.3f}"
            f" | backwards={backwards[node]}"
        )
    print(f"Wrote: {args.out}")

if __name__ == "__main__":
    main()
