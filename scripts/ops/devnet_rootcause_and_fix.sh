#!/usr/bin/env bash
set -euo pipefail

# DevNet “3 days no improvement / going backwards” root-cause + fix plan runner.
#
# This script is designed to be run from WSL bash (not PowerShell) and to provide
# hard proofs (invariants) before/after any drift fix.
#
# Usage (recommended):
#   export NODES="api1.ippan.uk api2.ippan.uk api3.ippan.uk api4.ippan.uk"
#   export ROLL="api2.ippan.uk api3.ippan.uk api4.ippan.uk api1.ippan.uk"
#   export VIDS="comma,separated,validator,ids"
#   export DLC_PATH="/opt/ippan/config/dlc.toml"
#   ./scripts/ops/devnet_rootcause_and_fix.sh all
#
# Optional bot pause:
#   export BOT_HOST="root@<FALKENSTEIN_IP>"
#   export BOT_SERVICE="ippan-tx-bot"
#   ./scripts/ops/devnet_rootcause_and_fix.sh bot-pause
#
# Phases:
#   snapshot | identity | round | systemd | fix | restart | deliverables | invariants | bot-pause | all

log() { printf '%s\n' "$*"; }

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || { echo "ERROR: missing required command: $1" >&2; exit 2; }
}

require_cmd curl
require_cmd python3
require_cmd ssh

NODES_STR="${NODES:-api1.ippan.uk api2.ippan.uk api3.ippan.uk api4.ippan.uk}"
ROLL_STR="${ROLL:-api2.ippan.uk api3.ippan.uk api4.ippan.uk api1.ippan.uk}"
read -r -a NODES <<< "${NODES_STR}"
read -r -a ROLL <<< "${ROLL_STR}"

# Avoid interactive host-key prompts in WSL / Cursor runs.
SSH_OPTS=(-o StrictHostKeyChecking=accept-new -o ConnectTimeout=7)

snapshot_status() {
  log "## PHASE 0 — Snapshot “what the network thinks” (/status on all nodes)"
  for h in "${NODES[@]}"; do
    log "=== ${h} ==="
    curl -sS -m 3 "http://${h}:8080/status" | python3 -c '
import json,sys
s=json.load(sys.stdin)
c=s.get("consensus") or {}
print("peer_count:", s.get("peer_count"))
print("validator_count:", s.get("validator_count"))
print("round:", c.get("round"))
print("rpc_queue:", s.get("rpc_queue_depth"), "/", s.get("rpc_queue_capacity"), "workers:", s.get("rpc_queue_workers"))
print("validator_ids_sample:", s.get("validator_ids_sample"))
'
  done
  log ""
  log "Decision rule: if any node shows peer_count=4 but validator_count=1 -> validator set drift (config/env mismatch)."
}

identity_check() {
  log "## PHASE 1 — Prove validator identity is NOT duplicated (best-effort log extraction)"
  for h in "${NODES[@]}"; do
    log "=== ${h} validator_id ==="
    ssh "${SSH_OPTS[@]}" root@"${h}" 'set -euo pipefail
      echo "-- /status identity fields (if exposed) --"
      curl -sS -m 3 http://127.0.0.1:8080/status | python3 -c '"'"'
import json,sys,re
s=json.load(sys.stdin)
wanted=[]
for k,v in s.items():
    lk=k.lower()
    if any(x in lk for x in ["self","peer_id","peerid","node_id","nodeid","validator_id","validatorid","identity","pubkey","public_key"]):
        wanted.append((k,v))
for k,v in sorted(wanted, key=lambda kv: kv[0].lower()):
    print(f"{k}: {v}")
'"'"' || true

      echo
      echo "-- node.toml identity/data hints --"
      sudo cat /etc/ippan/config/node.toml 2>/dev/null | grep -Ein "data|dir|path|key|identity|validator|secret|store|db" || true

      echo
      echo "-- log scrape (validator id if logged) --"
      sudo journalctl -u ippan-node --no-pager -n 600 \
        | egrep -oi "validator[_ -]?id[^a-f0-9]*[a-f0-9]{64}" \
        | tail -n 1 \
        || true

      echo
      echo "-- keyfile fingerprints (best-effort; look for any *key*/identity/validator files) --"
      # Try a few common base dirs; limit depth to keep it fast.
      for base in /var/lib/ippan /var/lib/ippan-node /opt/ippan /etc/ippan /home/ippan-devnet; do
        if [ -d "$base" ]; then
          find "$base" -maxdepth 6 -type f \( -iname "*validator*" -o -iname "*identity*" -o -iname "*key*" -o -iname "*secret*" \) -print0 2>/dev/null \
            | xargs -0 -r sha256sum 2>/dev/null \
            | sort \
            | head -n 40 \
            || true
        fi
      done
    '
  done
  log ""
  log "Decision rule: if the same 64-hex validator_id appears on multiple nodes -> shared validator.key or shared data dir (OPS bug)."
}

