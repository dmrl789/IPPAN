#!/usr/bin/env bash
set -euo pipefail

# DevNet “backwards” fix: consolidate consensus env + rolling restart + proofs
#
# Usage:
#   VIDS="..." DLC_PATH="/opt/ippan/config/dlc.toml" ./scripts/ops/devnet_backwards_fix.sh
# Or rely on defaults below (VIDS is required).

VIDS="${VIDS:-}"
if [[ -z "${VIDS}" ]]; then
  echo "ERROR: VIDS env var is required (comma-separated validator ids)." >&2
  exit 2
fi

DLC_PATH="${DLC_PATH:-/opt/ippan/config/dlc.toml}"

# Optional overrides (space-separated hostnames/IPs):
#   DEVNET_NODES="api1.ippan.uk api2.ippan.uk api3.ippan.uk api4.ippan.uk"
#   DEVNET_ROLL="api2.ippan.uk api3.ippan.uk api4.ippan.uk api1.ippan.uk"
#
# Defaults match the original api1..api4 IPs.
DEFAULT_NODES=(188.245.97.41 135.181.145.174 5.223.51.238 178.156.219.107)
DEFAULT_ROLL=(135.181.145.174 5.223.51.238 178.156.219.107 188.245.97.41)

if [[ -n "${DEVNET_NODES:-}" ]]; then
  read -r -a NODES <<< "${DEVNET_NODES}"
else
  NODES=("${DEFAULT_NODES[@]}")
fi

if [[ -n "${DEVNET_ROLL:-}" ]]; then
  read -r -a ROLL <<< "${DEVNET_ROLL}"
else
  ROLL=("${DEFAULT_ROLL[@]}")
fi

authoritative_dropin_path="/etc/systemd/system/ippan-node.service.d/60-devnet-consensus.conf"

log() { printf '%s\n' "$*"; }

assert_single_validator_ids_source() {
  # Enforce "one source of truth" for validator ids: exactly one non-disabled file
  # should define IPPAN_VALIDATOR_IDS. If multiple are active, drift will eventually
  # reappear depending on drop-in ordering and future edits.
  local ip="$1"
  log "=== assert single validator-ids source on ${ip} ==="
  ssh root@"${ip}" 'set -euo pipefail
    d=/etc/systemd/system/ippan-node.service.d
    active_files=$(ls -1 "$d" | grep -v "\.disabled$" || true)
    # Count occurrences across active drop-ins + unit file.
    hits=$(
      (
        for f in $active_files; do
          grep -H "IPPAN_VALIDATOR_IDS" "$d/$f" 2>/dev/null || true
        done
        grep -H "IPPAN_VALIDATOR_IDS" /etc/systemd/system/ippan-node.service 2>/dev/null || true
      ) | wc -l
    )

    if [ "$hits" -le 1 ]; then
      exit 0
    fi

    echo "ERROR: multiple active sources define IPPAN_VALIDATOR_IDS (drift risk)" >&2
    (
      for f in $active_files; do
        grep -H "IPPAN_VALIDATOR_IDS" "$d/$f" 2>/dev/null || true
      done
      grep -H "IPPAN_VALIDATOR_IDS" /etc/systemd/system/ippan-node.service 2>/dev/null || true
    ) >&2
    exit 11
  '
}

install_authoritative_dropin() {
  for ip in "${NODES[@]}"; do
    log "=== install 60-devnet-consensus.conf on ${ip} ==="
    ssh root@"${ip}" 'set -euo pipefail
      sudo mkdir -p /etc/systemd/system/ippan-node.service.d
      sudo tee '"${authoritative_dropin_path}"' >/dev/null <<EOF
[Service]
Environment=IPPAN_VALIDATOR_IDS='"${VIDS}"'
Environment=ENABLE_DLC=true
Environment=IPPAN_DLC_CONFIG_PATH='"${DLC_PATH}"'
EOF
      sudo systemctl daemon-reload
    '
  done
}

