//! Monitoring dashboard for IPPAN
//! 
//! Provides real-time metrics visualization and health monitoring endpoints

use crate::monitoring::metrics::{MetricsCollector, PerformanceMetrics, HealthStatus};
use axum::{
    extract::State,
    response::{Html, Json},
    routing::get,
    Router,
};
use serde_json::Value;
use std::sync::Arc;

/// Dashboard server for monitoring and metrics
pub struct DashboardServer {
    metrics_collector: Arc<MetricsCollector>,
}

impl DashboardServer {
    /// Create a new dashboard server
    pub fn new(metrics_collector: Arc<MetricsCollector>) -> Self {
        Self {
            metrics_collector,
        }
    }

    /// Create the dashboard router
    pub fn create_router(&self) -> Router {
        let metrics_collector = Arc::clone(&self.metrics_collector);
        
        Router::new()
            .route("/", get(Self::dashboard_html))
            .route("/metrics", get(Self::get_metrics))
            .route("/performance", get(Self::get_performance))
            .route("/health", get(Self::get_health))
            .route("/prometheus", get(Self::get_prometheus_metrics))
            .with_state(metrics_collector)
    }

    /// Dashboard HTML page
    async fn dashboard_html() -> Html<&'static str> {
        Html(include_str!("dashboard.html"))
    }

    /// Get all metrics as JSON
    async fn get_metrics(State(metrics_collector): State<Arc<MetricsCollector>>) -> Json<Value> {
        let metrics = metrics_collector.get_metrics().await;
        Json(serde_json::to_value(metrics).unwrap())
    }

    /// Get performance metrics as JSON
    async fn get_performance(State(metrics_collector): State<Arc<MetricsCollector>>) -> Json<PerformanceMetrics> {
        let performance = metrics_collector.get_performance_metrics().await;
        Json(performance)
    }

    /// Get health status as JSON
    async fn get_health(State(metrics_collector): State<Arc<MetricsCollector>>) -> Json<HealthStatus> {
        let health = metrics_collector.get_health_status().await;
        Json(health)
    }

    /// Get metrics in Prometheus format
    async fn get_prometheus_metrics(State(metrics_collector): State<Arc<MetricsCollector>>) -> String {
        metrics_collector.get_prometheus_metrics().await
    }
}

