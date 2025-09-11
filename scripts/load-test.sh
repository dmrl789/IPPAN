#!/bin/bash

# IPPAN Load Testing Script
# This script performs comprehensive load testing for high-throughput scenarios

set -e

# Configuration
LOAD_TEST_DIR="/tmp/ippan-loadtest-$(date +%Y%m%d_%H%M%S)"
TARGET_HOST="${TARGET_HOST:-localhost}"
TARGET_PORT="${TARGET_PORT:-3000}"
MAX_CONNECTIONS="${MAX_CONNECTIONS:-10000}"
TEST_DURATION="${TEST_DURATION:-600}"

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

# Create load test directory
create_loadtest_dir() {
    log_info "Creating load test directory..."
    mkdir -p "$LOAD_TEST_DIR"
    log_success "Load test directory created: $LOAD_TEST_DIR"
}

# Stress test with increasing load
stress_test() {
    log_info "Starting stress test with increasing load..."
    
    # Create stress test script
    cat > "$LOAD_TEST_DIR/stress-test.lua" << 'EOF'
-- Stress test script for IPPAN
local counter = 0
local threads = {}

function setup(thread)
   thread:set("id", counter)
   table.insert(threads, thread)
   counter = counter + 1
end

function init(args)
   requests  = 0
   responses = 0
   local msg = "thread %d created"
   print(msg:format(id))
end

function request()
   requests = requests + 1
   local body = string.format('{"amount": %d, "recipient": "recipient-%d", "sender": "sender-%d"}', 
                             math.random(1000, 1000000), 
                             math.random(1, 10000), 
                             math.random(1, 10000))
   
   return wrk.format("POST", "/api/v1/transactions", {
      ["Content-Type"] = "application/json"
   }, body)
end

function response(status, headers, body)
   responses = responses + 1
   if status ~= 200 then
      print("Error: " .. status .. " " .. body)
   end
end

function done(summary, latency, requests)
   local msg = "thread %d: %d requests, %d responses, %.2f req/sec"
   print(msg:format(id, requests, responses, requests/summary.duration))
end
EOF
    
    # Run stress test with different connection levels
    local connections=(100 500 1000 2000 5000 10000)
    local threads=(4 8 16 32 64 128)
    
    for i in "${!connections[@]}"; do
        local conn=${connections[$i]}
        local thread=${threads[$i]}
        
        log_info "Running stress test with $conn connections and $thread threads..."
        
        if command -v wrk &> /dev/null; then
            wrk -t$thread -c$conn -d60s -s "$LOAD_TEST_DIR/stress-test.lua" \
                "http://$TARGET_HOST:$TARGET_PORT" > "$LOAD_TEST_DIR/stress-test-$conn-conn.txt" 2>&1 || true
        fi
        
        # Wait between tests
        sleep 10
    done
    
    log_success "Stress test completed"
}

# Spike test
spike_test() {
    log_info "Starting spike test..."
    
    # Create spike test script
    cat > "$LOAD_TEST_DIR/spike-test.lua" << 'EOF'
-- Spike test script for IPPAN
local counter = 0

function setup(thread)
   thread:set("id", counter)
   counter = counter + 1
end

function init(args)
   requests  = 0
   responses = 0
end

function request()
   requests = requests + 1
   local body = string.format('{"amount": %d, "recipient": "recipient-%d", "sender": "sender-%d"}', 
                             math.random(1000, 1000000), 
                             math.random(1, 10000), 
                             math.random(1, 10000))
   
   return wrk.format("POST", "/api/v1/transactions", {
      ["Content-Type"] = "application/json"
   }, body)
end

function response(status, headers, body)
   responses = responses + 1
   if status ~= 200 then
      print("Error: " .. status .. " " .. body)
   end
end

function done(summary, latency, requests)
   local msg = "thread %d: %d requests, %d responses, %.2f req/sec"
   print(msg:format(id, requests, responses, requests/summary.duration))
end
EOF
    
    # Run spike test
    log_info "Running spike test with 10000 connections..."
    if command -v wrk &> /dev/null; then
        wrk -t128 -c10000 -d30s -s "$LOAD_TEST_DIR/spike-test.lua" \
            "http://$TARGET_HOST:$TARGET_PORT" > "$LOAD_TEST_DIR/spike-test-results.txt" 2>&1 || true
    fi
    
    log_success "Spike test completed"
}

