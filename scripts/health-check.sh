#!/bin/bash

# IPPAN Health Check Script
# This script performs comprehensive health checks on the IPPAN system

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
HEALTH_CHECK_TIMEOUT="${HEALTH_CHECK_TIMEOUT:-30}"
VERBOSE="${VERBOSE:-false}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Health check results
declare -A HEALTH_RESULTS
TOTAL_CHECKS=0
PASSED_CHECKS=0
FAILED_CHECKS=0
WARNING_CHECKS=0

# Logging functions
log_info() {
    if [[ "$VERBOSE" == "true" ]]; then
        echo -e "${BLUE}[INFO]${NC} $1"
    fi
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[FAIL]${NC} $1"
}

# Record health check result
record_result() {
    local check_name="$1"
    local status="$2"
    local message="$3"
    
    HEALTH_RESULTS["$check_name"]="$status|$message"
    ((TOTAL_CHECKS++))
    
    case "$status" in
        "PASS")
            ((PASSED_CHECKS++))
            log_success "$check_name: $message"
            ;;
        "WARN")
            ((WARNING_CHECKS++))
            log_warning "$check_name: $message"
            ;;
        "FAIL")
            ((FAILED_CHECKS++))
            log_error "$check_name: $message"
            ;;
    esac
}

# Check if service is running
check_service_running() {
    local service_name="$1"
    local container_name="$2"
    
    if docker ps | grep -q "$container_name"; then
        record_result "$service_name" "PASS" "Service is running"
    else
        record_result "$service_name" "FAIL" "Service is not running"
    fi
}

# Check HTTP endpoint
check_http_endpoint() {
    local name="$1"
    local url="$2"
    local expected_status="${3:-200}"
    
    log_info "Checking HTTP endpoint: $url"
    
    local response
    if response=$(curl -s -o /dev/null -w "%{http_code}" --max-time "$HEALTH_CHECK_TIMEOUT" "$url" 2>/dev/null); then
        if [[ "$response" == "$expected_status" ]]; then
            record_result "$name" "PASS" "HTTP $response - Endpoint is healthy"
        else
            record_result "$name" "WARN" "HTTP $response - Expected $expected_status"
        fi
    else
        record_result "$name" "FAIL" "Endpoint is not accessible"
    fi
}

# Check port availability
check_port() {
    local name="$1"
    local host="$2"
    local port="$3"
    
    log_info "Checking port: $host:$port"
    
    if nc -z "$host" "$port" 2>/dev/null; then
        record_result "$name" "PASS" "Port $port is open"
    else
        record_result "$name" "FAIL" "Port $port is not accessible"
    fi
}

# Check disk space
check_disk_space() {
    local name="$1"
    local path="$2"
    local threshold="${3:-90}"
    
    log_info "Checking disk space: $path"
    
    local usage
    if usage=$(df "$path" 2>/dev/null | awk 'NR==2 {print $5}' | sed 's/%//'); then
        if [[ "$usage" -lt "$threshold" ]]; then
            record_result "$name" "PASS" "Disk usage: ${usage}% (threshold: ${threshold}%)"
        else
            record_result "$name" "WARN" "Disk usage: ${usage}% (threshold: ${threshold}%)"
        fi
    else
        record_result "$name" "FAIL" "Cannot check disk space"
    fi
}

# Check memory usage
check_memory_usage() {
    local name="$1"
    local container_name="$2"
    local threshold="${3:-80}"
    
    log_info "Checking memory usage: $container_name"
    
    local usage
    if usage=$(docker stats --no-stream --format "{{.MemPerc}}" "$container_name" 2>/dev/null | sed 's/%//'); then
        if [[ "$usage" -lt "$threshold" ]]; then
            record_result "$name" "PASS" "Memory usage: ${usage}% (threshold: ${threshold}%)"
        else
            record_result "$name" "WARN" "Memory usage: ${usage}% (threshold: ${threshold}%)"
        fi
    else
        record_result "$name" "FAIL" "Cannot check memory usage"
    fi
}

# Check CPU usage
check_cpu_usage() {
    local name="$1"
    local container_name="$2"
    local threshold="${3:-80}"
    
    log_info "Checking CPU usage: $container_name"
    
    local usage
    if usage=$(docker stats --no-stream --format "{{.CPUPerc}}" "$container_name" 2>/dev/null | sed 's/%//'); then
        if [[ "$usage" -lt "$threshold" ]]; then
            record_result "$name" "PASS" "CPU usage: ${usage}% (threshold: ${threshold}%)"
        else
            record_result "$name" "WARN" "CPU usage: ${usage}% (threshold: ${threshold}%)"
        fi
    else
        record_result "$name" "FAIL" "Cannot check CPU usage"
    fi
}

