#!/bin/bash

# Production health check script for IPPAN AI Service

set -euo pipefail

# Configuration
SERVICE_URL="http://localhost:8080"
HEALTH_ENDPOINT="$SERVICE_URL/health"
METRICS_ENDPOINT="$SERVICE_URL/metrics"
TIMEOUT=10
RETRIES=3

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    local status=$1
    local message=$2
    case $status in
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

# Function to check endpoint
check_endpoint() {
    local url=$1
    local name=$2
    local expected_status=${3:-200}
    
    for i in $(seq 1 $RETRIES); do
        if response=$(curl -s -w "%{http_code}" -o /dev/null --max-time $TIMEOUT "$url" 2>/dev/null); then
            if [ "$response" = "$expected_status" ]; then
                print_status "OK" "$name is healthy (HTTP $response)"
                return 0
            else
                print_status "WARN" "$name returned HTTP $response (expected $expected_status)"
            fi
        else
            print_status "WARN" "$name check failed (attempt $i/$RETRIES)"
        fi
        
        if [ $i -lt $RETRIES ]; then
            sleep 2
        fi
    done
    
    print_status "ERROR" "$name is not responding"
    return 1
}

# Function to check service status
check_service_status() {
    local status=$1
    case $status in
        "running")
            print_status "OK" "Service is running"
            ;;
        "stopped")
            print_status "ERROR" "Service is stopped"
            ;;
        "failed")
            print_status "ERROR" "Service has failed"
            ;;
        *)
            print_status "WARN" "Service status is unknown: $status"
            ;;
    esac
}

# Function to check system resources
check_system_resources() {
    # Check memory usage
    local memory_usage=$(free | awk 'NR==2{printf "%.0f", $3*100/$2}')
    if [ "$memory_usage" -gt 90 ]; then
        print_status "ERROR" "High memory usage: ${memory_usage}%"
    elif [ "$memory_usage" -gt 80 ]; then
        print_status "WARN" "Memory usage is high: ${memory_usage}%"
    else
        print_status "OK" "Memory usage is normal: ${memory_usage}%"
    fi
    
    # Check disk usage
    local disk_usage=$(df / | awk 'NR==2{print $5}' | sed 's/%//')
    if [ "$disk_usage" -gt 90 ]; then
        print_status "ERROR" "High disk usage: ${disk_usage}%"
    elif [ "$disk_usage" -gt 80 ]; then
        print_status "WARN" "Disk usage is high: ${disk_usage}%"
    else
        print_status "OK" "Disk usage is normal: ${disk_usage}%"
    fi
}

# Main health check
echo "🏥 IPPAN AI Service Health Check"
echo "================================="
echo "Time: $(date)"
echo ""

# Check if service is running
if systemctl is-active --quiet ippan-ai-service; then
    service_status="running"
else
    service_status="stopped"
fi

check_service_status "$service_status"

# Check system resources
echo ""
echo "💻 System Resources:"
check_system_resources

# Check endpoints
echo ""
echo "🌐 Service Endpoints:"
check_endpoint "$HEALTH_ENDPOINT" "Health Check"
check_endpoint "$METRICS_ENDPOINT" "Metrics Endpoint"

# Check detailed health if service is running
if [ "$service_status" = "running" ]; then
    echo ""
    echo "🔍 Detailed Health Status:"
    
    # Get health status
    if health_response=$(curl -s --max-time $TIMEOUT "$HEALTH_ENDPOINT" 2>/dev/null); then
        echo "Health Response:"
        echo "$health_response" | jq '.' 2>/dev/null || echo "$health_response"
    else
        print_status "ERROR" "Failed to get detailed health status"
    fi
fi

# Check logs for errors
echo ""
echo "📋 Recent Logs:"
if journalctl -u ippan-ai-service --no-pager -n 10 --since "5 minutes ago" | grep -i error > /dev/null; then
    print_status "WARN" "Errors found in recent logs"
    echo "Recent errors:"
    journalctl -u ippan-ai-service --no-pager -n 20 --since "5 minutes ago" | grep -i error | head -5
else
    print_status "OK" "No errors in recent logs"
fi

echo ""
echo "🏁 Health check completed at $(date)"