#!/bin/bash

# IPPAN Performance Testing Script
set -e

echo "🚀 Starting IPPAN Performance Tests..."

# Check if k6 is installed
if ! command -v k6 &> /dev/null; then
    echo "❌ k6 not found. Installing k6..."
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        sudo apt-key adv --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
        echo "deb https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
        sudo apt-get update
        sudo apt-get install k6
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        brew install k6
    else
        echo "Please install k6 manually from https://k6.io/docs/getting-started/installation/"
        exit 1
    fi
fi

# Check if the application is running
echo "🔍 Checking if IPPAN application is running..."
if ! curl -f http://localhost:3001/health > /dev/null 2>&1; then
    echo "❌ IPPAN application is not running on localhost:3001"
    echo "Please start the application first:"
    echo "  cd apps/unified-ui && npm run server"
    exit 1
fi

echo "✅ IPPAN application is running"

# Create results directory
mkdir -p test-results
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RESULTS_DIR="test-results/performance_$TIMESTAMP"
mkdir -p "$RESULTS_DIR"

echo "📊 Running performance tests..."

# Test 1: Basic Load Test
echo "🧪 Test 1: Basic Load Test (100 users)"
k6 run --out json="$RESULTS_DIR/basic_load_test.json" tests/load-test.js

# Test 2: Stress Test
echo "🧪 Test 2: Stress Test (1000 users)"
k6 run --stage 2m:1000,5m:1000,2m:0 --out json="$RESULTS_DIR/stress_test.json" tests/load-test.js

# Test 3: Spike Test
echo "🧪 Test 3: Spike Test (sudden load increase)"
k6 run --stage 1m:100,30s:1000,1m:100,30s:1000,1m:100,30s:0 --out json="$RESULTS_DIR/spike_test.json" tests/load-test.js

# Test 4: Volume Test
echo "🧪 Test 4: Volume Test (sustained high load)"
k6 run --stage 2m:500,10m:500,2m:0 --out json="$RESULTS_DIR/volume_test.json" tests/load-test.js

# Test 5: Endurance Test
echo "🧪 Test 5: Endurance Test (long duration)"
k6 run --stage 5m:100,30m:100,5m:0 --out json="$RESULTS_DIR/endurance_test.json" tests/load-test.js

echo "📈 Generating performance report..."

# Generate summary report
cat > "$RESULTS_DIR/performance_summary.md" << EOF
# IPPAN Performance Test Results

**Test Date:** $(date)
**Test Duration:** $(date -d @$(($(date +%s) - $(date -d "$TIMESTAMP" +%s))) -u +%H:%M:%S)

## Test Results Summary

### 1. Basic Load Test (100 users)
- **Target:** 100 concurrent users
- **Duration:** 19 minutes
- **Results:** See basic_load_test.json

### 2. Stress Test (1000 users)
- **Target:** 1000 concurrent users
- **Duration:** 9 minutes
- **Results:** See stress_test.json

### 3. Spike Test
- **Target:** Sudden load spikes from 100 to 1000 users
- **Duration:** 5 minutes
- **Results:** See spike_test.json

### 4. Volume Test (500 users)
- **Target:** 500 concurrent users sustained
- **Duration:** 14 minutes
- **Results:** See volume_test.json

### 5. Endurance Test (100 users)
- **Target:** 100 concurrent users for extended period
- **Duration:** 40 minutes
- **Results:** See endurance_test.json

## Performance Targets

- **Response Time:** 95th percentile < 1000ms
- **Error Rate:** < 10%
- **Throughput:** Target 1-10M TPS
- **Availability:** 99.9% uptime

## Recommendations

Based on the test results, the following optimizations may be needed:

1. **Database Optimization:** If response times are high
2. **Caching Layer:** For frequently accessed data
3. **Load Balancing:** For horizontal scaling
4. **Resource Scaling:** CPU/Memory optimization

## Next Steps

1. Review individual test results
2. Identify performance bottlenecks
3. Implement optimizations
4. Re-run tests to validate improvements
5. Deploy to production with confidence

EOF

echo "✅ Performance tests completed!"
echo "📁 Results saved to: $RESULTS_DIR"
echo "📊 Summary report: $RESULTS_DIR/performance_summary.md"

# Display quick summary
echo ""
echo "🎯 Quick Summary:"
echo "=================="
echo "All performance tests have been completed."
echo "Check the results directory for detailed analysis."
echo ""
echo "To view results:"
echo "  cat $RESULTS_DIR/performance_summary.md"
echo ""
echo "To analyze specific tests:"
echo "  jq '.metrics' $RESULTS_DIR/basic_load_test.json"
echo ""

# Check if results meet targets
echo "🔍 Checking if results meet performance targets..."
if [ -f "$RESULTS_DIR/basic_load_test.json" ]; then
    ERROR_RATE=$(jq -r '.metrics.http_req_failed.values.rate' "$RESULTS_DIR/basic_load_test.json")
    P95_RESPONSE=$(jq -r '.metrics.http_req_duration.values["p(95)"]' "$RESULTS_DIR/basic_load_test.json")
    
    echo "Basic Load Test Results:"
    echo "  Error Rate: $(echo "$ERROR_RATE * 100" | bc -l | cut -c1-5)%"
    echo "  95th Percentile Response Time: $(echo "$P95_RESPONSE" | cut -c1-6)ms"
    
    if (( $(echo "$ERROR_RATE < 0.1" | bc -l) )) && (( $(echo "$P95_RESPONSE < 1000" | bc -l) )); then
        echo "✅ Performance targets met!"
    else
        echo "⚠️  Performance targets not met. Review results for optimization opportunities."
    fi
fi

echo ""
echo "🚀 Performance testing complete!"