# Check log errors
check_log_errors() {
    local name="$1"
    local container_name="$2"
    local log_level="${3:-ERROR}"
    
    log_info "Checking log errors: $container_name"
    
    local error_count
    if error_count=$(docker logs "$container_name" --since 1h 2>&1 | grep -i "$log_level" | wc -l); then
        if [[ "$error_count" -eq 0 ]]; then
            record_result "$name" "PASS" "No $log_level messages in last hour"
        else
            record_result "$name" "WARN" "$error_count $log_level messages in last hour"
        fi
    else
        record_result "$name" "FAIL" "Cannot check logs"
    fi
}

# Check database connectivity
check_database() {
    local name="$1"
    local container_name="$2"
    
    log_info "Checking database connectivity: $container_name"
    
    if docker exec "$container_name" ippan db status 2>/dev/null | grep -q "healthy"; then
        record_result "$name" "PASS" "Database is healthy"
    else
        record_result "$name" "FAIL" "Database is not healthy"
    fi
}

# Check network connectivity
check_network_connectivity() {
    local name="$1"
    local container_name="$2"
    
    log_info "Checking network connectivity: $container_name"
    
    if docker exec "$container_name" ippan network status 2>/dev/null | grep -q "connected"; then
        record_result "$name" "PASS" "Network is connected"
    else
        record_result "$name" "FAIL" "Network is not connected"
    fi
}

# Check consensus status
check_consensus_status() {
    local name="$1"
    local container_name="$2"
    
    log_info "Checking consensus status: $container_name"
    
    if docker exec "$container_name" ippan consensus status 2>/dev/null | grep -q "active"; then
        record_result "$name" "PASS" "Consensus is active"
    else
        record_result "$name" "FAIL" "Consensus is not active"
    fi
}

# Check SSL certificate
check_ssl_certificate() {
    local name="$1"
    local cert_path="$2"
    
    log_info "Checking SSL certificate: $cert_path"
    
    if [[ -f "$cert_path" ]]; then
        local expiry_date
        if expiry_date=$(openssl x509 -enddate -noout -in "$cert_path" 2>/dev/null | cut -d= -f2); then
            local expiry_timestamp
            expiry_timestamp=$(date -d "$expiry_date" +%s 2>/dev/null || echo "0")
            local current_timestamp
            current_timestamp=$(date +%s)
            local days_until_expiry
            days_until_expiry=$(( (expiry_timestamp - current_timestamp) / 86400 ))
            
            if [[ "$days_until_expiry" -gt 30 ]]; then
                record_result "$name" "PASS" "Certificate expires in $days_until_expiry days"
            elif [[ "$days_until_expiry" -gt 0 ]]; then
                record_result "$name" "WARN" "Certificate expires in $days_until_expiry days"
            else
                record_result "$name" "FAIL" "Certificate has expired"
            fi
        else
            record_result "$name" "FAIL" "Cannot read certificate"
        fi
    else
        record_result "$name" "FAIL" "Certificate file not found"
    fi
}

# Check backup status
check_backup_status() {
    local name="$1"
    local backup_dir="$2"
    
    log_info "Checking backup status: $backup_dir"
    
    if [[ -d "$backup_dir" ]]; then
        local latest_backup
        latest_backup=$(find "$backup_dir" -name "*.tar.gz*" -type f -printf '%T@ %p\n' 2>/dev/null | sort -n | tail -1 | cut -d' ' -f2-)
        
        if [[ -n "$latest_backup" ]]; then
            local backup_age
            backup_age=$(($(date +%s) - $(stat -c %Y "$latest_backup" 2>/dev/null || echo "0")))
            local backup_age_hours
            backup_age_hours=$((backup_age / 3600))
            
            if [[ "$backup_age_hours" -lt 25 ]]; then
                record_result "$name" "PASS" "Latest backup is $backup_age_hours hours old"
            else
                record_result "$name" "WARN" "Latest backup is $backup_age_hours hours old"
            fi
        else
            record_result "$name" "WARN" "No backups found"
        fi
    else
        record_result "$name" "FAIL" "Backup directory not found"
    fi
}

# Check configuration
check_configuration() {
    local name="$1"
    local config_path="$2"
    
    log_info "Checking configuration: $config_path"
    
    if [[ -f "$config_path" ]]; then
        if docker exec ippan-node ippan config validate 2>/dev/null | grep -q "valid"; then
            record_result "$name" "PASS" "Configuration is valid"
        else
            record_result "$name" "FAIL" "Configuration is invalid"
        fi
    else
        record_result "$name" "FAIL" "Configuration file not found"
    fi
}

# Check metrics endpoint
check_metrics_endpoint() {
    local name="$1"
    local url="$2"
    
    log_info "Checking metrics endpoint: $url"
    
    local metrics
    if metrics=$(curl -s --max-time "$HEALTH_CHECK_TIMEOUT" "$url" 2>/dev/null); then
        if echo "$metrics" | grep -q "ippan_"; then
            record_result "$name" "PASS" "Metrics endpoint is working"
        else
            record_result "$name" "WARN" "Metrics endpoint accessible but no IPPAN metrics found"
        fi
    else
        record_result "$name" "FAIL" "Metrics endpoint is not accessible"
    fi
}

