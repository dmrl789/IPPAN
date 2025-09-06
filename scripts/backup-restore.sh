#!/bin/bash

# IPPAN Backup and Restore Script
# This script provides comprehensive backup and restore functionality for IPPAN

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BACKUP_DIR="${BACKUP_DIR:-$PROJECT_ROOT/backups}"
DATA_DIR="${DATA_DIR:-$PROJECT_ROOT/data}"
KEYS_DIR="${KEYS_DIR:-$PROJECT_ROOT/keys}"
CONFIG_DIR="${CONFIG_DIR:-$PROJECT_ROOT/config}"
LOG_DIR="${LOG_DIR:-$PROJECT_ROOT/logs}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
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
    if [[ ! -d "$BACKUP_DIR" ]]; then
        mkdir -p "$BACKUP_DIR"
        log_info "Created backup directory: $BACKUP_DIR"
    fi
}

# Generate backup filename
generate_backup_filename() {
    local backup_type="$1"
    local timestamp=$(date +%Y%m%d_%H%M%S)
    echo "${backup_type}_${timestamp}.tar.gz"
}

# Encrypt backup file
encrypt_backup() {
    local backup_file="$1"
    local encryption_key="${BACKUP_ENCRYPTION_KEY:-}"
    
    if [[ -n "$encryption_key" ]]; then
        log_info "Encrypting backup file..."
        openssl enc -aes-256-cbc -salt -in "$backup_file" \
            -out "${backup_file}.enc" \
            -k "$encryption_key"
        rm "$backup_file"
        echo "${backup_file}.enc"
    else
        log_warning "No encryption key provided - backup will not be encrypted"
        echo "$backup_file"
    fi
}

# Decrypt backup file
decrypt_backup() {
    local encrypted_file="$1"
    local encryption_key="${BACKUP_ENCRYPTION_KEY:-}"
    
    if [[ -n "$encryption_key" ]]; then
        log_info "Decrypting backup file..."
        local decrypted_file="${encrypted_file%.enc}"
        openssl enc -aes-256-cbc -d -in "$encrypted_file" \
            -out "$decrypted_file" \
            -k "$encryption_key"
        echo "$decrypted_file"
    else
        log_error "No encryption key provided - cannot decrypt backup"
        exit 1
    fi
}

# Full system backup
full_backup() {
    log_info "Starting full system backup..."
    
    create_backup_dir
    
    local backup_file="$BACKUP_DIR/$(generate_backup_filename "full_system")"
    
    # Create backup archive
    tar -czf "$backup_file" \
        -C "$PROJECT_ROOT" \
        --exclude="target" \
        --exclude="node_modules" \
        --exclude=".git" \
        --exclude="backups" \
        --exclude="*.log" \
        .
    
    # Encrypt if key provided
    backup_file=$(encrypt_backup "$backup_file")
    
    log_success "Full system backup completed: $backup_file"
    echo "$backup_file"
}

# Data-only backup
data_backup() {
    log_info "Starting data-only backup..."
    
    create_backup_dir
    
    local backup_file="$BACKUP_DIR/$(generate_backup_filename "data_only")"
    
    # Create data backup
    tar -czf "$backup_file" \
        -C "$DATA_DIR" . \
        -C "$KEYS_DIR" . \
        -C "$CONFIG_DIR" .
    
    # Encrypt if key provided
    backup_file=$(encrypt_backup "$backup_file")
    
    log_success "Data backup completed: $backup_file"
    echo "$backup_file"
}

# Incremental backup
incremental_backup() {
    log_info "Starting incremental backup..."
    
    create_backup_dir
    
    local backup_file="$BACKUP_DIR/$(generate_backup_filename "incremental")"
    local last_backup="$BACKUP_DIR/last_backup_timestamp"
    
    # Get last backup timestamp
    local since_date=""
    if [[ -f "$last_backup" ]]; then
        since_date="--newer-mtime=$(cat "$last_backup")"
    fi
    
    # Create incremental backup
    tar -czf "$backup_file" \
        $since_date \
        -C "$DATA_DIR" . \
        -C "$KEYS_DIR" . \
        -C "$CONFIG_DIR" .
    
    # Update timestamp
    date +%Y-%m-%d > "$last_backup"
    
    # Encrypt if key provided
    backup_file=$(encrypt_backup "$backup_file")
    
    log_success "Incremental backup completed: $backup_file"
    echo "$backup_file"
}

