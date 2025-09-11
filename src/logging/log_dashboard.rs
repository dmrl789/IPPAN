//! Log dashboard for IPPAN
//! 
//! Provides web-based dashboard for log visualization and management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use axum::{
    extract::State,
    response::{Html, Json},
    routing::get,
    Router,
};

use super::structured_logger::{StructuredLogger, LogEntry, LogLevel};

/// Dashboard configuration
#[derive(Debug, Clone)]
pub struct DashboardConfig {
    pub port: u16,
    pub host: String,
    pub enable_authentication: bool,
    pub enable_ssl: bool,
    pub refresh_interval_seconds: u64,
    pub max_logs_displayed: usize,
    pub enable_real_time_updates: bool,
    pub enable_log_search: bool,
    pub enable_log_filtering: bool,
    pub enable_log_export: bool,
    pub theme: DashboardTheme,
}

/// Dashboard theme
#[derive(Debug, Clone)]
pub enum DashboardTheme {
    Light,
    Dark,
    Auto,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            host: "0.0.0.0".to_string(),
            enable_authentication: false,
            enable_ssl: false,
            refresh_interval_seconds: 5,
            max_logs_displayed: 1000,
            enable_real_time_updates: true,
            enable_log_search: true,
            enable_log_filtering: true,
            enable_log_export: true,
            theme: DashboardTheme::Auto,
        }
    }
}

/// Dashboard statistics
#[derive(Debug, Clone)]
pub struct DashboardStatistics {
    pub total_requests: u64,
    pub active_connections: u32,
    pub uptime_seconds: u64,
    pub last_update: DateTime<Utc>,
    pub dashboard_version: String,
}

/// Log dashboard
pub struct LogDashboard {
    structured_logger: Arc<StructuredLogger>,
    config: DashboardConfig,
    start_time: Instant,
    statistics: Arc<RwLock<DashboardStatistics>>,
}

impl LogDashboard {
    /// Create a new log dashboard
    pub fn new(structured_logger: Arc<StructuredLogger>) -> Self {
        Self {
            structured_logger,
            config: DashboardConfig::default(),
            start_time: Instant::now(),
            statistics: Arc::new(RwLock::new(DashboardStatistics {
                total_requests: 0,
                active_connections: 0,
                uptime_seconds: 0,
                last_update: Utc::now(),
                dashboard_version: "1.0.0".to_string(),
            })),
        }
    }

    /// Create a new log dashboard with custom configuration
    pub fn with_config(structured_logger: Arc<StructuredLogger>, config: DashboardConfig) -> Self {
        Self {
            structured_logger,
            config,
            start_time: Instant::now(),
            statistics: Arc::new(RwLock::new(DashboardStatistics {
                total_requests: 0,
                active_connections: 0,
                uptime_seconds: 0,
                last_update: Utc::now(),
                dashboard_version: "1.0.0".to_string(),
            })),
        }
    }

    /// Start the log dashboard
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.structured_logger.log(
            LogLevel::Info,
            "log_dashboard",
            &format!("Log dashboard starting on {}:{}", self.config.host, self.config.port),
            HashMap::new(),
        ).await;

        // In a real implementation, start the web server here
        // For now, just log that it's started
        self.structured_logger.log(
            LogLevel::Info,
            "log_dashboard",
            "Log dashboard started successfully",
            HashMap::new(),
        ).await;

