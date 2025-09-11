#!/bin/bash

# IPPAN Backup Script
# This script creates automated backups of IPPAN data and configurations

set -e

# Configuration
BACKUP_DIR="/backups/ippan"
DATE=$(date +%Y%m%d_%H%M%S)
RETENTION_DAYS=30
ENCRYPTION_KEY="${BACKUP_ENCRYPTION_KEY:-changeme123}"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Create backup directory
create_backup_dir() {
    log_info "Creating backup directory..."
    mkdir -p "$BACKUP_DIR/$DATE"
    log_success "Backup directory created: $BACKUP_DIR/$DATE"
}

# Backup database
backup_database() {
    log_info "Backing up database..."
    
    # Create database dump
    docker exec ippan-node sqlite3 /data/ippan.db ".backup '$BACKUP_DIR/$DATE/ippan.db'"
    
    # Compress database backup
    gzip "$BACKUP_DIR/$DATE/ippan.db"
    
    log_success "Database backup completed"
}

# Backup configuration files
backup_config() {
    log_info "Backing up configuration files..."
    
    # Copy configuration files
    cp -r config/ "$BACKUP_DIR/$DATE/"
    cp -r deployments/ "$BACKUP_DIR/$DATE/"
    cp -r scripts/ "$BACKUP_DIR/$DATE/"
    
    # Copy environment files
    cp .env.production "$BACKUP_DIR/$DATE/" 2>/dev/null || true
    
    log_success "Configuration backup completed"
}

# Backup keys and certificates
backup_keys() {
    log_info "Backing up keys and certificates..."
    
    # Create keys backup
    mkdir -p "$BACKUP_DIR/$DATE/keys"
    cp -r keys/ "$BACKUP_DIR/$DATE/" 2>/dev/null || true
    cp -r deployments/ssl/ "$BACKUP_DIR/$DATE/" 2>/dev/null || true
    
    log_success "Keys and certificates backup completed"
}

# Backup logs
backup_logs() {
    log_info "Backing up logs..."
    
    # Create logs backup
    mkdir -p "$BACKUP_DIR/$DATE/logs"
    cp -r logs/ "$BACKUP_DIR/$DATE/" 2>/dev/null || true
    
    # Get Docker logs
    docker logs ippan-node > "$BACKUP_DIR/$DATE/logs/ippan-node.log" 2>&1 || true
    
    log_success "Logs backup completed"
}

# Encrypt backup
encrypt_backup() {
    log_info "Encrypting backup..."
    
    # Create encrypted archive
    tar -czf - -C "$BACKUP_DIR" "$DATE" | \
    openssl enc -aes-256-cbc -salt -k "$ENCRYPTION_KEY" -out "$BACKUP_DIR/ippan_backup_$DATE.tar.gz.enc"
    
    # Remove unencrypted directory
    rm -rf "$BACKUP_DIR/$DATE"
    
    log_success "Backup encrypted: ippan_backup_$DATE.tar.gz.enc"
}

# Upload to cloud storage
upload_to_cloud() {
    if [ -n "$CLOUD_BACKUP_ENABLED" ] && [ "$CLOUD_BACKUP_ENABLED" = "true" ]; then
        log_info "Uploading backup to cloud storage..."
        
        # Upload to AWS S3
        if [ -n "$AWS_S3_BUCKET" ]; then
            aws s3 cp "$BACKUP_DIR/ippan_backup_$DATE.tar.gz.enc" "s3://$AWS_S3_BUCKET/backups/"
            log_success "Backup uploaded to S3: s3://$AWS_S3_BUCKET/backups/ippan_backup_$DATE.tar.gz.enc"
        fi
        
        # Upload to Google Cloud Storage
        if [ -n "$GCS_BUCKET" ]; then
            gsutil cp "$BACKUP_DIR/ippan_backup_$DATE.tar.gz.enc" "gs://$GCS_BUCKET/backups/"
            log_success "Backup uploaded to GCS: gs://$GCS_BUCKET/backups/ippan_backup_$DATE.tar.gz.enc"
        fi
    fi
}

# Cleanup old backups
cleanup_old_backups() {
    log_info "Cleaning up old backups..."
    
    # Remove local backups older than retention period
    find "$BACKUP_DIR" -name "ippan_backup_*.tar.gz.enc" -mtime +$RETENTION_DAYS -delete
    
    # Remove cloud backups older than retention period
    if [ -n "$CLOUD_BACKUP_ENABLED" ] && [ "$CLOUD_BACKUP_ENABLED" = "true" ]; then
        if [ -n "$AWS_S3_BUCKET" ]; then
            aws s3 ls "s3://$AWS_S3_BUCKET/backups/" | \
            awk '$1 < "'$(date -d "$RETENTION_DAYS days ago" +%Y-%m-%d)'" {print $4}' | \
            xargs -I {} aws s3 rm "s3://$AWS_S3_BUCKET/backups/{}"
        fi
    fi
    
    log_success "Old backups cleaned up"
}

# Verify backup integrity
verify_backup() {
    log_info "Verifying backup integrity..."
    
    # Test decryption
    openssl enc -aes-256-cbc -d -k "$ENCRYPTION_KEY" -in "$BACKUP_DIR/ippan_backup_$DATE.tar.gz.enc" | \
    tar -tzf - > /dev/null
    
    if [ $? -eq 0 ]; then
        log_success "Backup integrity verified"
    else
        log_error "Backup integrity check failed"
        exit 1
    fi
}

# Send backup notification
send_notification() {
    if [ -n "$SLACK_WEBHOOK_URL" ]; then
        curl -X POST -H 'Content-type: application/json' \
        --data "{\"text\":\"✅ IPPAN backup completed successfully: ippan_backup_$DATE.tar.gz.enc\"}" \
        "$SLACK_WEBHOOK_URL"
    fi
    
    if [ -n "$EMAIL_RECIPIENTS" ]; then
        echo "IPPAN backup completed successfully: ippan_backup_$DATE.tar.gz.enc" | \
        mail -s "IPPAN Backup Completed" "$EMAIL_RECIPIENTS"
    fi
}

# Main backup function
main() {
    log_info "Starting IPPAN backup process..."
    
    create_backup_dir
    backup_database
    backup_config
    backup_keys
    backup_logs
    encrypt_backup
    upload_to_cloud
    cleanup_old_backups
    verify_backup
    send_notification
    
    log_success "IPPAN backup process completed successfully!"
    echo ""
    echo "📁 Backup Information:"
    echo "  - Backup File: ippan_backup_$DATE.tar.gz.enc"
    echo "  - Location: $BACKUP_DIR"
    echo "  - Size: $(du -h "$BACKUP_DIR/ippan_backup_$DATE.tar.gz.enc" | cut -f1)"
    echo "  - Encryption: AES-256-CBC"
    echo "  - Retention: $RETENTION_DAYS days"
}

# Run main function
main "$@"
