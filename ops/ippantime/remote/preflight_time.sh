#!/usr/bin/env bash
set -euo pipefail
echo "== timedatectl =="
timedatectl || true
echo
echo "== timesync-status =="
timedatectl timesync-status || true
echo
echo "== chrony tracking =="
chronyc tracking || true
