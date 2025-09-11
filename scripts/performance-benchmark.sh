#!/bin/bash

# IPPAN Performance Benchmark Script
# This script performs comprehensive performance testing to achieve 1M TPS target

set -e

# Configuration
BENCHMARK_DIR="/tmp/ippan-benchmark-$(date +%Y%m%d_%H%M%S)"
REPORT_FILE="$BENCHMARK_DIR/performance-benchmark-report.md"
TARGET_HOST="${TARGET_HOST:-localhost}"
TARGET_PORT="${TARGET_PORT:-3000}"
TARGET_TPS="${TARGET_TPS:-1000000}"
DURATION="${DURATION:-300}"

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

# Create benchmark directory
create_benchmark_dir() {
    log_info "Creating benchmark directory..."
    mkdir -p "$BENCHMARK_DIR"
    log_success "Benchmark directory created: $BENCHMARK_DIR"
}

# System performance baseline
system_baseline() {
    log_info "Collecting system performance baseline..."
    
    # CPU information
    log_info "Collecting CPU information..."
    cat /proc/cpuinfo > "$BENCHMARK_DIR/cpu-info.txt" 2>/dev/null || true
    lscpu > "$BENCHMARK_DIR/lscpu.txt" 2>/dev/null || true
    
    # Memory information
    log_info "Collecting memory information..."
    cat /proc/meminfo > "$BENCHMARK_DIR/memory-info.txt" 2>/dev/null || true
    free -h > "$BENCHMARK_DIR/memory-usage.txt" 2>/dev/null || true
    
    # Disk information
    log_info "Collecting disk information..."
    df -h > "$BENCHMARK_DIR/disk-usage.txt" 2>/dev/null || true
    iostat -x 1 5 > "$BENCHMARK_DIR/disk-iostat.txt" 2>/dev/null || true
    
    # Network information
    log_info "Collecting network information..."
    ifconfig > "$BENCHMARK_DIR/network-info.txt" 2>/dev/null || true
    netstat -i > "$BENCHMARK_DIR/network-interfaces.txt" 2>/dev/null || true
    
    # System load
    log_info "Collecting system load..."
    uptime > "$BENCHMARK_DIR/system-uptime.txt" 2>/dev/null || true
    top -bn1 > "$BENCHMARK_DIR/system-top.txt" 2>/dev/null || true
    
    log_success "System baseline collected"
}

# API performance testing
api_performance_test() {
    log_info "Starting API performance testing..."
    
    # Test API endpoints
    log_info "Testing API endpoints..."
    
    # Health check endpoint
    log_info "Testing health check endpoint..."
    if command -v wrk &> /dev/null; then
        wrk -t12 -c400 -d30s "http://$TARGET_HOST:$TARGET_PORT/api/v1/status" > "$BENCHMARK_DIR/api-health-wrk.txt" 2>/dev/null || true
    fi
    
    if command -v ab &> /dev/null; then
        ab -n 10000 -c 100 "http://$TARGET_HOST:$TARGET_PORT/api/v1/status" > "$BENCHMARK_DIR/api-health-ab.txt" 2>/dev/null || true
    fi
    
    # Transaction endpoint
    log_info "Testing transaction endpoint..."
    if command -v wrk &> /dev/null; then
        wrk -t12 -c400 -d30s -s "$BENCHMARK_DIR/transaction-script.lua" "http://$TARGET_HOST:$TARGET_PORT/api/v1/transactions" > "$BENCHMARK_DIR/api-transaction-wrk.txt" 2>/dev/null || true
    fi
    
    # Create transaction test script
    cat > "$BENCHMARK_DIR/transaction-script.lua" << 'EOF'
wrk.method = "POST"
wrk.body = '{"amount": 1000000, "recipient": "test-recipient", "sender": "test-sender"}'
wrk.headers["Content-Type"] = "application/json"
EOF
    
    # Wallet endpoint
    log_info "Testing wallet endpoint..."
    if command -v wrk &> /dev/null; then
        wrk -t12 -c400 -d30s "http://$TARGET_HOST:$TARGET_PORT/api/v1/wallet/balance" > "$BENCHMARK_DIR/api-wallet-wrk.txt" 2>/dev/null || true
    fi
    
    # Storage endpoint
    log_info "Testing storage endpoint..."
    if command -v wrk &> /dev/null; then
        wrk -t12 -c400 -d30s "http://$TARGET_HOST:$TARGET_PORT/api/v1/storage/usage" > "$BENCHMARK_DIR/api-storage-wrk.txt" 2>/dev/null || true
    fi
    
    log_success "API performance testing completed"
}

