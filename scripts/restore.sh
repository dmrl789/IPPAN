#!/bin/bash

# IPPAN Restore Script
# This script restores IPPAN from backup files

set -e

# Configuration
BACKUP_DIR="/backups/ippan"
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

# List available backups
list_backups() {
    log_info "Available backups:"
    ls -la "$BACKUP_DIR"/ippan_backup_*.tar.gz.enc 2>/dev/null || {
        log_error "No backups found in $BACKUP_DIR"
        exit 1
    }
}

# Select backup to restore
select_backup() {
    list_backups
    echo ""
    read -p "Enter backup filename to restore: " BACKUP_FILE
    
    if [ ! -f "$BACKUP_DIR/$BACKUP_FILE" ]; then
        log_error "Backup file not found: $BACKUP_DIR/$BACKUP_FILE"
        exit 1
    fi
    
    log_success "Selected backup: $BACKUP_FILE"
}

# Stop services
stop_services() {
    log_info "Stopping IPPAN services..."
    
    # Stop Docker containers
    docker-compose -f deployments/production/docker-compose.production.yml down
    
    log_success "Services stopped"
}

# Decrypt and extract backup
extract_backup() {
    log_info "Extracting backup..."
    
    # Create temporary directory
    TEMP_DIR="/tmp/ippan_restore_$(date +%s)"
    mkdir -p "$TEMP_DIR"
    
    # Decrypt and extract backup
    openssl enc -aes-256-cbc -d -k "$ENCRYPTION_KEY" -in "$BACKUP_DIR/$BACKUP_FILE" | \
    tar -xzf - -C "$TEMP_DIR"
    
    log_success "Backup extracted to: $TEMP_DIR"
}

# Restore database
restore_database() {
    log_info "Restoring database..."
    
    # Find backup directory
    BACKUP_DATE_DIR=$(find "$TEMP_DIR" -name "ippan.db.gz" -exec dirname {} \; | head -1)
    
    if [ -n "$BACKUP_DATE_DIR" ] && [ -f "$BACKUP_DATE_DIR/ippan.db.gz" ]; then
        # Decompress database
        gunzip "$BACKUP_DATE_DIR/ippan.db.gz"
        
        # Restore database
        cp "$BACKUP_DATE_DIR/ippan.db" /data/
        chown ippan:ippan /data/ippan.db
        
        log_success "Database restored"
    else
        log_warning "No database backup found"
    fi
}

# Restore configuration
restore_config() {
    log_info "Restoring configuration..."
    
    # Find backup directory
    BACKUP_DATE_DIR=$(find "$TEMP_DIR" -name "config" -type d | head -1)
    
    if [ -n "$BACKUP_DATE_DIR" ]; then
        # Restore configuration files
        cp -r "$BACKUP_DATE_DIR/config/" ./
        cp -r "$BACKUP_DATE_DIR/deployments/" ./
        cp -r "$BACKUP_DATE_DIR/scripts/" ./
        
        # Restore environment file
        if [ -f "$BACKUP_DATE_DIR/.env.production" ]; then
            cp "$BACKUP_DATE_DIR/.env.production" ./
        fi
        
        log_success "Configuration restored"
    else
        log_warning "No configuration backup found"
    fi
}

# Restore keys and certificates
restore_keys() {
    log_info "Restoring keys and certificates..."
    
    # Find backup directory
    BACKUP_DATE_DIR=$(find "$TEMP_DIR" -name "keys" -type d | head -1)
    
    if [ -n "$BACKUP_DATE_DIR" ]; then
        # Restore keys
        cp -r "$BACKUP_DATE_DIR/keys/" ./
        cp -r "$BACKUP_DATE_DIR/ssl/" ./deployments/ 2>/dev/null || true
        
        # Set proper permissions
        chmod 600 keys/* 2>/dev/null || true
        chmod 600 deployments/ssl/*.key 2>/dev/null || true
        
        log_success "Keys and certificates restored"
    else
        log_warning "No keys backup found"
    fi
}

# Restore logs
restore_logs() {
    log_info "Restoring logs..."
    
    # Find backup directory
    BACKUP_DATE_DIR=$(find "$TEMP_DIR" -name "logs" -type d | head -1)
    
    if [ -n "$BACKUP_DATE_DIR" ]; then
        # Restore logs
        cp -r "$BACKUP_DATE_DIR/logs/" ./
        
        log_success "Logs restored"
    else
        log_warning "No logs backup found"
    fi
}

# Start services
start_services() {
    log_info "Starting IPPAN services..."
    
    # Start Docker containers
    docker-compose -f deployments/production/docker-compose.production.yml up -d
    
    # Wait for services to be ready
    sleep 30
    
    # Check service health
    if curl -f http://localhost:3000/api/v1/status > /dev/null 2>&1; then
        log_success "Services started successfully"
    else
        log_error "Services failed to start properly"
        exit 1
    fi
}

# Cleanup temporary files
cleanup() {
    log_info "Cleaning up temporary files..."
    rm -rf "$TEMP_DIR"
    log_success "Cleanup completed"
}

# Verify restoration
verify_restoration() {
    log_info "Verifying restoration..."
    
    # Check database
    if [ -f "/data/ippan.db" ]; then
        log_success "Database file exists"
    else
        log_error "Database file not found"
        exit 1
    fi
    
    # Check configuration
    if [ -f "config/production.toml" ]; then
        log_success "Configuration file exists"
    else
        log_error "Configuration file not found"
        exit 1
    fi
    
    # Check API health
    if curl -f http://localhost:3000/api/v1/status > /dev/null 2>&1; then
        log_success "API is responding"
    else
        log_error "API is not responding"
        exit 1
    fi
    
    log_success "Restoration verification completed"
}

# Send restoration notification
send_notification() {
    if [ -n "$SLACK_WEBHOOK_URL" ]; then
        curl -X POST -H 'Content-type: application/json' \
        --data "{\"text\":\"✅ IPPAN restoration completed successfully from backup: $BACKUP_FILE\"}" \
        "$SLACK_WEBHOOK_URL"
    fi
    
    if [ -n "$EMAIL_RECIPIENTS" ]; then
        echo "IPPAN restoration completed successfully from backup: $BACKUP_FILE" | \
        mail -s "IPPAN Restoration Completed" "$EMAIL_RECIPIENTS"
    fi
}

# Main restore function
main() {
    log_info "Starting IPPAN restore process..."
    
    select_backup
    stop_services
    extract_backup
    restore_database
    restore_config
    restore_keys
    restore_logs
    start_services
    verify_restoration
    cleanup
    send_notification
    
    log_success "IPPAN restore process completed successfully!"
    echo ""
    echo "📁 Restoration Information:"
    echo "  - Backup File: $BACKUP_FILE"
    echo "  - Restoration Time: $(date)"
    echo "  - Services Status: Running"
    echo "  - API Endpoint: http://localhost:3000"
}

# Run main function
main "$@"