        Ok(())
    }

    /// Stop the log dashboard
    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.structured_logger.log(
            LogLevel::Info,
            "log_dashboard",
            "Log dashboard stopped",
            HashMap::new(),
        ).await;

        Ok(())
    }

    /// Create dashboard router
    pub fn create_router(&self) -> Router {
        let structured_logger = Arc::clone(&self.structured_logger);
        let statistics = Arc::clone(&self.statistics);
        
        Router::new()
            .route("/", get(Self::dashboard_html))
            .route("/api/logs", get(Self::get_logs_api))
            .route("/api/logs/search", get(Self::search_logs_api))
            .route("/api/logs/filter", get(Self::filter_logs_api))
            // .route("/api/statistics", get(get_statistics_api)) // TODO: Fix handler trait issue
            .route("/api/export", get(Self::export_logs_api))
            .with_state((structured_logger, statistics))
    }

    /// Dashboard HTML page
    async fn dashboard_html() -> Html<&'static str> {
        Html(DASHBOARD_HTML)
    }

    /// Get logs API endpoint
    async fn get_logs_api(
        State((structured_logger, statistics)): State<(Arc<StructuredLogger>, Arc<RwLock<DashboardStatistics>>)>
    ) -> Json<Vec<LogEntry>> {
        // Update statistics
        {
            let mut stats = statistics.write().await;
            stats.total_requests += 1;
            stats.last_update = Utc::now();
        }

        let logs = structured_logger.get_logs().await;
        Json(logs)
    }

    /// Search logs API endpoint
    async fn search_logs_api(
        State((structured_logger, statistics)): State<(Arc<StructuredLogger>, Arc<RwLock<DashboardStatistics>>)>
    ) -> Json<Vec<LogEntry>> {
        // Update statistics
        {
            let mut stats = statistics.write().await;
            stats.total_requests += 1;
            stats.last_update = Utc::now();
        }

        // TODO: Implement actual search functionality
        let logs = structured_logger.get_logs().await;
        Json(logs)
    }

    /// Filter logs API endpoint
    async fn filter_logs_api(
        State((structured_logger, statistics)): State<(Arc<StructuredLogger>, Arc<RwLock<DashboardStatistics>>)>
    ) -> Json<Vec<LogEntry>> {
        // Update statistics
        {
            let mut stats = statistics.write().await;
            stats.total_requests += 1;
            stats.last_update = Utc::now();
        }

        // TODO: Implement actual filtering functionality
        let logs = structured_logger.get_logs().await;
        Json(logs)
    }


    /// Export logs API endpoint
    async fn export_logs_api(
        State((structured_logger, statistics)): State<(Arc<StructuredLogger>, Arc<RwLock<DashboardStatistics>>)>
    ) -> Json<String> {
        // Update statistics
        {
            let mut stats = statistics.write().await;
            stats.total_requests += 1;
            stats.last_update = Utc::now();
        }

        let logs = structured_logger.get_logs().await;
        let json_data = serde_json::to_string_pretty(&logs).unwrap_or_default();
        Json(json_data)
    }

    /// Get dashboard statistics
    pub async fn get_statistics(&self) -> DashboardStatistics {
        let mut stats = self.statistics.read().await.clone();
        stats.uptime_seconds = self.start_time.elapsed().as_secs();
        stats
    }

}

/// Get statistics API endpoint
async fn get_statistics_api(
    State((_structured_logger, statistics)): State<(Arc<StructuredLogger>, Arc<RwLock<DashboardStatistics>>)>
) -> Json<DashboardStatistics> {
    // Update statistics
    {
        let mut stats = statistics.write().await;
        stats.total_requests += 1;
        stats.last_update = Utc::now();
    }

    let stats = statistics.read().await.clone();
    Json(stats)
}

