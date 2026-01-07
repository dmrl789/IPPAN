# IPPAN Time jitter triage

Use when `ops/ippantime/out/REPORT.md` shows high jitter (e.g., Ashburn nodes 30–39 ~6ms p99) or a node outlier (e.g. `node-00`).

This kit is **read-only** and safe in production: it collects snapshots and short counter samples to help classify jitter as **I/O**, **CPU contention**, or **network queueing**.

## 0) Install / stage on the target server

Copy this folder to the server (example):

```bash
rsync -az ops/ippantime/diag/ root@<SRV_IP>:/root/ippantime/diag/
ssh root@<SRV_IP> "chmod +x /root/ippantime/diag/*.sh /root/ippantime/diag/*.py"
```

## 1) Collect on a server (recommended: srv4)

```bash
bash /root/ippantime/diag/collect_srv_diag.sh /root/ippantime/diag
```

## 2) Optional: operator reachability/RTT spot-check

```bash
bash ops/ippantime/diag/collect_operator_view.sh srv4 <IP> 8110 8119
```

## 3) Rank jitter hotspots from latest probe CSVs

```bash
python3 ops/ippantime/diag/analyze_jitter_hotspots.py
sed -n '1,120p' ops/ippantime/out/JITTER_HOTSPOTS.md
```

Outputs:

- `ops/ippantime/out/JITTER_HOTSPOTS.md`

## 4) Interpreting the server diag bundle (quick classifier)

The collector writes a timestamped folder like `/root/ippantime/diag/YYYYmmdd_HHMMSS/`.

### CPU contention

Look at:

- `01_top.txt` and `01b_mpstat.txt`
- **High `%usr`/`%sys`**, high run queue, high steal time (`%steal`), or one core pinned at 100% → likely CPU/IRQ contention.

First moves (ops-only):

- Use systemd `CPUAffinity=` for the IPPAN instances (see below).
- Ensure `irqbalance` is present/enabled, then verify IRQ distribution in `10_interrupts.txt`.

### I/O

Look at:

- `02_iostat.txt`
- **High `%util`**, high `await`, or high `svctm` on the disk hosting logs/DBs → likely I/O jitter.

First moves (ops-only):

- Move logs to faster disk / reduce log volume.
- Prefer `IOSchedulingClass=best-effort` + low `IOSchedulingPriority` for the node process.

### Network queueing / NIC

Look at:

- `08b_tc_qdisc.txt`, `09_ethtool_stats_<iface>.txt`, `10c_softnet_stat.txt`, `07_ip_link.txt`
- Drops, overruns, growing backlog, or qdisc queue build-up → likely NIC/queueing.

First moves (ops-only):

- Check IRQ distribution and consider pinning heavy NIC IRQs away from node cores.
- Verify MTU and offload settings are sane (`09_ethtool_<iface>.txt`).

## 5) Apply non-code mitigations (srv4 focus)

Do **only** these safe ops changes (no node code).

### 5A) CPU affinity for IPPAN instances (systemd)

Create overrides for the srv4 instances (adjust unit names):

```bash
systemctl edit ippan-node@30
```

Add:

```ini
[Service]
CPUAffinity=2 3 4 5
Nice=-5
IOSchedulingClass=best-effort
IOSchedulingPriority=2
```

Repeat for `@31..@39` (or use a drop-in template if you have one).

Restart:

```bash
systemctl daemon-reload
systemctl restart ippan-node@30 ippan-node@31 ippan-node@32 ippan-node@33 ippan-node@34 ippan-node@35 ippan-node@36 ippan-node@37 ippan-node@38 ippan-node@39
```

### 5B) Ensure RPC threads are not starved

If you have a separate RPC worker setting, raise it in env (ops-level) **only** if already supported by config/env.

### 5C) Reduce noisy logging on srv4 (if logs are heavy)

If you’re logging at debug/trace, lower to info/warn.

## 6) Optional: probe pinning (rule out probe-side jitter)

Run probes pinned to a stable core set (example):

```bash
taskset -c 2-5 bash ops/ippantime/run_probe_from_host.sh tr-a
taskset -c 2-5 bash ops/ippantime/run_probe_from_host.sh tr-b
```

## 7) Re-run the mesh and compare jitter

```bash
bash ops/ippantime/run_all.sh
python3 ops/ippantime/bin/analyze_ippantime.py
python3 ops/ippantime/diag/analyze_jitter_hotspots.py
```

Acceptance: `nodes 30–39 jitter p99` drops materially (target: < 3ms) while **backwards stays 0**.