/// Dashboard HTML template
pub const DASHBOARD_HTML: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>IPPAN Monitoring Dashboard</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <style>
        body {
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            margin: 0;
            padding: 20px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
        }
        .container {
            max-width: 1400px;
            margin: 0 auto;
            background: white;
            border-radius: 10px;
            box-shadow: 0 10px 30px rgba(0,0,0,0.2);
            overflow: hidden;
        }
        .header {
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 20px;
            text-align: center;
        }
        .header h1 {
            margin: 0;
            font-size: 2.5em;
        }
        .header p {
            margin: 10px 0 0 0;
            opacity: 0.9;
        }
        .content {
            padding: 30px;
        }
        .grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }
        .card {
            border: 1px solid #e0e0e0;
            border-radius: 8px;
            padding: 20px;
            background: #f9f9f9;
        }
        .card h3 {
            margin: 0 0 15px 0;
            color: #333;
            border-bottom: 2px solid #667eea;
            padding-bottom: 10px;
        }
        .metric {
            display: flex;
            justify-content: space-between;
            margin-bottom: 10px;
            padding: 8px;
            background: white;
            border-radius: 4px;
            border-left: 4px solid #667eea;
        }
        .metric-name {
            font-weight: bold;
            color: #333;
        }
        .metric-value {
            color: #667eea;
            font-weight: bold;
        }
        .health-indicator {
            display: inline-block;
            width: 12px;
            height: 12px;
            border-radius: 50%;
            margin-right: 8px;
        }
        .health-healthy { background: #28a745; }
        .health-degraded { background: #ffc107; }
        .health-unhealthy { background: #dc3545; }
        .health-unknown { background: #6c757d; }
        .chart-container {
            position: relative;
            height: 300px;
            margin-top: 20px;
        }
        .refresh-button {
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            border: none;
            padding: 10px 20px;
            border-radius: 5px;
            cursor: pointer;
            font-size: 14px;
            margin-bottom: 20px;
        }
        .refresh-button:hover {
            opacity: 0.9;
        }
        .status-bar {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 15px;
            background: #f8f9fa;
            border-radius: 5px;
            margin-bottom: 20px;
        }
        .uptime {
            font-size: 14px;
            color: #666;
        }
        .last-update {
            font-size: 12px;
            color: #999;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🚀 IPPAN Monitoring Dashboard</h1>
            <p>Real-time system metrics and health monitoring</p>
        </div>
        
        <div class="content">
            <div class="status-bar">
                <div>
                    <span class="health-indicator" id="overall-health"></span>
                    <span id="overall-status">Loading...</span>
                </div>
                <div class="uptime">
                    Uptime: <span id="uptime">Loading...</span>
                </div>
                <div class="last-update">
                    Last Update: <span id="last-update">Loading...</span>
                </div>
                <button class="refresh-button" onclick="refreshData()">🔄 Refresh</button>
            </div>

            <div class="grid">
                <!-- System Health -->
                <div class="card">
                    <h3>System Health</h3>
                    <div id="health-metrics">
                        <div class="metric">
                            <span class="metric-name">Overall Status</span>
                            <span class="metric-value" id="health-status">Loading...</span>
                        </div>
                        <div class="metric">
                            <span class="metric-name">Components</span>
                            <span class="metric-value" id="component-count">Loading...</span>
                        </div>
                        <div class="metric">
                            <span class="metric-name">Last Check</span>
                            <span class="metric-value" id="last-check">Loading...</span>
                        </div>
                    </div>
                </div>

                <!-- API Performance -->
                <div class="card">
                    <h3>API Performance</h3>
                    <div id="api-metrics">
                        <div class="metric">
                            <span class="metric-name">Total Requests</span>
                            <span class="metric-value" id="total-requests">Loading...</span>
                        </div>
                        <div class="metric">
                            <span class="metric-name">Success Rate</span>
                            <span class="metric-value" id="success-rate">Loading...</span>
                        </div>
                        <div class="metric">
                            <span class="metric-name">Avg Response Time</span>
                            <span class="metric-value" id="avg-response-time">Loading...</span>
                        </div>
                        <div class="metric">
                            <span class="metric-name">Requests/sec</span>
                            <span class="metric-value" id="requests-per-sec">Loading...</span>
                        </div>
                    </div>
                </div>

                <!-- Storage Performance -->
                <div class="card">
                    <h3>Storage Performance</h3>
                    <div id="storage-metrics">
                        <div class="metric">
                            <span class="metric-name">Files Stored</span>
                            <span class="metric-value" id="files-stored">Loading...</span>
                        </div>
                        <div class="metric">
                            <span class="metric-name">Files Retrieved</span>
                            <span class="metric-value" id="files-retrieved">Loading...</span>
                        </div>
                        <div class="metric">
                            <span class="metric-name">Storage Used</span>
                            <span class="metric-value" id="storage-used">Loading...</span>
                        </div>
                        <div class="metric">
                            <span class="metric-name">Avg Latency</span>
                            <span class="metric-value" id="storage-latency">Loading...</span>
                        </div>
                    </div>
                </div>

                <!-- Network Performance -->
                <div class="card">
                    <h3>Network Performance</h3>
                    <div id="network-metrics">
                        <div class="metric">
                            <span class="metric-name">Connected Peers</span>
                            <span class="metric-value" id="connected-peers">Loading...</span>
                        </div>
                        <div class="metric">
                            <span class="metric-name">Messages Sent</span>
                            <span class="metric-value" id="messages-sent">Loading...</span>
                        </div>
                        <div class="metric">
                            <span class="metric-name">Messages Received</span>
                            <span class="metric-value" id="messages-received">Loading...</span>
                        </div>
                        <div class="metric">
                            <span class="metric-name">Network Latency</span>
                            <span class="metric-value" id="network-latency">Loading...</span>
                        </div>
                    </div>
                </div>

                <!-- Consensus Performance -->
                <div class="card">
                    <h3>Consensus Performance</h3>
                    <div id="consensus-metrics">
                        <div class="metric">
                            <span class="metric-name">Current Round</span>
                            <span class="metric-value" id="current-round">Loading...</span>
                        </div>
                        <div class="metric">
                            <span class="metric-name">Blocks Created</span>
                            <span class="metric-value" id="blocks-created">Loading...</span>
                        </div>
                        <div class="metric">
                            <span class="metric-name">Transactions Processed</span>
                            <span class="metric-value" id="transactions-processed">Loading...</span>
                        </div>
                        <div class="metric">
                            <span class="metric-name">Participation Rate</span>
                            <span class="metric-value" id="participation-rate">Loading...</span>
                        </div>
                    </div>
                </div>

                <!-- System Resources -->
                <div class="card">
                    <h3>System Resources</h3>
                    <div id="system-metrics">
                        <div class="metric">
                            <span class="metric-name">Memory Usage</span>
                            <span class="metric-value" id="memory-usage">Loading...</span>
                        </div>
                        <div class="metric">
                            <span class="metric-name">CPU Usage</span>
                            <span class="metric-value" id="cpu-usage">Loading...</span>
                        </div>
                        <div class="metric">
                            <span class="metric-name">Disk Usage</span>
                            <span class="metric-value" id="disk-usage">Loading...</span>
                        </div>
                        <div class="metric">
                            <span class="metric-name">Thread Count</span>
                            <span class="metric-value" id="thread-count">Loading...</span>
                        </div>
                    </div>
                </div>
            </div>

            <!-- Charts -->
            <div class="grid">
                <div class="card">
                    <h3>API Requests Over Time</h3>
                    <div class="chart-container">
                        <canvas id="apiChart"></canvas>
                    </div>
                </div>
                
                <div class="card">
                    <h3>Storage Operations Over Time</h3>
                    <div class="chart-container">
                        <canvas id="storageChart"></canvas>
                    </div>
                </div>
            </div>
        </div>
    </div>

    <script>
        let apiChart, storageChart;
        let apiData = { labels: [], datasets: [] };
        let storageData = { labels: [], datasets: [] };

        function formatBytes(bytes) {
            const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
            if (bytes === 0) return '0 B';
            const i = Math.floor(Math.log(bytes) / Math.log(1024));
            return Math.round(bytes / Math.pow(1024, i) * 100) / 100 + ' ' + sizes[i];
        }

        function formatDuration(seconds) {
            const hours = Math.floor(seconds / 3600);
            const minutes = Math.floor((seconds % 3600) / 60);
            const secs = seconds % 60;
            return `${hours}h ${minutes}m ${secs}s`;
        }

        function getHealthClass(status) {
            switch(status.toLowerCase()) {
                case 'healthy': return 'health-healthy';
                case 'degraded': return 'health-degraded';
                case 'unhealthy': return 'health-unhealthy';
                default: return 'health-unknown';
            }
        }

        async function refreshData() {
            try {
                // Fetch performance metrics
                const performanceResponse = await fetch('/performance');
                const performance = await performanceResponse.json();
                
                // Fetch health status
                const healthResponse = await fetch('/health');
                const health = await healthResponse.json();
                
                updateMetrics(performance, health);
                updateCharts(performance);
                
                document.getElementById('last-update').textContent = new Date().toLocaleTimeString();
            } catch (error) {
                console.error('Error fetching metrics:', error);
            }
        }

        function updateMetrics(performance, health) {
            // Update overall status
            const overallHealth = document.getElementById('overall-health');
            const overallStatus = document.getElementById('overall-status');
            overallHealth.className = `health-indicator ${getHealthClass(health.overall_status)}`;
            overallStatus.textContent = health.overall_status;
            
            // Update uptime
            document.getElementById('uptime').textContent = formatDuration(health.uptime_seconds);
            
            // Update health metrics
            document.getElementById('health-status').textContent = health.overall_status;
            document.getElementById('component-count').textContent = Object.keys(health.components).length;
            document.getElementById('last-check').textContent = new Date(health.last_check * 1000).toLocaleTimeString();
            
            // Update API metrics
            const api = performance.api_operations;
            document.getElementById('total-requests').textContent = api.total_requests.toLocaleString();
            document.getElementById('success-rate').textContent = api.total_requests > 0 ? 
                ((api.successful_requests / api.total_requests) * 100).toFixed(1) + '%' : '0%';
            document.getElementById('avg-response-time').textContent = api.average_response_time_ms.toFixed(2) + 'ms';
            document.getElementById('requests-per-sec').textContent = api.requests_per_second.toFixed(2);
            
            // Update storage metrics
            const storage = performance.storage_operations;
            document.getElementById('files-stored').textContent = storage.files_stored.toLocaleString();
            document.getElementById('files-retrieved').textContent = storage.files_retrieved.toLocaleString();
            document.getElementById('storage-used').textContent = formatBytes(storage.used_storage_bytes);
            document.getElementById('storage-latency').textContent = storage.average_storage_latency_ms.toFixed(2) + 'ms';
            
            // Update network metrics
            const network = performance.network_operations;
            document.getElementById('connected-peers').textContent = network.connected_peers;
            document.getElementById('messages-sent').textContent = network.messages_sent.toLocaleString();
            document.getElementById('messages-received').textContent = network.messages_received.toLocaleString();
            document.getElementById('network-latency').textContent = network.network_latency_ms.toFixed(2) + 'ms';
            
            // Update consensus metrics
            const consensus = performance.consensus_operations;
            document.getElementById('current-round').textContent = consensus.current_round.toLocaleString();
            document.getElementById('blocks-created').textContent = consensus.blocks_created.toLocaleString();
            document.getElementById('transactions-processed').textContent = consensus.transactions_processed.toLocaleString();
            document.getElementById('participation-rate').textContent = (consensus.consensus_participation_rate * 100).toFixed(1) + '%';
            
            // Update system metrics
            const system = performance.system_metrics;
            document.getElementById('memory-usage').textContent = formatBytes(system.memory_usage_bytes);
            document.getElementById('cpu-usage').textContent = system.cpu_usage_percent.toFixed(1) + '%';
            document.getElementById('disk-usage').textContent = formatBytes(system.disk_usage_bytes);
            document.getElementById('thread-count').textContent = system.thread_count;
        }

        function updateCharts(performance) {
            const now = new Date().toLocaleTimeString();
            
            // Update API chart
            if (!apiChart) {
                const apiCtx = document.getElementById('apiChart').getContext('2d');
                apiChart = new Chart(apiCtx, {
                    type: 'line',
                    data: {
                        labels: [],
                        datasets: [{
                            label: 'Requests/sec',
                            data: [],
                            borderColor: '#667eea',
                            backgroundColor: 'rgba(102, 126, 234, 0.1)',
                            tension: 0.4
                        }]
                    },
                    options: {
                        responsive: true,
                        maintainAspectRatio: false,
                        scales: {
                            y: {
                                beginAtZero: true
                            }
                        }
                    }
                });
            }
            
            apiChart.data.labels.push(now);
            apiChart.data.datasets[0].data.push(performance.api_operations.requests_per_second);
            
            if (apiChart.data.labels.length > 20) {
                apiChart.data.labels.shift();
                apiChart.data.datasets[0].data.shift();
            }
            
            apiChart.update();
            
            // Update storage chart
            if (!storageChart) {
                const storageCtx = document.getElementById('storageChart').getContext('2d');
                storageChart = new Chart(storageCtx, {
                    type: 'line',
                    data: {
                        labels: [],
                        datasets: [{
                            label: 'Storage Operations/sec',
                            data: [],
                            borderColor: '#764ba2',
                            backgroundColor: 'rgba(118, 75, 162, 0.1)',
                            tension: 0.4
                        }]
                    },
                    options: {
                        responsive: true,
                        maintainAspectRatio: false,
                        scales: {
                            y: {
                                beginAtZero: true
                            }
                        }
                    }
                });
            }
            
            storageChart.data.labels.push(now);
            storageChart.data.datasets[0].data.push(performance.storage_operations.storage_operations_per_second);
            
            if (storageChart.data.labels.length > 20) {
                storageChart.data.labels.shift();
                storageChart.data.datasets[0].data.shift();
            }
            
            storageChart.update();
        }

        // Initial load
        refreshData();
        
        // Auto-refresh every 5 seconds
        setInterval(refreshData, 5000);
    </script>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_server_creation() {
        let metrics_collector = Arc::new(MetricsCollector::new());
        let dashboard = DashboardServer::new(metrics_collector);
        // Note: get_metrics() is async, so we can't test it in a sync test
        // The test just verifies the dashboard server can be created
    }

    #[test]
    fn test_dashboard_html_contains_expected_content() {
        assert!(DASHBOARD_HTML.contains("IPPAN Monitoring Dashboard"));
        assert!(DASHBOARD_HTML.contains("System Health"));
        assert!(DASHBOARD_HTML.contains("API Performance"));
        assert!(DASHBOARD_HTML.contains("Storage Performance"));
    }
} 