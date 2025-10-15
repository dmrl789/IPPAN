#!/usr/bin/env bash
set -euo pipefail

# Deploys the IPPAN gateway/API host (REST + WS) behind Nginx with TLS.
#
# Required/optional environment variables:
#   API_DOMAIN (default: api.ippan.org)
#   LETSENCRYPT_EMAIL (required)
#   NODE_RPC_URL (default: http://10.0.0.5:8545)
#   NODE_WS_URL (default: ws://10.0.0.5:8546)
#   ALLOWED_ORIGINS (default: https://ui.ippan.org)
#   API_IMAGE (default: ghcr.io/dmrl789/ippan-gateway:latest)
#   GATEWAY_IP (default: 10.0.0.4)
#   DOCKER_NETWORK (default: ippan_net)
#   NETWORK_SUBNET (default: 10.0.0.0/24)
#   COMPOSE_DIR (default: /opt/ippan/gateway)
#
# The script installs Docker, Nginx, configures docker-compose, provisions
# Let's Encrypt certificates, and enforces firewall rules.

log() {
  printf '[api] %s\n' "$*"
}

require_root() {
  if [[ $(id -u) -ne 0 ]]; then
    log 'this script must be run as root'
    exit 1
  fi
}

require_var() {
  local name="$1"
  if [[ -z "${!name:-}" ]]; then
    log "environment variable '$name' must be set"
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
    ufw \
    nginx \
    certbot \
    python3-certbot-nginx
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
  local compose_path="$compose_dir/docker-compose.yml"
  local env_path="$compose_dir/gateway.env"

  mkdir -p "$compose_dir"

  cat > "$env_path" <<ENV
NODE_RPC_URL=${NODE_RPC_URL}
NODE_WS_URL=${NODE_WS_URL}
GATEWAY_ALLOWED_ORIGINS=${ALLOWED_ORIGINS}
ENV
  chmod 600 "$env_path"

  cat > "$compose_path" <<COMPOSE
version: "3.9"
services:
  ippan-gateway:
    image: ${API_IMAGE}
    container_name: ippan-gateway
    restart: always
    env_file:
      - ${env_path}
    environment:
      - RUST_LOG=info
    ports:
      - "127.0.0.1:8080:8080"
    networks:
      ${DOCKER_NETWORK}:
        ipv4_address: ${GATEWAY_IP}
    healthcheck:
      test: ["CMD", "curl", "-fsS", "http://127.0.0.1:8080/health"]
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
  local unit_path="/etc/systemd/system/ippan-gateway.service"

  cat > "$unit_path" <<UNIT
[Unit]
Description=IPPAN Gateway stack
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
  systemctl enable --now ippan-gateway.service
  log 'systemd service ippan-gateway.service enabled'
}

configure_nginx() {
  local domain="$1"
  local site_path="/etc/nginx/sites-available/${domain}.conf"

  cat > "$site_path" <<NGINX
server {
    listen 80;
    server_name ${domain};

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
    }

    location /ws {
        proxy_pass http://127.0.0.1:8080/ws;
        proxy_http_version 1.1;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_cache_bypass \$http_upgrade;
    }
}
NGINX

  ln -sf "$site_path" /etc/nginx/sites-enabled/
  nginx -t
  systemctl enable --now nginx
}

obtain_certificate() {
  local domain="$1"
  local email="$2"
  log "requesting Let's Encrypt certificate for ${domain}"
  certbot --nginx --non-interactive --agree-tos -m "$email" -d "$domain" --redirect
}

configure_firewall() {
  log 'configuring ufw firewall rules'
  ufw allow OpenSSH
  ufw allow 'Nginx Full'
  ufw --force enable
}

pull_and_start() {
  local compose_dir="$1"
  docker compose -f "$compose_dir/docker-compose.yml" pull
  docker compose -f "$compose_dir/docker-compose.yml" up -d --remove-orphans
}

main() {
  require_root

  API_DOMAIN=${API_DOMAIN:-api.ippan.org}
  require_var LETSENCRYPT_EMAIL
  NODE_RPC_URL=${NODE_RPC_URL:-http://10.0.0.5:8545}
  NODE_WS_URL=${NODE_WS_URL:-ws://10.0.0.5:8546}
  ALLOWED_ORIGINS=${ALLOWED_ORIGINS:-https://ui.ippan.org}
  API_IMAGE=${API_IMAGE:-ghcr.io/dmrl789/ippan-gateway:latest}
  GATEWAY_IP=${GATEWAY_IP:-10.0.0.4}
  DOCKER_NETWORK=${DOCKER_NETWORK:-ippan_net}
  NETWORK_SUBNET=${NETWORK_SUBNET:-10.0.0.0/24}
  COMPOSE_DIR=${COMPOSE_DIR:-/opt/ippan/gateway}

  install_packages
  install_docker
  ensure_network "$DOCKER_NETWORK" "$NETWORK_SUBNET"
  write_compose "$COMPOSE_DIR"
  write_systemd_unit "$COMPOSE_DIR"
  pull_and_start "$COMPOSE_DIR"
  configure_nginx "$API_DOMAIN"
  obtain_certificate "$API_DOMAIN" "$LETSENCRYPT_EMAIL"
  configure_firewall

  log 'deployment complete'
}

main "$@"