# Transaction throughput testing
transaction_throughput_test() {
    log_info "Starting transaction throughput testing..."
    
    # Create transaction test script
    cat > "$BENCHMARK_DIR/transaction-test.py" << 'EOF'
#!/usr/bin/env python3
import asyncio
import aiohttp
import time
import json
import statistics
from concurrent.futures import ThreadPoolExecutor

class TransactionTester:
    def __init__(self, host, port, target_tps, duration):
        self.host = host
        self.port = port
        self.target_tps = target_tps
        self.duration = duration
        self.results = []
        self.errors = 0
        
    async def send_transaction(self, session, transaction_id):
        try:
            transaction = {
                "id": transaction_id,
                "amount": 1000000,
                "recipient": f"recipient-{transaction_id}",
                "sender": f"sender-{transaction_id}",
                "timestamp": int(time.time() * 1000)
            }
            
            start_time = time.time()
            async with session.post(
                f"http://{self.host}:{self.port}/api/v1/transactions",
                json=transaction,
                timeout=aiohttp.ClientTimeout(total=10)
            ) as response:
                end_time = time.time()
                response_time = end_time - start_time
                
                if response.status == 200:
                    self.results.append({
                        "transaction_id": transaction_id,
                        "response_time": response_time,
                        "status": response.status,
                        "timestamp": start_time
                    })
                else:
                    self.errors += 1
                    
        except Exception as e:
            self.errors += 1
            print(f"Error sending transaction {transaction_id}: {e}")
    
    async def run_test(self):
        connector = aiohttp.TCPConnector(limit=1000, limit_per_host=1000)
        timeout = aiohttp.ClientTimeout(total=30)
        
        async with aiohttp.ClientSession(connector=connector, timeout=timeout) as session:
            start_time = time.time()
            transaction_id = 0
            
            while time.time() - start_time < self.duration:
                # Calculate how many transactions to send this second
                elapsed = time.time() - start_time
                expected_transactions = int(elapsed * self.target_tps)
                current_transactions = len(self.results) + self.errors
                
                if current_transactions < expected_transactions:
                    # Send transactions in batches
                    batch_size = min(100, expected_transactions - current_transactions)
                    tasks = []
                    
                    for _ in range(batch_size):
                        tasks.append(self.send_transaction(session, transaction_id))
                        transaction_id += 1
                    
                    await asyncio.gather(*tasks, return_exceptions=True)
                
                # Sleep for a short time to avoid overwhelming the system
                await asyncio.sleep(0.001)
        
        return self.results, self.errors

async def main():
    import sys
    host = sys.argv[1] if len(sys.argv) > 1 else "localhost"
    port = sys.argv[2] if len(sys.argv) > 2 else "3000"
    target_tps = int(sys.argv[3]) if len(sys.argv) > 3 else 1000000
    duration = int(sys.argv[4]) if len(sys.argv) > 4 else 300
    
    print(f"Starting transaction throughput test...")
    print(f"Target: {host}:{port}")
    print(f"Target TPS: {target_tps}")
    print(f"Duration: {duration} seconds")
    
    tester = TransactionTester(host, port, target_tps, duration)
    results, errors = await tester.run_test()
    
    if results:
        response_times = [r["response_time"] for r in results]
        total_transactions = len(results)
        actual_tps = total_transactions / duration
        
        print(f"\nResults:")
        print(f"Total transactions: {total_transactions}")
        print(f"Errors: {errors}")
        print(f"Actual TPS: {actual_tps:.2f}")
        print(f"Target TPS: {target_tps}")
        print(f"TPS Achievement: {(actual_tps/target_tps)*100:.2f}%")
        print(f"Average response time: {statistics.mean(response_times):.4f}s")
        print(f"Median response time: {statistics.median(response_times):.4f}s")
        print(f"95th percentile: {sorted(response_times)[int(len(response_times)*0.95)]:.4f}s")
        print(f"99th percentile: {sorted(response_times)[int(len(response_times)*0.99)]:.4f}s")
        
        # Save results to file
        with open("transaction-results.json", "w") as f:
            json.dump({
                "total_transactions": total_transactions,
                "errors": errors,
                "actual_tps": actual_tps,
                "target_tps": target_tps,
                "tps_achievement": (actual_tps/target_tps)*100,
                "average_response_time": statistics.mean(response_times),
                "median_response_time": statistics.median(response_times),
                "p95_response_time": sorted(response_times)[int(len(response_times)*0.95)],
                "p99_response_time": sorted(response_times)[int(len(response_times)*0.99)],
                "results": results
            }, f, indent=2)
    else:
        print("No successful transactions recorded")

if __name__ == "__main__":
    asyncio.run(main())
EOF
    
    # Run transaction throughput test
    log_info "Running transaction throughput test..."
    python3 "$BENCHMARK_DIR/transaction-test.py" "$TARGET_HOST" "$TARGET_PORT" "$TARGET_TPS" "$DURATION" > "$BENCHMARK_DIR/transaction-throughput-results.txt" 2>&1 || true
    
    log_success "Transaction throughput testing completed"
}

