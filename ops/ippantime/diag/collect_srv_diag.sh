#!/usr/bin/env bash
set -euo pipefail

# Read-only diagnostic bundle for jitter triage (CPU vs IO vs network queueing).
# Safe for production: captures snapshots and short counters only.
#
# Usage:
#   bash collect_srv_diag.sh [/root/ippantime/diag]

OUT_DIR="${1:-/root/ippantime/diag}"
TS="$(date +%Y%m%d_%H%M%S)"
DIR="$OUT_DIR/$TS"
mkdir -p "$DIR"

echo "Writing to $DIR"

have() { command -v "$1" >/dev/null 2>&1; }

{
  echo "=== HOST ==="
  hostname || true
  date -Is || true
  uname -a || true
  echo

  echo "=== TIME ==="
  timedatectl || true
  timedatectl timesync-status || true
  if have chronyc; then
    chronyc tracking || true
    chronyc sources -v || true
  fi
  echo

  echo "=== CPU/MEM ==="
  lscpu | sed -n '1,120p' || true
  free -h || true
  echo

  echo "=== LOAD ==="
  uptime || true
} > "$DIR/00_system.txt"

# Top snapshot
top -b -n 1 > "$DIR/01_top.txt" || true

# CPU contention short samples
if have mpstat; then mpstat -P ALL 1 5 > "$DIR/01b_mpstat.txt" 2>/dev/null || true; fi
if have vmstat; then vmstat 1 5 > "$DIR/01c_vmstat.txt" 2>/dev/null || true; fi
if have pidstat; then pidstat -durh 1 5 > "$DIR/01d_pidstat.txt" 2>/dev/null || true; fi

# IO + disk
if have iostat; then iostat -xz 1 10 > "$DIR/02_iostat.txt" 2>/dev/null || true; fi
df -hT > "$DIR/03_df.txt" || true
mount > "$DIR/04_mount.txt" || true

# Network + sockets
ss -s > "$DIR/05_ss_summary.txt" || true
ss -lntp > "$DIR/06_listeners.txt" || true
ip -s link > "$DIR/07_ip_link.txt" || true
ip route > "$DIR/08_routes.txt" || true
if have tc; then tc -s qdisc show > "$DIR/08b_tc_qdisc.txt" 2>/dev/null || true; fi
if have nstat; then nstat -az > "$DIR/08c_nstat.txt" 2>/dev/null || true; fi
cat /proc/interrupts > "$DIR/10_interrupts.txt" || true
cat /proc/softirqs > "$DIR/10b_softirqs.txt" 2>/dev/null || true
cat /proc/net/softnet_stat > "$DIR/10c_softnet_stat.txt" 2>/dev/null || true

# Best-effort NIC stats (try default route iface first, then eth0)
IFACE="$(ip route show default 2>/dev/null | awk '/default/ {print $5; exit}')"
{
  echo "default_iface=${IFACE:-<unknown>}"
} > "$DIR/09_iface.txt"
if have ethtool; then
  for i in "${IFACE:-}" eth0; do
    [[ -n "${i}" ]] || continue
    ethtool "$i" > "$DIR/09_ethtool_${i}.txt" 2>/dev/null || true
    ethtool -S "$i" > "$DIR/09_ethtool_stats_${i}.txt" 2>/dev/null || true
  done
fi

# Firewall view (read-only)
if have ufw; then ufw status verbose > "$DIR/12_ufw_status.txt" 2>/dev/null || true; fi
if have iptables; then iptables -S > "$DIR/12b_iptables_rules.txt" 2>/dev/null || true; fi

# Systemd unit + limits (adjust pattern if your units differ)
systemctl list-units --type=service | grep -i ippan > "$DIR/11_systemd_units.txt" || true
for u in $(systemctl list-units --type=service --no-legend | awk '{print $1}' | grep -E 'ippan'); do
  systemctl cat "$u" > "$DIR/systemd_${u}.txt" || true
done

# Recent logs (last 200 lines per unit)
for u in $(systemctl list-units --type=service --no-legend | awk '{print $1}' | grep -E 'ippan'); do
  journalctl -u "$u" -n 200 --no-pager > "$DIR/journal_${u}.txt" || true
done

echo "DONE: $DIR"