# Database backup
database_backup() {
    log_info "Starting database backup..."
    
    create_backup_dir
    
    local backup_file="$BACKUP_DIR/$(generate_backup_filename "database")"
    
    # Check if running in Docker
    if docker ps | grep -q ippan-node; then
        # Docker backup
        docker exec ippan-node ippan db backup /tmp/db_backup.sql
        docker cp ippan-node:/tmp/db_backup.sql "$backup_file"
        docker exec ippan-node rm /tmp/db_backup.sql
    else
        # Direct backup
        if command -v ippan &> /dev/null; then
            ippan db backup "$backup_file"
        else
            log_error "IPPAN binary not found and not running in Docker"
            exit 1
        fi
    fi
    
    # Encrypt if key provided
    backup_file=$(encrypt_backup "$backup_file")
    
    log_success "Database backup completed: $backup_file"
    echo "$backup_file"
}

# Restore from backup
restore_backup() {
    local backup_file="$1"
    local restore_type="${2:-full}"
    
    if [[ ! -f "$backup_file" ]]; then
        log_error "Backup file not found: $backup_file"
        exit 1
    fi
    
    log_info "Starting restore from: $backup_file"
    
    # Decrypt if encrypted
    if [[ "$backup_file" == *.enc ]]; then
        backup_file=$(decrypt_backup "$backup_file")
    fi
    
    # Stop services if running
    if docker ps | grep -q ippan-node; then
        log_info "Stopping IPPAN services..."
        docker-compose -f docker-compose.production.yml down
    fi
    
    case "$restore_type" in
        "full")
            log_info "Performing full system restore..."
            tar -xzf "$backup_file" -C "$PROJECT_ROOT"
            ;;
        "data")
            log_info "Performing data-only restore..."
            tar -xzf "$backup_file" -C "$PROJECT_ROOT"
            ;;
        "database")
            log_info "Performing database restore..."
            if docker ps | grep -q ippan-node; then
                docker cp "$backup_file" ippan-node:/tmp/db_restore.sql
                docker exec ippan-node ippan db restore /tmp/db_restore.sql
                docker exec ippan-node rm /tmp/db_restore.sql
            else
                if command -v ippan &> /dev/null; then
                    ippan db restore "$backup_file"
                else
                    log_error "IPPAN binary not found and not running in Docker"
                    exit 1
                fi
            fi
            ;;
        *)
            log_error "Invalid restore type: $restore_type"
            exit 1
            ;;
    esac
    
    # Clean up decrypted file
    if [[ "$backup_file" != "$1" ]]; then
        rm "$backup_file"
    fi
    
    log_success "Restore completed successfully"
}

