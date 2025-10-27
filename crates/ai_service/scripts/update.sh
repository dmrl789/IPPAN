#!/bin/bash

# Production update script for IPPAN AI Service

set -euo pipefail

# Configuration
SERVICE_NAME="ippan-ai-service"
SERVICE_DIR="/opt/ippan-ai-service"
BACKUP_DIR="/opt/backups/ippan-ai-service"
NEW_VERSION=${1:-"latest"}

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    local status=$1
    local message=$2
    case $status in
        "INFO")
            echo -e "${BLUE}ℹ️  $message${NC}"
            ;;
        "OK")
            echo -e "${GREEN}✅ $message${NC}"
            ;;
        "WARN")
            echo -e "${YELLOW}⚠️  $message${NC}"
            ;;
        "ERROR")
            echo -e "${RED}❌ $message${NC}"
            ;;
    esac
}

# Function to check if service is running
is_service_running() {
    systemctl is-active --quiet ippan-ai-service
}

# Function to wait for service to be ready
wait_for_service() {
    local max_attempts=30
    local attempt=1
    
    print_status "INFO" "Waiting for service to be ready..."
    
    while [ $attempt -le $max_attempts ]; do
        if curl -s --max-time 5 http://localhost:8080/health > /dev/null 2>&1; then
            print_status "OK" "Service is ready"
            return 0
        fi
        
        print_status "INFO" "Attempt $attempt/$max_attempts - Service not ready yet..."
        sleep 2
        ((attempt++))
    done
    
    print_status "ERROR" "Service failed to become ready within timeout"
    return 1
}

# Function to create backup
create_backup() {
    print_status "INFO" "Creating backup before update..."
    
    local timestamp=$(date +"%Y%m%d_%H%M%S")
    local backup_file="$BACKUP_DIR/${SERVICE_NAME}_backup_$timestamp.tar.gz"
    
    mkdir -p "$BACKUP_DIR"
    
    if tar -czf "$backup_file" -C "$SERVICE_DIR" config/ data/ logs/ 2>/dev/null; then
        print_status "OK" "Backup created: $backup_file"
        echo "$backup_file"
    else
        print_status "WARN" "Failed to create backup, continuing with update..."
        echo ""
    fi
}

# Function to download new version
download_new_version() {
    print_status "INFO" "Downloading new version: $NEW_VERSION"
    
    # This would be replaced with actual download logic
    # For now, we'll assume the new binary is already available
    if [ -f "/tmp/ippan-ai-service-$NEW_VERSION" ]; then
        print_status "OK" "New version downloaded"
        echo "/tmp/ippan-ai-service-$NEW_VERSION"
    else
        print_status "ERROR" "New version not found: /tmp/ippan-ai-service-$NEW_VERSION"
        return 1
    fi
}

# Function to update service
update_service() {
    local new_binary=$1
    
    print_status "INFO" "Updating service binary..."
    
    # Stop the service
    if is_service_running; then
        print_status "INFO" "Stopping service..."
        systemctl stop ippan-ai-service
    fi
    
    # Backup current binary
    if [ -f "/usr/local/bin/ippan-ai-service" ]; then
        cp "/usr/local/bin/ippan-ai-service" "/usr/local/bin/ippan-ai-service.backup"
    fi
    
    # Install new binary
    cp "$new_binary" "/usr/local/bin/ippan-ai-service"
    chmod +x "/usr/local/bin/ippan-ai-service"
    
    print_status "OK" "Service binary updated"
}

# Function to start service
start_service() {
    print_status "INFO" "Starting service..."
    
    if systemctl start ippan-ai-service; then
        print_status "OK" "Service started"
    else
        print_status "ERROR" "Failed to start service"
        return 1
    fi
}

# Function to rollback
rollback() {
    local backup_file=$1
    
    print_status "WARN" "Rolling back to previous version..."
    
    # Stop service
    systemctl stop ippan-ai-service || true
    
    # Restore previous binary
    if [ -f "/usr/local/bin/ippan-ai-service.backup" ]; then
        cp "/usr/local/bin/ippan-ai-service.backup" "/usr/local/bin/ippan-ai-service"
        print_status "OK" "Binary rolled back"
    fi
    
    # Restore configuration if backup exists
    if [ -n "$backup_file" ] && [ -f "$backup_file" ]; then
        tar -xzf "$backup_file" -C "$SERVICE_DIR"
        print_status "OK" "Configuration rolled back"
    fi
    
    # Start service
    if systemctl start ippan-ai-service; then
        print_status "OK" "Service rolled back and started"
    else
        print_status "ERROR" "Failed to start service after rollback"
        return 1
    fi
}

# Main update process
echo "🔄 IPPAN AI Service Update"
echo "=========================="
echo "Time: $(date)"
echo "New Version: $NEW_VERSION"
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    print_status "ERROR" "Please run as root"
    exit 1
fi

# Check if service exists
if ! systemctl list-unit-files | grep -q ippan-ai-service; then
    print_status "ERROR" "Service not found"
    exit 1
fi

# Create backup
backup_file=$(create_backup)

# Download new version
new_binary=$(download_new_version)
if [ $? -ne 0 ]; then
    print_status "ERROR" "Failed to download new version"
    exit 1
fi

# Update service
update_service "$new_binary"

# Start service
if start_service; then
    # Wait for service to be ready
    if wait_for_service; then
        print_status "OK" "Update completed successfully"
        
        # Clean up
        rm -f "$new_binary"
        rm -f "/usr/local/bin/ippan-ai-service.backup"
        
        echo ""
        echo "🎉 Update completed successfully at $(date)"
    else
        print_status "ERROR" "Service failed to become ready after update"
        rollback "$backup_file"
        exit 1
    fi
else
    print_status "ERROR" "Failed to start service after update"
    rollback "$backup_file"
    exit 1
fi