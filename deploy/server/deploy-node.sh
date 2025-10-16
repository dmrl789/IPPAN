#!/usr/bin/env bash
set -euo pipefail

PUB_IP="${1:?public_ip}"
API_HOST_PORT="${2:-7080}"
P2P_PORT="${3:-4001}"

mkdir -p /opt/ippan/deploy
cd /opt/ippan

apt-get update -y
DEBIAN_FRONTEND=noninteractive apt-get install -y docker.io docker-compose-plugin jq ufw

tee docker-compose.override.clear-node-ports.yml >/dev/null <<'YML'
services:
  ippan-node-1:
    ports: []
YML

tee docker-compose.override.node.yml >/dev/null <<YML
services:
  ippan-node-1:
    image: ghcr.io/dmrl789/ippan:latest
    user: "0:0"
    command:
      - sh
      - -lc
      - |
        set -e
        echo 'deb http://deb.debian.org/debian bookworm main' > /etc/apt/sources.list.d/bookworm.list
        apt-get update -y
        apt-get install -y --no-install-recommends -t bookworm libssl3 ca-certificates
        exec ippan-node
    networks:
      default:
        aliases: [node]
    environment:
      IPPAN_P2P_LISTEN: "/ip4/0.0.0.0/tcp/${P2P_PORT}"
      IPPAN_P2P_ANNOUNCE: "/ip4/${PUB_IP}/tcp/${P2P_PORT}"
    ports:
      - "0.0.0.0:${P2P_PORT}:${P2P_PORT}"
      - "127.0.0.1:${API_HOST_PORT}:8080"
YML

docker compose -f docker-compose.yml -f docker-compose.full-stack.yml \
  -f docker-compose.override.clear-node-ports.yml -f docker-compose.override.node.yml up -d --no-build --no-deps ippan-node-1

ufw allow ${P2P_PORT}/tcp || true
ufw reload || true

curl -fsSL "http://127.0.0.1:${API_HOST_PORT}/health" | jq .
echo "P2P ${P2P_PORT} listening:"
ss -ltnp | grep ":${P2P_PORT}" || true