# List available backups
list_backups() {
    log_info "Available backups:"
    
    if [[ ! -d "$BACKUP_DIR" ]]; then
        log_warning "No backup directory found"
        return
    fi
    
    local backups=($(ls -t "$BACKUP_DIR"/*.tar.gz* 2>/dev/null || true))
    
    if [[ ${#backups[@]} -eq 0 ]]; then
        log_warning "No backups found"
        return
    fi
    
    printf "%-50s %-20s %-10s\n" "Filename" "Date" "Size"
    printf "%-50s %-20s %-10s\n" "--------" "----" "----"
    
    for backup in "${backups[@]}"; do
        local filename=$(basename "$backup")
        local date=$(stat -c %y "$backup" 2>/dev/null | cut -d' ' -f1 || echo "Unknown")
        local size=$(du -h "$backup" 2>/dev/null | cut -f1 || echo "Unknown")
        printf "%-50s %-20s %-10s\n" "$filename" "$date" "$size"
    done
}

# Clean up old backups
cleanup_backups() {
    local retention_days="${1:-30}"
    
    log_info "Cleaning up backups older than $retention_days days..."
    
    if [[ ! -d "$BACKUP_DIR" ]]; then
        log_warning "No backup directory found"
        return
    fi
    
    local deleted_count=0
    while IFS= read -r -d '' backup; do
        rm "$backup"
        ((deleted_count++))
        log_info "Deleted: $(basename "$backup")"
    done < <(find "$BACKUP_DIR" -name "*.tar.gz*" -mtime +$retention_days -print0)
    
    log_success "Cleaned up $deleted_count old backups"
}

# Verify backup integrity
verify_backup() {
    local backup_file="$1"
    
    if [[ ! -f "$backup_file" ]]; then
        log_error "Backup file not found: $backup_file"
        exit 1
    fi
    
    log_info "Verifying backup integrity: $backup_file"
    
    # Decrypt if encrypted
    local temp_file="$backup_file"
    if [[ "$backup_file" == *.enc ]]; then
        temp_file=$(decrypt_backup "$backup_file")
    fi
    
    # Test archive integrity
    if tar -tzf "$temp_file" >/dev/null 2>&1; then
        log_success "Backup integrity verified"
    else
        log_error "Backup integrity check failed"
        exit 1
    fi
    
    # Clean up temp file
    if [[ "$temp_file" != "$backup_file" ]]; then
        rm "$temp_file"
    fi
}

# Schedule backup
schedule_backup() {
    local backup_type="${1:-incremental}"
    local schedule="${2:-0 2 * * *}"  # Daily at 2 AM
    
    log_info "Scheduling $backup_type backup: $schedule"
    
    # Create cron job
    local cron_job="$schedule $SCRIPT_DIR/backup-restore.sh $backup_type"
    
    # Add to crontab
    (crontab -l 2>/dev/null; echo "$cron_job") | crontab -
    
    log_success "Backup scheduled successfully"
}

# Show usage
show_usage() {
    cat << EOF
IPPAN Backup and Restore Script

Usage: $0 <command> [options]

Commands:
    full                    Create full system backup
    data                    Create data-only backup
    incremental             Create incremental backup
    database                Create database backup
    restore <file> [type]   Restore from backup (full|data|database)
    list                    List available backups
    cleanup [days]          Clean up old backups (default: 30 days)
    verify <file>           Verify backup integrity
    schedule <type> [cron]  Schedule automatic backups
    help                    Show this help message

Options:
    BACKUP_DIR              Backup directory (default: ./backups)
    DATA_DIR                Data directory (default: ./data)
    KEYS_DIR                Keys directory (default: ./keys)
    CONFIG_DIR              Config directory (default: ./config)
    BACKUP_ENCRYPTION_KEY   Encryption key for backups

Examples:
    $0 full                                    # Create full backup
    $0 data                                    # Create data backup
    $0 restore backups/data_only_20231201.tar.gz data  # Restore data
    $0 list                                    # List backups
    $0 cleanup 7                               # Clean up backups older than 7 days
    $0 schedule incremental "0 2 * * *"        # Schedule daily incremental backups

EOF
}

# Main function
main() {
    local command="${1:-help}"
    
    case "$command" in
        "full")
            full_backup
            ;;
        "data")
            data_backup
            ;;
        "incremental")
            incremental_backup
            ;;
        "database")
            database_backup
            ;;
        "restore")
            if [[ $# -lt 2 ]]; then
                log_error "Restore command requires backup file"
                show_usage
                exit 1
            fi
            restore_backup "$2" "${3:-full}"
            ;;
        "list")
            list_backups
            ;;
        "cleanup")
            cleanup_backups "${2:-30}"
            ;;
        "verify")
            if [[ $# -lt 2 ]]; then
                log_error "Verify command requires backup file"
                show_usage
                exit 1
            fi
            verify_backup "$2"
            ;;
        "schedule")
            schedule_backup "${2:-incremental}" "${3:-0 2 * * *}"
            ;;
        "help"|"--help"|"-h")
            show_usage
            ;;
        *)
            log_error "Unknown command: $command"
            show_usage
            exit 1
            ;;
    esac
}

# Run main function
main "$@"
