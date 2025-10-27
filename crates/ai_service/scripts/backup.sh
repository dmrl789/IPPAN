#!/bin/bash

# Production backup script for IPPAN AI Service

set -euo pipefail

# Configuration
BACKUP_DIR="/opt/backups/ippan-ai-service"
SERVICE_NAME="ippan-ai-service"
RETENTION_DAYS=30

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Generate timestamp
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
BACKUP_FILE="$BACKUP_DIR/${SERVICE_NAME}_backup_$TIMESTAMP.tar.gz"

echo "🔄 Starting backup of $SERVICE_NAME at $(date)"

# Backup configuration
echo "📁 Backing up configuration..."
tar -czf "$BACKUP_DIR/config_$TIMESTAMP.tar.gz" -C /opt/ippan-ai-service config/

# Backup data
echo "📁 Backing up data..."
tar -czf "$BACKUP_DIR/data_$TIMESTAMP.tar.gz" -C /opt/ippan-ai-service data/

# Backup logs
echo "📁 Backing up logs..."
tar -czf "$BACKUP_DIR/logs_$TIMESTAMP.tar.gz" -C /opt/ippan-ai-service logs/

# Create full backup
echo "📦 Creating full backup..."
tar -czf "$BACKUP_FILE" \
    -C /opt/ippan-ai-service \
    config/ data/ logs/

# Verify backup
if [ -f "$BACKUP_FILE" ]; then
    echo "✅ Backup created successfully: $BACKUP_FILE"
    echo "📊 Backup size: $(du -h "$BACKUP_FILE" | cut -f1)"
else
    echo "❌ Backup failed!"
    exit 1
fi

# Cleanup old backups
echo "🧹 Cleaning up old backups..."
find "$BACKUP_DIR" -name "${SERVICE_NAME}_backup_*.tar.gz" -mtime +$RETENTION_DAYS -delete

# List current backups
echo "📋 Current backups:"
ls -lh "$BACKUP_DIR"/*.tar.gz 2>/dev/null || echo "No backups found"

echo "🎉 Backup completed successfully at $(date)"