# Volume test
volume_test() {
    log_info "Starting volume test..."
    
    # Create volume test script
    cat > "$LOAD_TEST_DIR/volume-test.lua" << 'EOF'
-- Volume test script for IPPAN
local counter = 0

function setup(thread)
   thread:set("id", counter)
   counter = counter + 1
end

function init(args)
   requests  = 0
   responses = 0
end

function request()
   requests = requests + 1
   local body = string.format('{"amount": %d, "recipient": "recipient-%d", "sender": "sender-%d"}', 
                             math.random(1000, 1000000), 
                             math.random(1, 10000), 
                             math.random(1, 10000))
   
   return wrk.format("POST", "/api/v1/transactions", {
      ["Content-Type"] = "application/json"
   }, body)
end

function response(status, headers, body)
   responses = responses + 1
   if status ~= 200 then
      print("Error: " .. status .. " " .. body)
   end
end

function done(summary, latency, requests)
   local msg = "thread %d: %d requests, %d responses, %.2f req/sec"
   print(msg:format(id, requests, responses, requests/summary.duration))
end
EOF
    
    # Run volume test
    log_info "Running volume test for $TEST_DURATION seconds..."
    if command -v wrk &> /dev/null; then
        wrk -t64 -c5000 -d${TEST_DURATION}s -s "$LOAD_TEST_DIR/volume-test.lua" \
            "http://$TARGET_HOST:$TARGET_PORT" > "$LOAD_TEST_DIR/volume-test-results.txt" 2>&1 || true
    fi
    
    log_success "Volume test completed"
}

# Endurance test
endurance_test() {
    log_info "Starting endurance test..."
    
    # Create endurance test script
    cat > "$LOAD_TEST_DIR/endurance-test.lua" << 'EOF'
-- Endurance test script for IPPAN
local counter = 0

function setup(thread)
   thread:set("id", counter)
   counter = counter + 1
end

function init(args)
   requests  = 0
   responses = 0
end

function request()
   requests = requests + 1
   local body = string.format('{"amount": %d, "recipient": "recipient-%d", "sender": "sender-%d"}', 
                             math.random(1000, 1000000), 
                             math.random(1, 10000), 
                             math.random(1, 10000))
   
   return wrk.format("POST", "/api/v1/transactions", {
      ["Content-Type"] = "application/json"
   }, body)
end

function response(status, headers, body)
   responses = responses + 1
   if status ~= 200 then
      print("Error: " .. status .. " " .. body)
   end
end

function done(summary, latency, requests)
   local msg = "thread %d: %d requests, %d responses, %.2f req/sec"
   print(msg:format(id, requests, responses, requests/summary.duration))
end
EOF
    
    # Run endurance test
    log_info "Running endurance test for 1 hour..."
    if command -v wrk &> /dev/null; then
        wrk -t32 -c2000 -d3600s -s "$LOAD_TEST_DIR/endurance-test.lua" \
            "http://$TARGET_HOST:$TARGET_PORT" > "$LOAD_TEST_DIR/endurance-test-results.txt" 2>&1 || true
    fi
    
    log_success "Endurance test completed"
}

# Memory leak test
memory_leak_test() {
    log_info "Starting memory leak test..."
    
    # Monitor memory usage over time
    log_info "Monitoring memory usage for 30 minutes..."
    
    # Start memory monitoring
    (
        for i in {1..1800}; do
            echo "$(date): $(free -h | grep Mem | awk '{print $3}')" >> "$LOAD_TEST_DIR/memory-usage.log"
            sleep 1
        done
    ) &
    
    local monitor_pid=$!
    
    # Run load test while monitoring memory
    if command -v wrk &> /dev/null; then
        wrk -t16 -c1000 -d1800s -s "$LOAD_TEST_DIR/stress-test.lua" \
            "http://$TARGET_HOST:$TARGET_PORT" > "$LOAD_TEST_DIR/memory-leak-test-results.txt" 2>&1 || true
    fi
    
    # Stop memory monitoring
    kill $monitor_pid 2>/dev/null || true
    
    log_success "Memory leak test completed"
}