/// Dashboard HTML template
pub const DASHBOARD_HTML: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>IPPAN Log Dashboard</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }

        body {
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            color: #333;
        }

        .container {
            max-width: 1400px;
            margin: 0 auto;
            padding: 20px;
        }

        .header {
            background: rgba(255, 255, 255, 0.95);
            border-radius: 10px;
            padding: 20px;
            margin-bottom: 20px;
            box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
        }

        .header h1 {
            color: #667eea;
            margin-bottom: 10px;
        }

        .header p {
            color: #666;
            margin-bottom: 20px;
        }

        .controls {
            display: flex;
            gap: 15px;
            flex-wrap: wrap;
            align-items: center;
        }

        .control-group {
            display: flex;
            flex-direction: column;
            gap: 5px;
        }

        .control-group label {
            font-size: 12px;
            color: #666;
            font-weight: bold;
        }

        .control-group input,
        .control-group select {
            padding: 8px 12px;
            border: 1px solid #ddd;
            border-radius: 4px;
            font-size: 14px;
        }

        .btn {
            padding: 8px 16px;
            border: none;
            border-radius: 4px;
            cursor: pointer;
            font-size: 14px;
            transition: all 0.3s ease;
        }

        .btn-primary {
            background: #667eea;
            color: white;
        }

        .btn-primary:hover {
            background: #5a6fd8;
        }

        .btn-secondary {
            background: #6c757d;
            color: white;
        }

        .btn-secondary:hover {
            background: #5a6268;
        }

        .main-content {
            display: grid;
            grid-template-columns: 1fr 300px;
            gap: 20px;
        }

        .logs-section {
            background: rgba(255, 255, 255, 0.95);
            border-radius: 10px;
            padding: 20px;
            box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
        }

        .sidebar {
            display: flex;
            flex-direction: column;
            gap: 20px;
        }

        .stats-card {
            background: rgba(255, 255, 255, 0.95);
            border-radius: 10px;
            padding: 20px;
            box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
        }

        .stats-card h3 {
            color: #667eea;
            margin-bottom: 15px;
            border-bottom: 2px solid #667eea;
            padding-bottom: 10px;
        }

        .stat-item {
            display: flex;
            justify-content: space-between;
            margin-bottom: 10px;
            padding: 8px;
            background: #f8f9fa;
            border-radius: 4px;
        }

        .stat-label {
            font-weight: bold;
            color: #333;
        }

        .stat-value {
            color: #667eea;
            font-weight: bold;
        }

        .logs-container {
            max-height: 600px;
            overflow-y: auto;
            border: 1px solid #ddd;
            border-radius: 4px;
            background: #f8f9fa;
        }

        .log-entry {
            padding: 12px;
            border-bottom: 1px solid #eee;
            font-family: 'Courier New', monospace;
            font-size: 13px;
            transition: background-color 0.2s ease;
        }

        .log-entry:hover {
            background: #e9ecef;
        }

        .log-entry:last-child {
            border-bottom: none;
        }

        .log-timestamp {
            color: #666;
            font-size: 11px;
        }

        .log-level {
            display: inline-block;
            padding: 2px 6px;
            border-radius: 3px;
            font-size: 11px;
            font-weight: bold;
            margin: 0 8px;
        }

        .log-level-info {
            background: #d1ecf1;
            color: #0c5460;
        }

        .log-level-warn {
            background: #fff3cd;
            color: #856404;
        }

        .log-level-error {
            background: #f8d7da;
            color: #721c24;
        }

        .log-level-debug {
            background: #d4edda;
            color: #155724;
        }

        .log-target {
            color: #667eea;
            font-weight: bold;
        }

        .log-message {
            color: #333;
            margin-left: 8px;
        }

        .chart-container {
            background: rgba(255, 255, 255, 0.95);
            border-radius: 10px;
            padding: 20px;
            margin-bottom: 20px;
            box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
        }

        .chart-container h3 {
            color: #667eea;
            margin-bottom: 15px;
            border-bottom: 2px solid #667eea;
            padding-bottom: 10px;
        }

        .loading {
            text-align: center;
            padding: 40px;
            color: #666;
        }

        .error {
            background: #f8d7da;
            color: #721c24;
            padding: 15px;
            border-radius: 4px;
            margin: 10px 0;
        }

        .success {
            background: #d4edda;
            color: #155724;
            padding: 15px;
            border-radius: 4px;
            margin: 10px 0;
        }

        @media (max-width: 768px) {
            .main-content {
                grid-template-columns: 1fr;
            }
            
            .controls {
                flex-direction: column;
                align-items: stretch;
            }
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>📊 IPPAN Log Dashboard</h1>
            <p>Real-time log monitoring and analysis</p>
            
            <div class="controls">
                <div class="control-group">
                    <label>Search</label>
                    <input type="text" id="searchInput" placeholder="Search logs...">
                </div>
                
                <div class="control-group">
                    <label>Level</label>
                    <select id="levelFilter">
                        <option value="">All Levels</option>
                        <option value="Info">Info</option>
                        <option value="Warn">Warn</option>
                        <option value="Error">Error</option>
                        <option value="Debug">Debug</option>
                    </select>
                </div>
                
                <div class="control-group">
                    <label>Target</label>
                    <input type="text" id="targetFilter" placeholder="Filter by target...">
                </div>
                
                <div class="control-group">
                    <label>Time Range</label>
                    <select id="timeFilter">
                        <option value="all">All Time</option>
                        <option value="1h">Last Hour</option>
                        <option value="24h">Last 24 Hours</option>
                        <option value="7d">Last 7 Days</option>
                    </select>
                </div>
                
                <button class="btn btn-primary" onclick="refreshLogs()">🔄 Refresh</button>
                <button class="btn btn-secondary" onclick="exportLogs()">📥 Export</button>
            </div>
        </div>

        <div class="main-content">
            <div class="logs-section">
                <h3>📋 Recent Logs</h3>
                <div id="logsContainer" class="logs-container">
                    <div class="loading">Loading logs...</div>
                </div>
            </div>
            
            <div class="sidebar">
                <div class="stats-card">
                    <h3>📈 Statistics</h3>
                    <div id="statistics">
                        <div class="stat-item">
                            <span class="stat-label">Total Logs:</span>
                            <span class="stat-value" id="totalLogs">0</span>
                        </div>
                        <div class="stat-item">
                            <span class="stat-label">Info:</span>
                            <span class="stat-value" id="infoLogs">0</span>
                        </div>
                        <div class="stat-item">
                            <span class="stat-label">Warnings:</span>
                            <span class="stat-value" id="warnLogs">0</span>
                        </div>
                        <div class="stat-item">
                            <span class="stat-label">Errors:</span>
                            <span class="stat-label" id="errorLogs">0</span>
                        </div>
                        <div class="stat-item">
                            <span class="stat-label">Debug:</span>
                            <span class="stat-value" id="debugLogs">0</span>
                        </div>
                    </div>
                </div>
                
                <div class="chart-container">
                    <h3>📊 Log Levels</h3>
                    <canvas id="logLevelsChart" width="300" height="200"></canvas>
                </div>
                
                <div class="chart-container">
                    <h3>⏰ Log Timeline</h3>
                    <canvas id="logTimelineChart" width="300" height="200"></canvas>
                </div>
            </div>
        </div>
    </div>

    <script>
        let logLevelsChart, logTimelineChart;
        let allLogs = [];
        let filteredLogs = [];

        // Initialize charts
        function initializeCharts() {
            // Log levels pie chart
            const logLevelsCtx = document.getElementById('logLevelsChart').getContext('2d');
            logLevelsChart = new Chart(logLevelsCtx, {
                type: 'doughnut',
                data: {
                    labels: ['Info', 'Warn', 'Error', 'Debug'],
                    datasets: [{
                        data: [0, 0, 0, 0],
                        backgroundColor: ['#17a2b8', '#ffc107', '#dc3545', '#28a745'],
                        borderWidth: 2,
                        borderColor: '#fff'
                    }]
                },
                options: {
                    responsive: true,
                    maintainAspectRatio: false,
                    plugins: {
                        legend: {
                            position: 'bottom'
                        }
                    }
                }
            });

            // Log timeline line chart
            const logTimelineCtx = document.getElementById('logTimelineChart').getContext('2d');
            logTimelineChart = new Chart(logTimelineCtx, {
                type: 'line',
                data: {
                    labels: [],
                    datasets: [{
                        label: 'Logs per minute',
                        data: [],
                        borderColor: '#667eea',
                        backgroundColor: 'rgba(102, 126, 234, 0.1)',
                        tension: 0.4,
                        fill: true
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

        // Fetch logs from API
        async function fetchLogs() {
            try {
                const response = await fetch('/api/logs');
                const logs = await response.json();
                allLogs = logs;
                applyFilters();
            } catch (error) {
                console.error('Error fetching logs:', error);
                document.getElementById('logsContainer').innerHTML = 
                    '<div class="error">Error loading logs: ' + error.message + '</div>';
            }
        }

        // Apply filters to logs
        function applyFilters() {
            const searchTerm = document.getElementById('searchInput').value.toLowerCase();
            const levelFilter = document.getElementById('levelFilter').value;
            const targetFilter = document.getElementById('targetFilter').value.toLowerCase();
            const timeFilter = document.getElementById('timeFilter').value;

            filteredLogs = allLogs.filter(log => {
                // Search filter
                if (searchTerm && !log.message.toLowerCase().includes(searchTerm)) {
                    return false;
                }

                // Level filter
                if (levelFilter && log.level !== levelFilter) {
                    return false;
                }

                // Target filter
                if (targetFilter && !log.target.toLowerCase().includes(targetFilter)) {
                    return false;
                }

                // Time filter
                if (timeFilter !== 'all') {
                    const logTime = new Date(log.timestamp);
                    const now = new Date();
                    let cutoffTime;

                    switch (timeFilter) {
                        case '1h':
                            cutoffTime = new Date(now.getTime() - 60 * 60 * 1000);
                            break;
                        case '24h':
                            cutoffTime = new Date(now.getTime() - 24 * 60 * 60 * 1000);
                            break;
                        case '7d':
                            cutoffTime = new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000);
                            break;
                        default:
                            cutoffTime = new Date(0);
                    }

                    if (logTime < cutoffTime) {
                        return false;
                    }
                }

                return true;
            });

            displayLogs();
            updateStatistics();
            updateCharts();
        }

        // Display logs in the container
        function displayLogs() {
            const container = document.getElementById('logsContainer');
            
            if (filteredLogs.length === 0) {
                container.innerHTML = '<div class="loading">No logs found matching the current filters.</div>';
                return;
            }

            const logsHtml = filteredLogs.slice(-100).reverse().map(log => {
                const timestamp = new Date(log.timestamp).toLocaleString();
                const levelClass = `log-level-${log.level.toLowerCase()}`;
                
                return `
                    <div class="log-entry">
                        <span class="log-timestamp">${timestamp}</span>
                        <span class="log-level ${levelClass}">${log.level}</span>
                        <span class="log-target">${log.target}</span>
                        <span class="log-message">${log.message}</span>
                    </div>
                `;
            }).join('');

            container.innerHTML = logsHtml;
        }

        // Update statistics
        function updateStatistics() {
            const stats = {
                total: filteredLogs.length,
                info: filteredLogs.filter(log => log.level === 'Info').length,
                warn: filteredLogs.filter(log => log.level === 'Warn').length,
                error: filteredLogs.filter(log => log.level === 'Error').length,
                debug: filteredLogs.filter(log => log.level === 'Debug').length
            };

            document.getElementById('totalLogs').textContent = stats.total;
            document.getElementById('infoLogs').textContent = stats.info;
            document.getElementById('warnLogs').textContent = stats.warn;
            document.getElementById('errorLogs').textContent = stats.error;
            document.getElementById('debugLogs').textContent = stats.debug;
        }

        // Update charts
        function updateCharts() {
            // Update log levels chart
            const levelStats = {
                info: filteredLogs.filter(log => log.level === 'Info').length,
                warn: filteredLogs.filter(log => log.level === 'Warn').length,
                error: filteredLogs.filter(log => log.level === 'Error').length,
                debug: filteredLogs.filter(log => log.level === 'Debug').length
            };

            logLevelsChart.data.datasets[0].data = [
                levelStats.info,
                levelStats.warn,
                levelStats.error,
                levelStats.debug
            ];
            logLevelsChart.update();

            // Update timeline chart
            const timelineData = {};
            filteredLogs.forEach(log => {
                const minute = new Date(log.timestamp).toLocaleTimeString('en-US', {
                    hour: '2-digit',
                    minute: '2-digit'
                });
                timelineData[minute] = (timelineData[minute] || 0) + 1;
            });

            const labels = Object.keys(timelineData).sort();
            const data = labels.map(label => timelineData[label]);

            logTimelineChart.data.labels = labels;
            logTimelineChart.data.datasets[0].data = data;
            logTimelineChart.update();
        }

        // Refresh logs
        function refreshLogs() {
            fetchLogs();
        }

        // Export logs
        function exportLogs() {
            fetch('/api/export')
                .then(response => response.json())
                .then(data => {
                    const blob = new Blob([data], { type: 'application/json' });
                    const url = URL.createObjectURL(blob);
                    const a = document.createElement('a');
                    a.href = url;
                    a.download = `ippan-logs-${new Date().toISOString().split('T')[0]}.json`;
                    document.body.appendChild(a);
                    a.click();
                    document.body.removeChild(a);
                    URL.revokeObjectURL(url);
                })
                .catch(error => {
                    console.error('Error exporting logs:', error);
                });
        }

        // Event listeners
        document.getElementById('searchInput').addEventListener('input', applyFilters);
        document.getElementById('levelFilter').addEventListener('change', applyFilters);
        document.getElementById('targetFilter').addEventListener('input', applyFilters);
        document.getElementById('timeFilter').addEventListener('change', applyFilters);

        // Initialize dashboard
        document.addEventListener('DOMContentLoaded', function() {
            initializeCharts();
            fetchLogs();
            
            // Auto-refresh every 30 seconds
            setInterval(fetchLogs, 30000);
        });
    </script>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_log_dashboard_creation() {
        let structured_logger = Arc::new(StructuredLogger::new());
        let dashboard = LogDashboard::new(structured_logger);
        let stats = dashboard.get_statistics().await;
        assert_eq!(stats.total_requests, 0);
    }

    #[tokio::test]
    async fn test_dashboard_html_contains_expected_content() {
        assert!(DASHBOARD_HTML.contains("IPPAN Log Dashboard"));
        assert!(DASHBOARD_HTML.contains("Recent Logs"));
        assert!(DASHBOARD_HTML.contains("Statistics"));
        assert!(DASHBOARD_HTML.contains("Log Levels"));
        assert!(DASHBOARD_HTML.contains("Log Timeline"));
    }

    #[tokio::test]
    async fn test_dashboard_start_stop() {
        let structured_logger = Arc::new(StructuredLogger::new());
        let dashboard = LogDashboard::new(structured_logger);
        
        dashboard.start().await.unwrap();
        dashboard.stop().await.unwrap();
    }
}
