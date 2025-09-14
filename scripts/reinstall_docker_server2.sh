#!/bin/bash
set -euo pipefail

echo "=== Stopping Docker and containerd (if running) ==="
sudo systemctl stop docker 2>/dev/null || true
sudo systemctl stop containerd 2>/dev/null || true

echo "=== Purging existing Docker packages ==="
sudo apt-get update -y
sudo apt-get remove -y docker.io docker-doc docker-compose docker-compose-v2 containerd runc 2>/dev/null || true
sudo apt-get purge -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin docker-ce-rootless-extras 2>/dev/null || true

echo "=== Removing Docker data and config ==="
sudo rm -rf /var/lib/docker /var/lib/containerd /etc/docker

echo "=== Installing Docker using get.docker.com ==="
curl -fsSL https://get.docker.com -o /tmp/get-docker.sh
sudo sh /tmp/get-docker.sh
rm -f /tmp/get-docker.sh

echo "=== Writing minimal /etc/docker/daemon.json ==="
sudo mkdir -p /etc/docker
sudo tee /etc/docker/daemon.json >/dev/null <<'JSON'
{
  "log-driver": "json-file",
  "log-opts": {
    "max-size": "100m",
    "max-file": "3"
  },
  "storage-driver": "overlay2"
}
JSON

echo "=== Enabling and starting Docker ==="
sudo systemctl daemon-reload
sudo systemctl enable --now docker
sudo usermod -aG docker "$USER" || true
sleep 3
docker --version || true

echo "=== Bringing up IPPAN stack ==="
cd /opt/ippan/mainnet || exit 0
docker-compose -f docker-compose.production.yml up -d || true

echo "=== Current containers ==="
docker ps --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}' || true

echo "=== Checking listening ports (3000,8080,9090) ==="
ss -tln | grep -E ':(3000|8080|9090)' || true

echo "=== Done ==="