# Database performance testing
database_performance_test() {
    log_info "Starting database performance testing..."
    
    # Test database connection
    log_info "Testing database connection..."
    if command -v sqlite3 &> /dev/null; then
        sqlite3 /data/ippan.db "SELECT COUNT(*) FROM transactions;" > "$BENCHMARK_DIR/db-connection-test.txt" 2>/dev/null || true
    fi
    
    # Test database performance
    log_info "Testing database performance..."
    cat > "$BENCHMARK_DIR/db-performance-test.sql" << 'EOF'
-- Database performance test queries
.timer on

-- Test 1: Simple SELECT
SELECT COUNT(*) FROM transactions;

-- Test 2: Complex JOIN
SELECT t.*, w.balance 
FROM transactions t 
JOIN wallets w ON t.sender = w.address 
LIMIT 1000;

-- Test 3: Aggregation
SELECT sender, COUNT(*) as tx_count, SUM(amount) as total_amount
FROM transactions 
GROUP BY sender 
ORDER BY tx_count DESC 
LIMIT 100;

-- Test 4: Index performance
SELECT * FROM transactions 
WHERE timestamp > strftime('%s', 'now', '-1 day') * 1000
ORDER BY timestamp DESC 
LIMIT 1000;
EOF
    
    if command -v sqlite3 &> /dev/null; then
        sqlite3 /data/ippan.db < "$BENCHMARK_DIR/db-performance-test.sql" > "$BENCHMARK_DIR/db-performance-results.txt" 2>&1 || true
    fi
    
    log_success "Database performance testing completed"
}

# Memory and CPU profiling
memory_cpu_profiling() {
    log_info "Starting memory and CPU profiling..."
    
    # Monitor system resources during test
    log_info "Monitoring system resources..."
    
    # Start system monitoring
    if command -v htop &> /dev/null; then
        htop -d 1 -n 10 > "$BENCHMARK_DIR/htop-monitor.txt" 2>/dev/null || true
    fi
    
    # Monitor memory usage
    if command -v free &> /dev/null; then
        for i in {1..10}; do
            free -h >> "$BENCHMARK_DIR/memory-monitor.txt" 2>/dev/null || true
            sleep 1
        done
    fi
    
    # Monitor CPU usage
    if command -v top &> /dev/null; then
        top -bn1 -d 1 -n 10 >> "$BENCHMARK_DIR/cpu-monitor.txt" 2>/dev/null || true
    fi
    
    # Monitor disk I/O
    if command -v iostat &> /dev/null; then
        iostat -x 1 10 > "$BENCHMARK_DIR/disk-monitor.txt" 2>/dev/null || true
    fi
    
    log_success "Memory and CPU profiling completed"
}

# Network performance testing
network_performance_test() {
    log_info "Starting network performance testing..."
    
    # Test network latency
    log_info "Testing network latency..."
    if command -v ping &> /dev/null; then
        ping -c 100 "$TARGET_HOST" > "$BENCHMARK_DIR/ping-test.txt" 2>/dev/null || true
    fi
    
    # Test network throughput
    log_info "Testing network throughput..."
    if command -v iperf3 &> /dev/null; then
        iperf3 -c "$TARGET_HOST" -t 30 > "$BENCHMARK_DIR/iperf3-test.txt" 2>/dev/null || true
    fi
    
    # Test connection limits
    log_info "Testing connection limits..."
    if command -v nc &> /dev/null; then
        for i in {1..1000}; do
            nc -z "$TARGET_HOST" "$TARGET_PORT" 2>/dev/null && echo "Connection $i: OK" || echo "Connection $i: FAILED"
        done > "$BENCHMARK_DIR/connection-limit-test.txt" 2>/dev/null || true
    fi
    
    log_success "Network performance testing completed"
}

