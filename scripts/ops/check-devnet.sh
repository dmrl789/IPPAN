#!/usr/bin/env bash
set -euo pipefail

# Devnet verifier: checks /status, /peers count, /time, and ippan-node sha256 across all devnet nodes.
#
# Requirements:
# - curl
# - jq
# - ssh (keys configured for root@<ip>)

RPC_PORT="${RPC_PORT:-8080}"

NODES=(
  "188.245.97.41"
  "135.181.145.174"
  "5.223.51.238"
  "178.156.219.107"
)

fail=0
hashes=()

for ip in "${NODES[@]}"; do
  echo "=== ${ip} ==="

  status_json="$(curl -fsS "http://${ip}:${RPC_PORT}/status")" || { echo "FAIL: /status"; fail=1; continue; }
  status="$(jq -r '.status' <<<"$status_json")"
  version="$(jq -r '.version // empty' <<<"$status_json")"
  peer_count="$(jq -r '.peer_count // empty' <<<"$status_json")"
  node_id="$(jq -r '.node_id // empty' <<<"$status_json")"

  peers_len="$(curl -fsS "http://${ip}:${RPC_PORT}/peers" | jq -r 'length')" || { echo "FAIL: /peers"; fail=1; continue; }
  time_us="$(curl -fsS "http://${ip}:${RPC_PORT}/time" | jq -r '.time_us')" || { echo "FAIL: /time"; fail=1; continue; }

  sha="$(ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new "root@${ip}" "sha256sum /usr/local/bin/ippan-node 2>/dev/null | awk '{print \$1}'" | head -n1 || true)"

  echo "status=${status} version=${version} node_id=${node_id} peer_count=${peer_count} peers_len=${peers_len} time_us=${time_us} sha256=${sha}"

  if [[ "$status" != "ok" ]]; then
    echo "FAIL: status != ok"
    fail=1
  fi
  if [[ -z "$sha" ]]; then
    echo "FAIL: missing sha256"
    fail=1
  else
    hashes+=("$sha")
  fi
done

unique_hashes="$(printf '%s\n' "${hashes[@]:-}" | sort -u | wc -l | tr -d ' ')"
echo "=== SUMMARY ==="
echo "unique_hashes=${unique_hashes}"

if [[ "${#hashes[@]}" -gt 0 ]] && [[ "$unique_hashes" -ne 1 ]]; then
  echo "FAIL: sha256 mismatch across nodes"
  exit 2
fi

if [[ "$fail" -ne 0 ]]; then
  exit 1
fi

echo "OK: all nodes healthy and binary hashes match"


