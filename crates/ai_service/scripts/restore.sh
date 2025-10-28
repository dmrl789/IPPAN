#!/bin/bash

# Production restore script for IPPAN AI Service

set -euo pipefail

# Configuration
BACKUP_DIR="/opt/backups/ippan-ai-service"
SERVICE_NAME="ippan-ai-service"
SERVICE_DIR="/opt/ippan-ai-service"

# Check if backup file is provided
if [ $# -eq 0 ]; then
    echo "❌ Please provide a backup file to restore"
    echo "Usage: $0 <backup_file>"
    echo "Available backups:"
    ls -lh "$BACKUP_DIR"/*.tar.gz 2>/dev/null || echo "No backups found"
    exit 1
fi

BACKUP_FILE="$1"

# Check if backup file exists
if [ ! -f "$BACKUP_FILE" ]; then
    echo "❌ Backup file not found: $BACKUP_FILE"
    exit 1
fi

echo "🔄 Starting restore of $SERVICE_NAME from $BACKUP_FILE at $(date)"

# Stop the service
echo "⏹️ Stopping service..."
systemctl stop ippan-ai-service || echo "Service was not running"

# Create backup of current state
echo "💾 Creating backup of current state..."
CURRENT_BACKUP="$BACKUP_DIR/current_state_$(date +"%Y%m%d_%H%M%S").tar.gz"
tar -czf "$CURRENT_BACKUP" -C "$SERVICE_DIR" config/ data/ logs/ 2>/dev/null || true

# Create service directory if it doesn't exist
mkdir -p "$SERVICE_DIR"

# Restore from backup
echo "📦 Restoring from backup..."
tar -xzf "$BACKUP_FILE" -C "$SERVICE_DIR"

# Set proper permissions
echo "🔐 Setting permissions..."
chown -R ippan:ippan "$SERVICE_DIR"
chmod -R 755 "$SERVICE_DIR"

# Start the service
echo "▶️ Starting service..."
systemctl start ippan-ai-service

# Wait for service to start
echo "⏳ Waiting for service to start..."
sleep 10

# Check service status
if systemctl is-active --quiet ippan-ai-service; then
    echo "✅ Service started successfully"
else
    echo "❌ Service failed to start"
    echo "📋 Service status:"
    systemctl status ippan-ai-service
    echo "📋 Service logs:"
    journalctl -u ippan-ai-service --no-pager -n 50
    exit 1
fi

# Verify restore
echo "🔍 Verifying restore..."
if [ -d "$SERVICE_DIR/config" ] && [ -d "$SERVICE_DIR/data" ] && [ -d "$SERVICE_DIR/logs" ]; then
    echo "✅ Restore verification successful"
else
    echo "❌ Restore verification failed"
    exit 1
fi

echo "🎉 Restore completed successfully at $(date)"