#!/usr/bin/env bash
set -euo pipefail

# ============================
# 🛡️ IPPAN Node Safe Cleanup & Restore Script
# ============================

BINARY_URL="https://example.com/path/to/ippan-node"   # TODO: set this
DATA_WIPE=false                                       # set to true to wipe /var/lib/ippan/data
BACKUP_FILE="/root/ippan-backup-$(date +%F_%H-%M-%S).tar.gz"

if [[ $EUID -ne 0 ]]; then
  echo "Please run as root." >&2
  exit 1
fi

echo "📦 Backing up current node config and binary to: $BACKUP_FILE"
tar -czf "$BACKUP_FILE" \
  /etc/ippan/config/node.toml \
  /etc/systemd/system/ippan-node.service \
  /usr/local/bin/ippan-node \
  2>/dev/null || true
echo "✅ Backup complete."

echo "🧹 Removing stale or broken IPPAN binary/config..."
rm -f /usr/local/bin/ippan-node
rm -f /etc/systemd/system/ippan-node.service
# Intentionally keep any overrides in /etc/systemd/system/ippan-node.service.d/

if [[ "$DATA_WIPE" == "true" ]]; then
  echo "⚠️ Wiping /var/lib/ippan/data ..."
  rm -rf /var/lib/ippan/data/*
fi

echo "📥 Installing known-good IPPAN binary and config..."
mkdir -p /etc/ippan/config
curl -fL "$BINARY_URL" -o /usr/local/bin/ippan-node
chmod +x /usr/local/bin/ippan-node

cat <<'EOF' > /etc/ippan/config/node.toml
bootstrap_nodes = [
  "http://188.245.97.41:9000",
  "http://135.181.145.174:9000",
  "http://5.223.51.238:9000",
  "http://178.156.219.107:9000"
]
EOF

cat <<'EOF' > /etc/systemd/system/ippan-node.service
[Unit]
Description=IPPAN Node
After=network.target

[Service]
ExecStart=/usr/local/bin/ippan-node --config /etc/ippan/config/node.toml
Restart=always
RestartSec=5s
User=root
# Preserve overrides in /etc/systemd/system/ippan-node.service.d/

[Install]
WantedBy=multi-user.target
EOF

echo "🔄 Reloading and restarting service..."
systemctl daemon-reexec
systemctl daemon-reload
systemctl restart ippan-node
systemctl enable ippan-node --now

echo "🔎 Verifying:"
ss -ltnp | grep :9000 || { echo "Port 9000 not listening"; exit 1; }
curl -sf http://127.0.0.1:9000/p2p/peers || { echo "Peers check failed"; exit 1; }
curl -sf http://127.0.0.1:8080/status || { echo "RPC status check failed"; exit 1; }

echo "✅ Done. Backup at $BACKUP_FILE"

