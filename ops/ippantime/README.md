# IPPAN Time Mesh Test

Runs IPPAN Time (`ippan_time_us`) sampling across 40 nodes (4 servers x 10) using 2 Falkenstein Threadrippers as probes (and optionally also probes from the 4 servers).

## Quick run

1. Edit `ops/ippantime/inventory.yaml` (hosts/IPs)
2. Ensure SSH key access to all probes (tr-a, tr-b, srv1..srv4)
3. Run:

```bash
bash ops/ippantime/run_all.sh
```

## 40-node readiness (bring up ports safely)

While you bring up the full 40-node port layout (and before you disable any `explicit_targets` fallback), run:

```bash
# keep running this while you bring up 8080–8119 (or your expected port layout)
python3 ops/ippantime/bin/check_targets.py ops/ippantime/inventory.yaml --mode expected || true
```

Once it reports `OK: 40` and exits 0, you can switch off any 4-target fallback and run the full mesh.

## Outputs

- `ops/ippantime/out/ippantime_<probe>.csv` — raw per-sample data
- `ops/ippantime/out/REPORT.md` — summary with rankings + cross-probe consistency

## What is measured

| Metric | Description |
|--------|-------------|
| `offset_us` | Difference between node's `ippan_time_us` and probe wall clock (mid-RTT) |
| `jitter_us` | Delta of offset between consecutive samples (stability indicator) |
| `rtt_ms` | Round-trip time from probe to node `/status` endpoint |
| `monotonic_backwards` | Count of times `ippan_time_us` decreased vs previous sample |

## File structure

```
ops/ippantime/
├── README.md                     # This file
├── inventory.yaml                # Edit: hosts, ports, probe config
├── bin/
│   ├── gen_targets.py            # Generate 40 node URLs from inventory
│   ├── ippantime_probe.py        # Main probe (run from TR-A/TR-B/servers)
│   ├── analyze_ippantime.py      # Aggregate CSVs → REPORT.md
│   └── check_targets.py          # OK/MISS reachability checker for expected/explicit target sets
├── remote/
│   ├── preflight_time.sh         # Check NTP/Chrony status on remote
│   └── preflight_http.sh         # Check node /status reachability
├── run_probe_from_host.sh        # Run probe locally (called on each probe host)
├── run_mesh_from_all_probes.sh   # Orchestrate probes via SSH + collect results
├── run_all.sh                    # Single entry point
└── out/
    ├── .gitkeep
    ├── ippantime_tr-a.csv        # (generated)
    ├── ippantime_tr-b.csv        # (generated)
    └── REPORT.md                 # (generated)
```

## Requirements

- Python 3.8+ with PyYAML (`pip install pyyaml`)
- SSH access to all probe hosts (passwordless keys recommended)
- `rsync` available on operator machine and probe hosts
- Nodes exposing `GET /status` returning JSON with `ippan_time_us` field

## Interpreting Results

### Offset signatures

| Pattern | Likely cause |
|---------|--------------|
| Large offset (>10ms), low jitter | Clock drift — tune Chrony |
| Small offset, high jitter | Network congestion or node overload |
| Monotonic backwards > 0 | Time service bug (should never happen) |

### Cross-probe consistency

If TR-A and TR-B see different p99 offsets for the same node, the difference indicates:
- Network asymmetry between probe locations
- Or the node's time drifting mid-test

## Customization

Edit `inventory.yaml`:
- `servers[].node_index_range` — which node indices run on each server
- `http.base_port` — base port (node i uses port BASE_PORT + i)
- `probe.samples` — number of samples per node (default 600)
- `probe.interval_ms` — milliseconds between sample rounds (default 25)

For non-standard port layouts, use `explicit_targets` instead of auto-generation.