# Perform comprehensive health check
comprehensive_health_check() {
    log_info "Starting comprehensive health check..."
    
    # Check Docker services
    check_service_running "IPPAN Node" "ippan-node"
    check_service_running "Prometheus" "ippan-prometheus"
    check_service_running "Grafana" "ippan-grafana"
    check_service_running "Alertmanager" "ippan-alertmanager"
    check_service_running "Redis" "ippan-redis"
    
    # Check HTTP endpoints
    check_http_endpoint "IPPAN Frontend" "http://localhost:80/health"
    check_http_endpoint "IPPAN API" "http://localhost:3000/api/v1/status"
    check_http_endpoint "Prometheus" "http://localhost:9090/-/healthy"
    check_http_endpoint "Grafana" "http://localhost:3001/api/health"
    check_http_endpoint "Alertmanager" "http://localhost:9093/-/healthy"
    
    # Check ports
    check_port "IPPAN P2P Port" "localhost" "8080"
    check_port "IPPAN API Port" "localhost" "3000"
    check_port "Prometheus Port" "localhost" "9090"
    check_port "Grafana Port" "localhost" "3001"
    check_port "Redis Port" "localhost" "6379"
    
    # Check system resources
    check_disk_space "Data Directory" "$PROJECT_ROOT/data" "90"
    check_disk_space "Backup Directory" "$PROJECT_ROOT/backups" "95"
    check_memory_usage "IPPAN Node Memory" "ippan-node" "80"
    check_cpu_usage "IPPAN Node CPU" "ippan-node" "80"
    
    # Check logs
    check_log_errors "IPPAN Node Logs" "ippan-node" "ERROR"
    
    # Check IPPAN-specific components
    check_database "IPPAN Database" "ippan-node"
    check_network_connectivity "IPPAN Network" "ippan-node"
    check_consensus_status "IPPAN Consensus" "ippan-node"
    
    # Check SSL certificate
    check_ssl_certificate "SSL Certificate" "$PROJECT_ROOT/ssl/ippan.crt"
    
    # Check backup status
    check_backup_status "Backup Status" "$PROJECT_ROOT/backups"
    
    # Check configuration
    check_configuration "IPPAN Configuration" "$PROJECT_ROOT/config/production.toml"
    
    # Check metrics
    check_metrics_endpoint "IPPAN Metrics" "http://localhost:8080/metrics"
}

# Generate health report
generate_health_report() {
    echo
    echo "=========================================="
    echo "           IPPAN HEALTH REPORT"
    echo "=========================================="
    echo
    
    printf "%-30s %-10s %s\n" "Check" "Status" "Message"
    printf "%-30s %-10s %s\n" "-----" "------" "-------"
    
    for check_name in "${!HEALTH_RESULTS[@]}"; do
        local result="${HEALTH_RESULTS[$check_name]}"
        local status="${result%%|*}"
        local message="${result#*|}"
        
        case "$status" in
            "PASS")
                printf "%-30s %-10s %s\n" "$check_name" "PASS" "$message"
                ;;
            "WARN")
                printf "%-30s %-10s %s\n" "$check_name" "WARN" "$message"
                ;;
            "FAIL")
                printf "%-30s %-10s %s\n" "$check_name" "FAIL" "$message"
                ;;
        esac
    done
    
    echo
    echo "=========================================="
    printf "Total Checks: %d\n" "$TOTAL_CHECKS"
    printf "Passed: %d\n" "$PASSED_CHECKS"
    printf "Warnings: %d\n" "$WARNING_CHECKS"
    printf "Failed: %d\n" "$FAILED_CHECKS"
    echo "=========================================="
    
    # Overall health status
    if [[ "$FAILED_CHECKS" -eq 0 ]]; then
        if [[ "$WARNING_CHECKS" -eq 0 ]]; then
            echo -e "${GREEN}Overall Status: HEALTHY${NC}"
            exit 0
        else
            echo -e "${YELLOW}Overall Status: HEALTHY WITH WARNINGS${NC}"
            exit 0
        fi
    else
        echo -e "${RED}Overall Status: UNHEALTHY${NC}"
        exit 1
    fi
}

# Show usage
show_usage() {
    cat << EOF
IPPAN Health Check Script

Usage: $0 [options]

Options:
    -v, --verbose    Enable verbose output
    -t, --timeout    Set timeout for HTTP checks (default: 30)
    -h, --help       Show this help message

Examples:
    $0                    # Run comprehensive health check
    $0 -v                 # Run with verbose output
    $0 -t 60              # Run with 60 second timeout

EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -v|--verbose)
            VERBOSE="true"
            shift
            ;;
        -t|--timeout)
            HEALTH_CHECK_TIMEOUT="$2"
            shift 2
            ;;
        -h|--help)
            show_usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
done

# Main execution
main() {
    comprehensive_health_check
    generate_health_report
}

# Run main function
main