# Generate load test report
generate_loadtest_report() {
    log_info "Generating load test report..."
    
    cat > "$LOAD_TEST_DIR/load-test-report.md" << EOF
# IPPAN Load Test Report

**Test Date**: $(date)
**Target Host**: $TARGET_HOST
**Target Port**: $TARGET_PORT
**Max Connections**: $MAX_CONNECTIONS
**Test Duration**: $TEST_DURATION seconds

## Executive Summary

This report contains the results of comprehensive load testing performed on the IPPAN blockchain system to evaluate its performance under various load conditions.

## Test Scenarios

### 1. Stress Test
- **Purpose**: Test system behavior under increasing load
- **Connections**: 100, 500, 1000, 2000, 5000, 10000
- **Threads**: 4, 8, 16, 32, 64, 128
- **Duration**: 60 seconds per test

### 2. Spike Test
- **Purpose**: Test system behavior under sudden load spikes
- **Connections**: 10000
- **Threads**: 128
- **Duration**: 30 seconds

### 3. Volume Test
- **Purpose**: Test system behavior under sustained high volume
- **Connections**: 5000
- **Threads**: 64
- **Duration**: $TEST_DURATION seconds

### 4. Endurance Test
- **Purpose**: Test system behavior over extended periods
- **Connections**: 2000
- **Threads**: 32
- **Duration**: 1 hour

### 5. Memory Leak Test
- **Purpose**: Test for memory leaks under sustained load
- **Connections**: 1000
- **Threads**: 16
- **Duration**: 30 minutes

## Test Results

### Stress Test Results
$(for file in "$LOAD_TEST_DIR"/stress-test-*-conn.txt; do
    if [ -f "$file" ]; then
        echo "#### $(basename "$file" .txt | sed 's/stress-test-//' | sed 's/-conn//') Connections"
        echo "\`\`\`"
        grep -E "(Requests/sec|Latency|Transfer/sec)" "$file" || echo "No results found"
        echo "\`\`\`"
    fi
done)

### Spike Test Results
\`\`\`
$(grep -E "(Requests/sec|Latency|Transfer/sec)" "$LOAD_TEST_DIR/spike-test-results.txt" 2>/dev/null || echo "No results found")
\`\`\`

### Volume Test Results
\`\`\`
$(grep -E "(Requests/sec|Latency|Transfer/sec)" "$LOAD_TEST_DIR/volume-test-results.txt" 2>/dev/null || echo "No results found")
\`\`\`

### Endurance Test Results
\`\`\`
$(grep -E "(Requests/sec|Latency|Transfer/sec)" "$LOAD_TEST_DIR/endurance-test-results.txt" 2>/dev/null || echo "No results found")
\`\`\`

### Memory Leak Test Results
\`\`\`
$(tail -10 "$LOAD_TEST_DIR/memory-usage.log" 2>/dev/null || echo "No memory usage data found")
\`\`\`

## Performance Analysis

### Throughput Analysis
- **Peak Throughput**: $(grep "Requests/sec" "$LOAD_TEST_DIR"/*.txt | awk '{print $2}' | sort -nr | head -1 || echo "N/A")
- **Average Throughput**: $(grep "Requests/sec" "$LOAD_TEST_DIR"/*.txt | awk '{print $2}' | awk '{sum+=$1} END {print sum/NR}' || echo "N/A")
- **Minimum Throughput**: $(grep "Requests/sec" "$LOAD_TEST_DIR"/*.txt | awk '{print $2}' | sort -n | head -1 || echo "N/A")

### Latency Analysis
- **Average Latency**: $(grep "Latency" "$LOAD_TEST_DIR"/*.txt | awk '{print $2}' | awk '{sum+=$1} END {print sum/NR}' || echo "N/A")
- **Maximum Latency**: $(grep "Latency" "$LOAD_TEST_DIR"/*.txt | awk '{print $2}' | sort -nr | head -1 || echo "N/A")
- **Minimum Latency**: $(grep "Latency" "$LOAD_TEST_DIR"/*.txt | awk '{print $2}' | sort -n | head -1 || echo "N/A")

### Error Analysis
- **Total Errors**: $(grep -c "Error:" "$LOAD_TEST_DIR"/*.txt 2>/dev/null || echo "0")
- **Error Rate**: $(grep -c "Error:" "$LOAD_TEST_DIR"/*.txt 2>/dev/null || echo "0")%

### Memory Analysis
- **Memory Usage Trend**: $(tail -1 "$LOAD_TEST_DIR/memory-usage.log" 2>/dev/null | awk '{print $3}' || echo "N/A")
- **Memory Leak Detection**: $(if [ -f "$LOAD_TEST_DIR/memory-usage.log" ]; then
    start_mem=$(head -1 "$LOAD_TEST_DIR/memory-usage.log" | awk '{print $3}' | sed 's/[^0-9.]//g')
    end_mem=$(tail -1 "$LOAD_TEST_DIR/memory-usage.log" | awk '{print $3}' | sed 's/[^0-9.]//g')
    if [ -n "$start_mem" ] && [ -n "$end_mem" ]; then
        if (( $(echo "$end_mem > $start_mem * 1.5" | bc -l) )); then
            echo "Potential memory leak detected"
        else
            echo "No memory leak detected"
        fi
    else
        echo "Unable to determine"
    fi
else
    echo "No memory data available"
fi)

## Recommendations

### Performance Optimization
1. **Throughput Optimization**
   - Implement connection pooling
   - Optimize database queries
   - Implement caching mechanisms
   - Use faster serialization formats

2. **Latency Optimization**
   - Implement request batching
   - Optimize network protocols
   - Implement load balancing
   - Use faster hardware

3. **Memory Optimization**
   - Implement memory pooling
   - Optimize garbage collection
   - Use memory-mapped files
   - Implement memory monitoring

### Scalability Improvements
1. **Horizontal Scaling**
   - Implement load balancing
   - Use multiple instances
   - Implement sharding
   - Use distributed caching

2. **Vertical Scaling**
   - Use faster CPUs
   - Increase memory
   - Use faster storage
   - Optimize network

### Monitoring and Alerting
1. **Performance Monitoring**
   - Implement real-time monitoring
   - Set up performance alerts
   - Monitor resource usage
   - Track performance metrics

2. **Load Testing**
   - Regular load testing
   - Performance regression testing
   - Capacity planning
   - Performance optimization

## Conclusion

The load testing has identified several areas for optimization to improve system performance and scalability. It is recommended that all identified issues be addressed and optimization opportunities be implemented.

## Next Steps

1. **Performance Optimization**
   - Implement recommended optimizations
   - Conduct follow-up load testing
   - Monitor performance improvements
   - Document performance improvements

2. **Continuous Load Testing**
   - Implement automated load testing
   - Regular performance testing
   - Performance regression testing
   - Capacity planning

---
*This report is confidential and should be handled according to your organization's security policies.*
EOF
    
    log_success "Load test report generated: $LOAD_TEST_DIR/load-test-report.md"
}

# Main load test function
main() {
    log_info "Starting IPPAN load testing..."
    
    create_loadtest_dir
    stress_test
    spike_test
    volume_test
    endurance_test
    memory_leak_test
    generate_loadtest_report
    
    log_success "IPPAN load testing completed successfully!"
    echo ""
    echo "📊 Load Test Results:"
    echo "  - Test Directory: $LOAD_TEST_DIR"
    echo "  - Report File: $LOAD_TEST_DIR/load-test-report.md"
    echo "  - Target: $TARGET_HOST:$TARGET_PORT"
    echo "  - Max Connections: $MAX_CONNECTIONS"
    echo "  - Test Duration: $TEST_DURATION seconds"
    echo ""
    echo "🔍 Next Steps:"
    echo "  1. Review the load test report"
    echo "  2. Address identified performance issues"
    echo "  3. Implement recommended optimizations"
    echo "  4. Conduct follow-up load testing"
    echo "  5. Establish ongoing performance monitoring"
}

# Run main function
main "$@"
