import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate } from 'k6/metrics';

// Custom metrics
export let errorRate = new Rate('errors');

// Test configuration
export let options = {
  stages: [
    { duration: '2m', target: 100 },   // Ramp up to 100 users
    { duration: '5m', target: 100 },   // Stay at 100 users
    { duration: '2m', target: 500 },   // Ramp up to 500 users
    { duration: '5m', target: 500 },   // Stay at 500 users
    { duration: '2m', target: 1000 },  // Ramp up to 1000 users
    { duration: '5m', target: 1000 },  // Stay at 1000 users
    { duration: '2m', target: 0 },     // Ramp down to 0 users
  ],
  thresholds: {
    http_req_duration: ['p(95)<1000'], // 95% of requests must complete below 1s
    http_req_failed: ['rate<0.1'],     // Error rate must be below 10%
    errors: ['rate<0.1'],              // Custom error rate must be below 10%
  },
};

const BASE_URL = 'http://localhost:3001';

// Test data
const testWallet = 'i1testwallet1234567890abcdef1234567890abcdef1234567890abcdef1234567890';
const testDomain = 'test.ippan';

export default function() {
  // Test 1: Health check
  let response = http.get(`${BASE_URL}/health`);
  check(response, {
    'health check status is 200': (r) => r.status === 200,
    'health check response time < 100ms': (r) => r.timings.duration < 100,
  });
  errorRate.add(response.status !== 200);

  // Test 2: Get wallet balance
  response = http.get(`${BASE_URL}/wallet/balance`, {
    headers: { 'X-Wallet-Address': testWallet }
  });
  check(response, {
    'wallet balance status is 200': (r) => r.status === 200,
    'wallet balance response time < 500ms': (r) => r.timings.duration < 500,
  });
  errorRate.add(response.status !== 200);

  // Test 3: List domains
  response = http.get(`${BASE_URL}/domains`);
  check(response, {
    'domains list status is 200': (r) => r.status === 200,
    'domains list response time < 500ms': (r) => r.timings.duration < 500,
  });
  errorRate.add(response.status !== 200);

  // Test 4: List storage files
  response = http.get(`${BASE_URL}/storage/files`);
  check(response, {
    'storage files status is 200': (r) => r.status === 200,
    'storage files response time < 500ms': (r) => r.timings.duration < 500,
  });
  errorRate.add(response.status !== 200);

  // Test 5: List AI models
  response = http.get(`${BASE_URL}/models`);
  check(response, {
    'models list status is 200': (r) => r.status === 200,
    'models list response time < 500ms': (r) => r.timings.duration < 500,
  });
  errorRate.add(response.status !== 200);

  // Test 6: Create inference job (POST request)
  const jobData = {
    model_id: 'model_1234567890abcdef1234567890abcdef1234567890abcdef1234567890',
    input_data: 'dGVzdCBpbnB1dCBkYXRh', // base64 encoded "test input data"
    sla: {
      max_latency_ms: 1000,
      max_cost_ipn: 1.0
    },
    privacy: 'public',
    bidding_window_seconds: 300,
    max_price_ipn: 0.5,
    escrow_amount_ipn: 0.1
  };

  response = http.post(`${BASE_URL}/jobs`, JSON.stringify(jobData), {
    headers: {
      'Content-Type': 'application/json',
      'X-Wallet-Address': testWallet
    }
  });
  check(response, {
    'create job status is 200 or 201': (r) => r.status === 200 || r.status === 201,
    'create job response time < 1000ms': (r) => r.timings.duration < 1000,
  });
  errorRate.add(response.status !== 200 && response.status !== 201);

  // Test 7: Domain verification challenge
  response = http.get(`${BASE_URL}/domains/${testDomain}/verify`);
  check(response, {
    'domain verify status is 200': (r) => r.status === 200,
    'domain verify response time < 500ms': (r) => r.timings.duration < 500,
  });
  errorRate.add(response.status !== 200);

  // Test 8: Get system status
  response = http.get(`${BASE_URL}/status`);
  check(response, {
    'system status is 200': (r) => r.status === 200,
    'system status response time < 200ms': (r) => r.timings.duration < 200,
  });
  errorRate.add(response.status !== 200);

  // Test 9: Get metrics
  response = http.get(`${BASE_URL}/metrics`);
  check(response, {
    'metrics status is 200': (r) => r.status === 200,
    'metrics response time < 500ms': (r) => r.timings.duration < 500,
  });
  errorRate.add(response.status !== 200);

  // Test 10: Get consensus round info
  response = http.get(`${BASE_URL}/consensus/round`);
  check(response, {
    'consensus round status is 200': (r) => r.status === 200,
    'consensus round response time < 500ms': (r) => r.timings.duration < 500,
  });
  errorRate.add(response.status !== 200);

  sleep(1); // Wait 1 second between requests
}

export function handleSummary(data) {
  return {
    'load-test-results.json': JSON.stringify(data, null, 2),
    'load-test-summary.html': htmlReport(data),
  };
}

function htmlReport(data) {
  return `
    <!DOCTYPE html>
    <html>
    <head>
        <title>IPPAN Load Test Results</title>
        <style>
            body { font-family: Arial, sans-serif; margin: 20px; }
            .metric { margin: 10px 0; padding: 10px; border: 1px solid #ddd; }
            .pass { background-color: #d4edda; }
            .fail { background-color: #f8d7da; }
        </style>
    </head>
    <body>
        <h1>IPPAN Load Test Results</h1>
        <div class="metric">
            <h3>Test Summary</h3>
            <p><strong>Duration:</strong> ${data.state.testRunDurationMs / 1000}s</p>
            <p><strong>Total Requests:</strong> ${data.metrics.http_reqs.values.count}</p>
            <p><strong>Failed Requests:</strong> ${data.metrics.http_req_failed.values.count}</p>
            <p><strong>Error Rate:</strong> ${(data.metrics.http_req_failed.values.rate * 100).toFixed(2)}%</p>
        </div>
        <div class="metric">
            <h3>Response Times</h3>
            <p><strong>Average:</strong> ${data.metrics.http_req_duration.values.avg.toFixed(2)}ms</p>
            <p><strong>95th Percentile:</strong> ${data.metrics.http_req_duration.values['p(95)'].toFixed(2)}ms</p>
            <p><strong>99th Percentile:</strong> ${data.metrics.http_req_duration.values['p(99)'].toFixed(2)}ms</p>
        </div>
        <div class="metric">
            <h3>Throughput</h3>
            <p><strong>Requests per second:</strong> ${data.metrics.http_reqs.values.rate.toFixed(2)}</p>
            <p><strong>Data received:</strong> ${(data.metrics.data_received.values.count / 1024 / 1024).toFixed(2)} MB</p>
            <p><strong>Data sent:</strong> ${(data.metrics.data_sent.values.count / 1024 / 1024).toFixed(2)} MB</p>
        </div>
    </body>
    </html>
  `;
}
