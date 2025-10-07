#!/usr/bin/env bash
set -euo pipefail

PUB_IP="${1:?public_ip}"
UI_DOMAIN="${2:-ui.ippan.org}"
API_HOST_PORT="${3:-7080}"
P2P_PORT="${4:-4001}"

mkdir -p /opt/ippan/deploy
cd /opt/ippan

apt-get update -y
DEBIAN_FRONTEND=noninteractive apt-get install -y docker.io docker-compose-plugin jq ufw nginx

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

if [[ "$PUB_IP" == "188.245.97.41" ]]; then
  if [[ ! -d /opt/ippan/ui/dist ]]; then
    mkdir -p /opt/ippan/ui/dist
    cat >/opt/ippan/ui/dist/index.html <<'HTML'
<!doctype html><html><head><meta charset="utf-8"><title>IPPAN UI</title></head>
<body><h1>IPPAN Unified UI</h1><p>Build goes here.</p></body></html>
HTML
  fi

  tee /etc/nginx/sites-available/${UI_DOMAIN} >/dev/null <<NGINX
server {
  listen 80;
  server_name ${UI_DOMAIN};
  return 301 https://${UI_DOMAIN}\$request_uri;
}

server {
  listen 443 ssl http2;
  server_name ${UI_DOMAIN};

  ssl_certificate     /etc/letsencrypt/live/${UI_DOMAIN}/fullchain.pem;
  ssl_certificate_key /etc/letsencrypt/live/${UI_DOMAIN}/privkey.pem;

  root /opt/ippan/ui/dist;
  index index.html;

  location /api/ {
    proxy_pass         http://127.0.0.1:${API_HOST_PORT}/;
    proxy_http_version 1.1;
    proxy_set_header   Host \$host;
    proxy_set_header   X-Forwarded-For \$proxy_add_x_forwarded_for;
    proxy_set_header   X-Forwarded-Proto \$scheme;
    proxy_buffering    off;
  }

  location /ws {
    proxy_pass         http://127.0.0.1:${API_HOST_PORT}/ws;
    proxy_http_version 1.1;
    proxy_set_header   Upgrade \$http_upgrade;
    proxy_set_header   Connection "upgrade";
    proxy_set_header   Host \$host;
    proxy_read_timeout 600s;
    proxy_send_timeout 600s;
  }

  location / {
    try_files \$uri /index.html;
  }
}
NGINX

  ln -sf /etc/nginx/sites-available/${UI_DOMAIN} /etc/nginx/sites-enabled/${UI_DOMAIN}
  nginx -t
  systemctl enable --now nginx
  systemctl reload nginx || systemctl restart nginx
fi

curl -fsSL "http://127.0.0.1:${API_HOST_PORT}/health" | jq .
echo "P2P ${P2P_PORT} listening:"
ss -ltnp | grep ":${P2P_PORT}" || true
