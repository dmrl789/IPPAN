#!/bin/bash

# IPPAN Production Monitoring Script
# Continuously monitors the health of IPPAN nodes and alerts on issues

set -e

# Configuration
NODE1_URL="http://188.245.97.41:8080"
NODE2_URL="http://135.181.145.174:8080"
CHECK_INTERVAL=30
LOG_FILE="/var/log/ippan/monitor.log"
ALERT_EMAIL="admin@ippan.org"

# Create log directory if it doesn't exist
mkdir -p "$(dirname "$LOG_FILE")"

# Function to log messages
log_message() {
    local level=$1
    local message=$2
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[$timestamp] [$level] $message" | tee -a "$LOG_FILE"
}

# Function to check node health
check_node() {
    local url=$1
    local node_name=$2
    
    local response=$(curl -s --connect-timeout 10 "$url/health" 2>/dev/null)
    if [ $? -eq 0 ]; then
        local status=$(echo "$response" | jq -r '.status // "unknown"' 2>/dev/null || echo "unknown")
        local peer_count=$(echo "$response" | jq -r '.peer_count // 0' 2>/dev/null || echo "0")
        local mempool_size=$(echo "$response" | jq -r '.mempool_size // 0' 2>/dev/null || echo "0")
        local uptime=$(echo "$response" | jq -r '.uptime_secs // 0' 2>/dev/null || echo "0")
        
        if [ "$status" = "ok" ]; then
            log_message "INFO" "$node_name: Healthy (peers: $peer_count, mempool: $mempool_size, uptime: ${uptime}s)"
            return 0
        else
            log_message "ERROR" "$node_name: Unhealthy (status: $status)"
            return 1
        fi
    else
        log_message "ERROR" "$node_name: Unreachable"
        return 1
    fi
}

# Function to check consensus
check_consensus() {
    local node1_health=$(curl -s --connect-timeout 10 "${NODE1_URL}/health" 2>/dev/null)
    local node2_health=$(curl -s --connect-timeout 10 "${NODE2_URL}/health" 2>/dev/null)
    
    if [ $? -eq 0 ] && echo "$node1_health" | jq -e '.consensus' >/dev/null 2>&1; then
        local node1_height=$(echo "$node1_health" | jq -r '.consensus.latest_block_height // 0' 2>/dev/null || echo "0")
        local node2_height=$(echo "$node2_health" | jq -r '.consensus.latest_block_height // 0' 2>/dev/null || echo "0")
        
        local height_diff=$((node1_height - node2_height))
        if [ ${height_diff#-} -le 5 ]; then
            log_message "INFO" "Consensus: Nodes synchronized (heights: $node1_height, $node2_height)"
        else
            log_message "WARN" "Consensus: Nodes out of sync (height diff: $height_diff)"
        fi
    else
        log_message "ERROR" "Consensus: Failed to get consensus information"
    fi
}

# Function to send alert
send_alert() {
    local message=$1
    log_message "ALERT" "$message"
    
    # In a production environment, you would send actual alerts here
    # For example: email, Slack, PagerDuty, etc.
    echo "ALERT: $message" > /tmp/ippan_alert.txt
}

# Function to perform health check
perform_health_check() {
    local node1_healthy=false
    local node2_healthy=false
    
    # Check Node 1
    if check_node "$NODE1_URL" "Node 1"; then
        node1_healthy=true
    else
        send_alert "Node 1 (188.245.97.41) is unhealthy or unreachable"
    fi
    
    # Check Node 2
    if check_node "$NODE2_URL" "Node 2"; then
        node2_healthy=true
    else
        send_alert "Node 2 (135.181.145.174) is unhealthy or unreachable"
    fi
    
    # Check consensus if both nodes are healthy
    if [ "$node1_healthy" = true ] && [ "$node2_healthy" = true ]; then
        check_consensus
    fi
    
    # Check if both nodes are down
    if [ "$node1_healthy" = false ] && [ "$node2_healthy" = false ]; then
        send_alert "CRITICAL: Both nodes are down"
    fi
}

# Function to cleanup on exit
cleanup() {
    log_message "INFO" "Monitoring stopped"
    exit 0
}

# Set up signal handlers
trap cleanup SIGINT SIGTERM

# Main monitoring loop
main() {
    log_message "INFO" "Starting IPPAN production monitoring"
    
    while true; do
        perform_health_check
        sleep "$CHECK_INTERVAL"
    done
}

# Run main function
main "$@"