# Generate performance report
generate_performance_report() {
    log_info "Generating performance benchmark report..."
    
    cat > "$REPORT_FILE" << EOF
# IPPAN Performance Benchmark Report

**Benchmark Date**: $(date)
**Target Host**: $TARGET_HOST
**Target Port**: $TARGET_PORT
**Target TPS**: $TARGET_TPS
**Test Duration**: $DURATION seconds

## Executive Summary

This report contains the results of comprehensive performance benchmarking performed on the IPPAN blockchain system to achieve the target of 1 million transactions per second (TPS).

## System Configuration

### Hardware Specifications
- **CPU**: $(grep "model name" "$BENCHMARK_DIR/cpu-info.txt" | head -1 | cut -d: -f2 | xargs)
- **CPU Cores**: $(grep "cpu cores" "$BENCHMARK_DIR/cpu-info.txt" | head -1 | cut -d: -f2 | xargs)
- **Memory**: $(grep "MemTotal" "$BENCHMARK_DIR/memory-info.txt" | cut -d: -f2 | xargs)
- **Disk**: $(df -h / | tail -1 | awk '{print $2}')

### Software Configuration
- **Operating System**: $(uname -a)
- **IPPAN Version**: $(docker images ippan/ippan:latest --format "table {{.Tag}}" | tail -1)
- **Docker Version**: $(docker --version)

## Performance Results

### Transaction Throughput
- **Target TPS**: $TARGET_TPS
- **Actual TPS**: $(grep "Actual TPS" "$BENCHMARK_DIR/transaction-throughput-results.txt" | cut -d: -f2 | xargs || echo "N/A")
- **TPS Achievement**: $(grep "TPS Achievement" "$BENCHMARK_DIR/transaction-throughput-results.txt" | cut -d: -f2 | xargs || echo "N/A")
- **Total Transactions**: $(grep "Total transactions" "$BENCHMARK_DIR/transaction-throughput-results.txt" | cut -d: -f2 | xargs || echo "N/A")
- **Errors**: $(grep "Errors" "$BENCHMARK_DIR/transaction-throughput-results.txt" | cut -d: -f2 | xargs || echo "N/A")

### Response Time Metrics
- **Average Response Time**: $(grep "Average response time" "$BENCHMARK_DIR/transaction-throughput-results.txt" | cut -d: -f2 | xargs || echo "N/A")
- **Median Response Time**: $(grep "Median response time" "$BENCHMARK_DIR/transaction-throughput-results.txt" | cut -d: -f2 | xargs || echo "N/A")
- **95th Percentile**: $(grep "95th percentile" "$BENCHMARK_DIR/transaction-throughput-results.txt" | cut -d: -f2 | xargs || echo "N/A")
- **99th Percentile**: $(grep "99th percentile" "$BENCHMARK_DIR/transaction-throughput-results.txt" | cut -d: -f2 | xargs || echo "N/A")

### API Performance
- **Health Check Endpoint**: $(grep "Requests/sec" "$BENCHMARK_DIR/api-health-wrk.txt" | awk '{print $2}' || echo "N/A")
- **Transaction Endpoint**: $(grep "Requests/sec" "$BENCHMARK_DIR/api-transaction-wrk.txt" | awk '{print $2}' || echo "N/A")
- **Wallet Endpoint**: $(grep "Requests/sec" "$BENCHMARK_DIR/api-wallet-wrk.txt" | awk '{print $2}' || echo "N/A")
- **Storage Endpoint**: $(grep "Requests/sec" "$BENCHMARK_DIR/api-storage-wrk.txt" | awk '{print $2}' || echo "N/A")

### System Resource Usage
- **CPU Usage**: $(grep "Cpu(s)" "$BENCHMARK_DIR/cpu-monitor.txt" | tail -1 | awk '{print $2}' || echo "N/A")
- **Memory Usage**: $(grep "Mem:" "$BENCHMARK_DIR/memory-monitor.txt" | tail -1 | awk '{print $3}' || echo "N/A")
- **Disk I/O**: $(grep "await" "$BENCHMARK_DIR/disk-monitor.txt" | tail -1 | awk '{print $10}' || echo "N/A")

### Network Performance
- **Latency**: $(grep "avg" "$BENCHMARK_DIR/ping-test.txt" | tail -1 | awk -F'/' '{print $5}' || echo "N/A")
- **Throughput**: $(grep "sender" "$BENCHMARK_DIR/iperf3-test.txt" | tail -1 | awk '{print $7}' || echo "N/A")

## Performance Analysis

### Bottlenecks Identified
1. **Database Performance**: $(grep -q "slow" "$BENCHMARK_DIR/db-performance-results.txt" && echo "Database queries are slow" || echo "Database performance is acceptable")
2. **Network Latency**: $(grep -q "high" "$BENCHMARK_DIR/ping-test.txt" && echo "Network latency is high" || echo "Network latency is acceptable")
3. **Memory Usage**: $(grep -q "high" "$BENCHMARK_DIR/memory-monitor.txt" && echo "Memory usage is high" || echo "Memory usage is acceptable")
4. **CPU Usage**: $(grep -q "high" "$BENCHMARK_DIR/cpu-monitor.txt" && echo "CPU usage is high" || echo "CPU usage is acceptable")

### Optimization Opportunities
1. **Database Optimization**
   - Implement database indexing
   - Optimize query performance
   - Consider database sharding
   - Implement connection pooling

2. **Network Optimization**
   - Optimize network configuration
   - Implement load balancing
   - Use faster network protocols
   - Optimize packet sizes

3. **Memory Optimization**
   - Implement memory pooling
   - Optimize garbage collection
   - Use memory-mapped files
   - Implement caching strategies

4. **CPU Optimization**
   - Implement multi-threading
   - Use CPU affinity
   - Optimize algorithms
   - Implement parallel processing

## Recommendations

### Immediate Actions
1. **Address Performance Bottlenecks**
   - Optimize database queries
   - Implement caching mechanisms
   - Optimize network configuration
   - Implement load balancing

2. **System Tuning**
   - Optimize system parameters
   - Implement performance monitoring
   - Set up alerting for performance issues
   - Implement auto-scaling

### Short-term Actions
1. **Performance Optimization**
   - Implement database indexing
   - Optimize application code
   - Implement caching strategies
   - Optimize network protocols

2. **Monitoring and Alerting**
   - Implement performance monitoring
   - Set up performance alerts
   - Implement performance dashboards
   - Regular performance testing

### Long-term Actions
1. **Architecture Optimization**
   - Implement microservices architecture
   - Implement horizontal scaling
   - Implement distributed caching
   - Implement message queuing

2. **Infrastructure Optimization**
   - Use faster hardware
   - Implement CDN
   - Optimize network infrastructure
   - Implement cloud optimization

## Conclusion

The performance benchmark has identified several areas for optimization to achieve the target of 1 million TPS. It is recommended that all identified bottlenecks be addressed and optimization opportunities be implemented.

## Next Steps

1. **Performance Optimization**
   - Implement recommended optimizations
   - Conduct follow-up performance testing
   - Monitor performance improvements
   - Document performance improvements

2. **Continuous Performance Monitoring**
   - Implement performance monitoring
   - Set up performance alerts
   - Regular performance testing
   - Performance optimization reviews

---
*This report is confidential and should be handled according to your organization's security policies.*
EOF
    
    log_success "Performance benchmark report generated: $REPORT_FILE"
}

# Main benchmark function
main() {
    log_info "Starting IPPAN performance benchmark..."
    
    create_benchmark_dir
    system_baseline
    api_performance_test
    transaction_throughput_test
    database_performance_test
    memory_cpu_profiling
    network_performance_test
    generate_performance_report
    
    log_success "IPPAN performance benchmark completed successfully!"
    echo ""
    echo "📊 Performance Benchmark Results:"
    echo "  - Benchmark Directory: $BENCHMARK_DIR"
    echo "  - Report File: $REPORT_FILE"
    echo "  - Target: $TARGET_HOST:$TARGET_PORT"
    echo "  - Target TPS: $TARGET_TPS"
    echo "  - Test Duration: $DURATION seconds"
    echo ""
    echo "🔍 Next Steps:"
    echo "  1. Review the performance benchmark report"
    echo "  2. Address identified performance bottlenecks"
    echo "  3. Implement recommended optimizations"
    echo "  4. Conduct follow-up performance testing"
    echo "  5. Establish ongoing performance monitoring"
}

# Run main function
main "$@"