disable_drift_dropins() {
  # 4.1 Disable legacy validator set env
  for ip in "${NODES[@]}"; do
    log "=== disable legacy consensus validators on ${ip} ==="
    ssh root@"${ip}" 'set -euo pipefail
      f=/etc/systemd/system/ippan-node.service.d/99-consensus-validators.conf
      if [ -f "$f" ]; then sudo mv -f "$f" "${f}.disabled"; fi
      sudo systemctl daemon-reload
    '
  done

  # 4.1b Disable extra validator set drop-in (keep ONE source of truth: 60-devnet-consensus.conf)
  for ip in "${NODES[@]}"; do
    log "=== disable extra validator set drop-in on ${ip} ==="
    ssh root@"${ip}" 'set -euo pipefail
      f=/etc/systemd/system/ippan-node.service.d/99-validator-set.conf
      if [ -f "$f" ]; then sudo mv -f "$f" "${f}.disabled"; fi
      sudo systemctl daemon-reload
    '
  done

  # 4.2 Disable duplicate DLC-path overrides
  for ip in "${NODES[@]}"; do
    log "=== disable duplicate DLC path drop-ins on ${ip} ==="
    ssh root@"${ip}" 'set -euo pipefail
      for f in \
        /etc/systemd/system/ippan-node.service.d/31-dlc-runtime.conf \
        /etc/systemd/system/ippan-node.service.d/99-dlc-config-path.conf \
        /etc/systemd/system/ippan-node.service.d/15-dlc-config.conf
      do
        if [ -f "$f" ]; then sudo mv -f "$f" "${f}.disabled"; fi
      done
      sudo systemctl daemon-reload
    '
  done

  # 4.3 Disable extra bootstrap override spam (optional but recommended)
  for ip in "${NODES[@]}"; do
    log "=== disable extra bootstrap overrides on ${ip} ==="
    ssh root@"${ip}" 'set -euo pipefail
      for f in \
        /etc/systemd/system/ippan-node.service.d/30-bootstrap.conf \
        /etc/systemd/system/ippan-node.service.d/99-bootstrap-override.conf
      do
        if [ -f "$f" ]; then sudo mv -f "$f" "${f}.disabled"; fi
      done
      sudo systemctl daemon-reload
    '
  done

  # Safety check: after drift cleanup, enforce single-source validator id config.
  for ip in "${NODES[@]}"; do
    assert_single_validator_ids_source "${ip}"
  done
}

restart_and_prove() {
  for ip in "${ROLL[@]}"; do
    log "=== restart ${ip} ==="
    ssh root@"${ip}" 'set -euo pipefail
      sudo systemctl restart ippan-node
      sleep 3
      sudo systemctl is-active ippan-node

      ok=0
      for i in $(seq 1 12); do
        # If the service is still warming up, /status can return empty / non-JSON briefly.
        if curl -sS -m 15 http://127.0.0.1:8080/status | python3 -c "import json,sys; s=json.load(sys.stdin); print(\"validator_count:\", s.get(\"validator_count\")); print(\"peer_count:\", s.get(\"peer_count\")); print(\"validator_ids_sample:\", s.get(\"validator_ids_sample\")); print(\"round:\", (s.get(\"consensus\") or {}).get(\"round\"))" 2>/dev/null; then
          ok=1
          break
        fi
        sleep 2
      done

      if [ "$ok" -ne 1 ]; then
        echo "ERROR: /status did not return valid JSON after restart" >&2
        echo "--- curl (headers) ---" >&2
        curl -sS -m 10 -D- http://127.0.0.1:8080/status -o /tmp/ippan_status_body.txt >&2 || true
        echo "--- body (first 400 bytes) ---" >&2
        head -c 400 /tmp/ippan_status_body.txt >&2 || true
        echo "" >&2
        echo "--- systemctl status ippan-node ---" >&2
        sudo systemctl status ippan-node --no-pager >&2 || true
        echo "--- journalctl -u ippan-node (last 80 lines) ---" >&2
        sudo journalctl -u ippan-node -n 80 --no-pager >&2 || true
        exit 10
      fi
    '
  done
}

deliverable_dropins() {
  for ip in "${NODES[@]}"; do
    log "=== drop-ins on ${ip} ==="
    ssh root@"${ip}" 'ls -1 /etc/systemd/system/ippan-node.service.d | sed "s/^/  /"'
  done
}

deliverable_status() {
  for ip in "${NODES[@]}"; do
    log "=== status ${ip} ==="
    ssh root@"${ip}" 'curl -sS -m 8 http://127.0.0.1:8080/status | python3 -c "import json,sys; s=json.load(sys.stdin); print({\"validator_count\": s.get(\"validator_count\"), \"peer_count\": s.get(\"peer_count\"), \"round\": (s.get(\"consensus\") or {}).get(\"round\"), \"validator_ids_sample\": s.get(\"validator_ids_sample\")})"'
  done
}

main() {
  log "Using DLC_PATH=${DLC_PATH}"

  case "${1:-all}" in
    all)
      install_authoritative_dropin
      disable_drift_dropins
      restart_and_prove
      deliverable_dropins
      deliverable_status
      ;;
    install)
      install_authoritative_dropin
      ;;
    disable)
      disable_drift_dropins
      ;;
    restart)
      restart_and_prove
      ;;
    deliverables)
      deliverable_dropins
      deliverable_status
      ;;
    *)
      echo "Usage: $0 [all|install|disable|restart|deliverables]" >&2
      exit 2
      ;;
  esac
}

main "$@"


