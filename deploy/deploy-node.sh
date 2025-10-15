#!/usr/bin/env bash
set -euo pipefail

# Deploys an IPPAN validator/worker node inside Docker.
#
# Configurable environment variables:
#   NODE_NAME (default: ippan-node)
#   NODE_ROLE (default: validator)
#   NODE_IMAGE (default: ghcr.io/dmrl789/ippan-node:latest)
#   NODE_IP (default: 10.0.0.5)
#   NODE_BIND_IP (default: 0.0.0.0)
#   SEED_PEERS (default: node2.ippan.org:8545)
#   EXTRA_ENV (optional additional env lines appended to node.env)
#   DATA_DIR (default: /var/lib/ippan/node)
#   DOCKER_NETWORK (default: ippan_net)
#   NETWORK_SUBNET (default: 10.0.0.0/24)
#   COMPOSE_DIR (default: /opt/ippan/node)
#   ALLOWED_RPC_CIDR (default: 10.0.0.0/24)
#
# The node only exposes RPC/WebSocket ports to trusted CIDRs via UFW.

log() {
  printf '[node] %s\n' "$*"
}

require_root() {
  if [[ $(id -u) -ne 0 ]]; then
    log 'this script must be run as root'
    exit 1
  fi
}

install_packages() {
  log 'installing apt dependencies'
  apt-get update
  DEBIAN_FRONTEND=noninteractive apt-get install -y \
    ca-certificates \
    curl \
    gnupg \
    lsb-release \
    ufw
}

install_docker() {
  if command -v docker >/dev/null 2>&1; then
    log 'docker already installed; skipping'
    return
  fi

  log 'installing docker engine and compose plugin'
  install -m 0755 -d /etc/apt/keyrings
  curl -fsSL https://download.docker.com/linux/$(. /etc/os-release && echo "$ID")/gpg \
    | gpg --dearmor -o /etc/apt/keyrings/docker.gpg
  chmod a+r /etc/apt/keyrings/docker.gpg

  local codename
  codename=$(. /etc/os-release && echo "$VERSION_CODENAME")
  echo \
"deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/$(. /etc/os-release && echo "$ID") \"$codename\" stable" \
    > /etc/apt/sources.list.d/docker.list

  apt-get update
  DEBIAN_FRONTEND=noninteractive apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
  systemctl enable --now docker
}

ensure_network() {
  local network_name="$1"
  local subnet="$2"

  if docker network inspect "$network_name" >/dev/null 2>&1; then
    log "docker network '$network_name' already exists"
    return
  fi

  log "creating docker network '$network_name' with subnet '$subnet'"
  docker network create --subnet="$subnet" "$network_name"
}

write_compose() {
  local compose_dir="$1"
  local data_dir="$2"
  local compose_path="$compose_dir/docker-compose.yml"
  local env_path="$compose_dir/node.env"

  mkdir -p "$compose_dir" "$data_dir"
  chmod 700 "$data_dir"

  cat > "$env_path" <<ENV
IPPAN_ROLE=${NODE_ROLE}
IPPAN_SEED_PEERS=${SEED_PEERS}
ENV

  if [[ -n "${EXTRA_ENV:-}" ]]; then
    printf '\n%s\n' "$EXTRA_ENV" >> "$env_path"
  fi
  chmod 600 "$env_path"

  cat > "$compose_path" <<COMPOSE
version: "3.9"
services:
  ${NODE_NAME}:
    image: ${NODE_IMAGE}
    container_name: ${NODE_NAME}
    restart: always
    env_file:
      - ${env_path}
    volumes:
      - ${data_dir}:/var/lib/ippan
    ports:
      - "${NODE_BIND_IP}:8545:8545"
      - "${NODE_BIND_IP}:8546:8546"
    networks:
      ${DOCKER_NETWORK}:
        ipv4_address: ${NODE_IP}
    healthcheck:
      test: ["CMD", "curl", "-fsS", "http://127.0.0.1:8545/health"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 20s
networks:
  ${DOCKER_NETWORK}:
    external: true
COMPOSE

  log "docker compose file written to $compose_path"
}

write_systemd_unit() {
  local compose_dir="$1"
  local unit_path="/etc/systemd/system/${NODE_NAME}.service"

  cat > "$unit_path" <<UNIT
[Unit]
Description=IPPAN node ${NODE_NAME}
Requires=docker.service
After=docker.service

[Service]
Type=oneshot
RemainAfterExit=yes
WorkingDirectory=${compose_dir}
ExecStart=/usr/bin/docker compose up -d --remove-orphans
ExecStop=/usr/bin/docker compose down
ExecReload=/usr/bin/docker compose up -d --remove-orphans
TimeoutStartSec=0

[Install]
WantedBy=multi-user.target
UNIT

  systemctl daemon-reload
  systemctl enable --now "${NODE_NAME}.service"
  log "systemd service ${NODE_NAME}.service enabled"
}

configure_firewall() {
  log 'configuring ufw firewall rules'
  ufw allow OpenSSH
  ufw allow from ${ALLOWED_RPC_CIDR} to any port 8545 proto tcp
  ufw allow from ${ALLOWED_RPC_CIDR} to any port 8546 proto tcp
  ufw --force enable
}

pull_and_start() {
  local compose_dir="$1"
  docker compose -f "$compose_dir/docker-compose.yml" pull
  docker compose -f "$compose_dir/docker-compose.yml" up -d --remove-orphans
}

main() {
  require_root

  NODE_NAME=${NODE_NAME:-ippan-node}
  NODE_ROLE=${NODE_ROLE:-validator}
  NODE_IMAGE=${NODE_IMAGE:-ghcr.io/dmrl789/ippan-node:latest}
  NODE_IP=${NODE_IP:-10.0.0.5}
  NODE_BIND_IP=${NODE_BIND_IP:-0.0.0.0}
  SEED_PEERS=${SEED_PEERS:-node2.ippan.org:8545}
  DATA_DIR=${DATA_DIR:-/var/lib/ippan/node}
  DOCKER_NETWORK=${DOCKER_NETWORK:-ippan_net}
  NETWORK_SUBNET=${NETWORK_SUBNET:-10.0.0.0/24}
  COMPOSE_DIR=${COMPOSE_DIR:-/opt/ippan/node}
  ALLOWED_RPC_CIDR=${ALLOWED_RPC_CIDR:-10.0.0.0/24}

  install_packages
  install_docker
  ensure_network "$DOCKER_NETWORK" "$NETWORK_SUBNET"
  write_compose "$COMPOSE_DIR" "$DATA_DIR"
  write_systemd_unit "$COMPOSE_DIR"
  pull_and_start "$COMPOSE_DIR"
  configure_firewall

  log 'deployment complete'
}

main "$@"