round_advance_check() {
  log "## PHASE 2 — Prove consensus is alive (round must advance)"
  for h in "${NODES[@]}"; do
    log "=== ${h} round advance check ==="
    A="$(curl -sS -m 3 "http://${h}:8080/status" | python3 -c 'import json,sys; s=json.load(sys.stdin); print((s.get("consensus") or {}).get("round"))')"
    sleep 3
    B="$(curl -sS -m 3 "http://${h}:8080/status" | python3 -c 'import json,sys; s=json.load(sys.stdin); print((s.get("consensus") or {}).get("round"))')"
    echo "${h} round: ${A} -> ${B}"
  done
  log ""
  log "Decision rule: if round is static on any node while peer_count is 4 -> consensus loop not running (startup drift / DLC mismatch / validator set mismatch)."
}

systemd_dump_one() {
  local h="${1:-api1.ippan.uk}"
  log "## PHASE 3 — Dump systemd env sources on ${h} (repeat on others if needed)"
  ssh "${SSH_OPTS[@]}" root@"${h}" 'set -euo pipefail
    echo "=== systemctl cat ippan-node ==="
    sudo systemctl cat ippan-node | sed -n "1,260p"
    echo
    echo "=== drop-ins list ==="
    ls -1 /etc/systemd/system/ippan-node.service.d | sed "s/^/  /"
  '
  log ""
  log "Red flags: multiple IPPAN_DLC_CONFIG_PATH, legacy CONSENSUS_VALIDATOR_IDS, duplicated bootstrap overrides."
}

fix_drift() {
  log "## PHASE 4/5 — Fix drift with ONE authoritative drop-in + rolling restart + proofs"
  if [[ -z "${VIDS:-}" ]]; then
    echo "ERROR: VIDS is required to apply the drift fix." >&2
    exit 2
  fi
  export DLC_PATH="${DLC_PATH:-/opt/ippan/config/dlc.toml}"
  export DEVNET_NODES="${NODES_STR}"
  export DEVNET_ROLL="${ROLL_STR}"
  ./scripts/ops/devnet_backwards_fix.sh all
}

restart_only() {
  log "## Rolling restart + proofs"
  export DLC_PATH="${DLC_PATH:-/opt/ippan/config/dlc.toml}"
  export DEVNET_NODES="${NODES_STR}"
  export DEVNET_ROLL="${ROLL_STR}"
  ./scripts/ops/devnet_backwards_fix.sh restart
}

deliverables_only() {
  log "## Deliverables — drop-ins + /status snippets"
  export DLC_PATH="${DLC_PATH:-/opt/ippan/config/dlc.toml}"
  export DEVNET_NODES="${NODES_STR}"
  export DEVNET_ROLL="${ROLL_STR}"
  ./scripts/ops/devnet_backwards_fix.sh deliverables
}

invariants_gate() {
  log "## PHASE 7 — Invariant gate"
  ./scripts/ops/devnet_invariant_check.sh "${NODES[@]}"
}

bot_pause() {
  log "## PHASE 6 — Bot sanity (optional): pause the bot during stabilization"
  if [[ -z "${BOT_HOST:-}" ]]; then
    echo "ERROR: set BOT_HOST (e.g. root@<FALKENSTEIN_IP>) to pause the bot." >&2
    exit 2
  fi
  svc="${BOT_SERVICE:-ippan-tx-bot}"
  ssh "${SSH_OPTS[@]}" "${BOT_HOST}" "sudo systemctl stop ${svc} || true; sudo systemctl is-active ${svc} || true"
}

main() {
  case "${1:-all}" in
    snapshot) snapshot_status ;;
    identity) identity_check ;;
    round) round_advance_check ;;
    systemd) systemd_dump_one "${2:-api1.ippan.uk}" ;;
    fix) fix_drift ;;
    restart) restart_only ;;
    deliverables) deliverables_only ;;
    invariants) invariants_gate ;;
    bot-pause) bot_pause ;;
    all)
      snapshot_status
      identity_check
      round_advance_check
      systemd_dump_one "${NODES[0]}"
      fix_drift
      invariants_gate
      ;;
    *)
      echo "Usage: $0 [snapshot|identity|round|systemd <host>|fix|restart|deliverables|invariants|bot-pause|all]" >&2
      exit 2
      ;;
  esac
}

main "$@"


