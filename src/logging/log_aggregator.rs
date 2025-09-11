//! Log aggregator for IPPAN
//! 
//! Collects, processes, and aggregates logs from multiple sources

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::{interval, sleep};

use super::structured_logger::{StructuredLogger, LogEntry, LogLevel};

/// Aggregation configuration
#[derive(Debug, Clone)]
pub struct AggregatorConfig {
    pub aggregation_interval_seconds: u64,
    pub max_batch_size: usize,
    pub enable_real_time_aggregation: bool,
    pub enable_batch_aggregation: bool,
    pub aggregation_rules: Vec<AggregationRule>,
    pub output_formats: Vec<OutputFormat>,
    pub enable_compression: bool,
    pub enable_encryption: bool,
}

impl Default for AggregatorConfig {
    fn default() -> Self {
        Self {
            aggregation_interval_seconds: 60,
            max_batch_size: 1000,
            enable_real_time_aggregation: true,
            enable_batch_aggregation: true,
            aggregation_rules: vec![
                AggregationRule::CountByLevel,
                AggregationRule::CountByTarget,
                AggregationRule::CountByTimeWindow { window_minutes: 5 },
                AggregationRule::CountByErrorType,
                AggregationRule::PerformanceMetrics,
            ],
            output_formats: vec![OutputFormat::Json, OutputFormat::Prometheus],
            enable_compression: true,
            enable_encryption: false,
        }
    }
}

/// Aggregation rules
#[derive(Debug, Clone)]
pub enum AggregationRule {
    CountByLevel,
    CountByTarget,
    CountByTimeWindow { window_minutes: u32 },
    CountByErrorType,
    PerformanceMetrics,
    Custom { name: String, expression: String },
}

/// Output formats for aggregated data
#[derive(Debug, Clone)]
pub enum OutputFormat {
    Json,
    Prometheus,
    InfluxDB,
    Elasticsearch,
    Custom { name: String, formatter: String },
}

/// Aggregated log data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedLogData {
    pub timestamp: DateTime<Utc>,
    pub aggregation_type: String,
    pub data: HashMap<String, serde_json::Value>,
    pub metadata: AggregationMetadata,
}

/// Aggregation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationMetadata {
    pub source_logs_count: u64,
    pub aggregation_duration_ms: f64,
    pub aggregation_rule: String,
    pub data_size_bytes: u64,
    pub compression_ratio: Option<f64>,
}

/// Aggregator statistics
#[derive(Debug, Clone)]
pub struct AggregatorStatistics {
    pub total_aggregated_logs: u64,
    pub total_aggregation_operations: u64,
    pub average_aggregation_time_ms: f64,
    pub total_data_processed_bytes: u64,
    pub compression_savings_bytes: u64,
    pub uptime_seconds: u64,
    pub active_aggregation_rules: u32,
    pub output_formats_used: u32,
}

/// Log aggregator
pub struct LogAggregator {
    structured_logger: Arc<StructuredLogger>,
    config: AggregatorConfig,
    aggregated_data: Arc<RwLock<Vec<AggregatedLogData>>>,
    start_time: Instant,
    is_running: Arc<RwLock<bool>>,
}

impl LogAggregator {
    /// Create a new log aggregator
    pub fn new(structured_logger: Arc<StructuredLogger>) -> Self {
        Self {
            structured_logger,
            config: AggregatorConfig::default(),
            aggregated_data: Arc::new(RwLock::new(Vec::new())),
            start_time: Instant::now(),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Create a new log aggregator with custom configuration
    pub fn with_config(structured_logger: Arc<StructuredLogger>, config: AggregatorConfig) -> Self {
        Self {
            structured_logger,
            config,
            aggregated_data: Arc::new(RwLock::new(Vec::new())),
            start_time: Instant::now(),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the log aggregator
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);

        // Start real-time aggregation if enabled
        if self.config.enable_real_time_aggregation {
            self.start_real_time_aggregation().await;
        }

        // Start batch aggregation if enabled
        if self.config.enable_batch_aggregation {
            self.start_batch_aggregation().await;
        }

        self.structured_logger.log(
            LogLevel::Info,
            "log_aggregator",
            "Log aggregator started",
            HashMap::new(),
        ).await;

        Ok(())
    }

    /// Stop the log aggregator
    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        drop(is_running);

        self.structured_logger.log(
            LogLevel::Info,
            "log_aggregator",
            "Log aggregator stopped",
            HashMap::new(),
        ).await;

        Ok(())
    }

    /// Start real-time aggregation
    async fn start_real_time_aggregation(&self) {
        // TODO: Implement real-time aggregation loop
        // Temporarily disabled due to async/Send issues
    }

    /// Start batch aggregation
    async fn start_batch_aggregation(&self) {
        // TODO: Implement batch aggregation loop
        // Temporarily disabled due to async/Send issues
    }

    /// Perform aggregation
    async fn perform_aggregation(
        structured_logger: &Arc<StructuredLogger>,
        aggregated_data: &Arc<RwLock<Vec<AggregatedLogData>>>,
        config: &AggregatorConfig,
        aggregation_type: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        
        // Get recent logs
        let logs = structured_logger.get_logs().await;
        let recent_logs: Vec<LogEntry> = logs.into_iter()
            .filter(|log| log.timestamp > Utc::now() - chrono::Duration::minutes(5))
            .collect();

        if recent_logs.is_empty() {
            return Ok(());
        }

        // Apply aggregation rules
        for rule in &config.aggregation_rules {
            let aggregated_data_point = Self::apply_aggregation_rule(rule, &recent_logs, aggregation_type).await;
            
            let mut aggregated_data_guard = aggregated_data.write().await;
            aggregated_data_guard.push(aggregated_data_point);
            
            // Maintain size limit
            if aggregated_data_guard.len() > 10000 {
                aggregated_data_guard.remove(0);
            }
        }

        let duration = start_time.elapsed();
        
        structured_logger.log(
            LogLevel::Debug,
            "log_aggregator",
            &format!("Aggregation completed in {}ms", duration.as_millis()),
            HashMap::new(),
        ).await;

        Ok(())
    }

    /// Apply aggregation rule
    async fn apply_aggregation_rule(
        rule: &AggregationRule,
        logs: &[LogEntry],
        aggregation_type: &str,
    ) -> AggregatedLogData {
        let start_time = Instant::now();
        let mut data = HashMap::new();

        match rule {
            AggregationRule::CountByLevel => {
                let mut level_counts = HashMap::new();
                for log in logs {
                    *level_counts.entry(format!("{:?}", log.level)).or_insert(0) += 1;
                }
                data.insert("level_counts".to_string(), serde_json::to_value(level_counts).unwrap());
            }
            AggregationRule::CountByTarget => {
                let mut target_counts = HashMap::new();
                for log in logs {
                    *target_counts.entry(log.target.clone()).or_insert(0) += 1;
                }
                data.insert("target_counts".to_string(), serde_json::to_value(target_counts).unwrap());
            }
            AggregationRule::CountByTimeWindow { window_minutes } => {
                let mut time_counts = HashMap::new();
                for log in logs {
                    let time_key = log.timestamp.format("%Y-%m-%d %H:%M").to_string();
                    *time_counts.entry(time_key).or_insert(0) += 1;
                }
                data.insert("time_counts".to_string(), serde_json::to_value(time_counts).unwrap());
            }
            AggregationRule::CountByErrorType => {
                let mut error_counts = HashMap::new();
                for log in logs {
                    if let Some(error_details) = &log.error_details {
                        *error_counts.entry(error_details.error_type.clone()).or_insert(0) += 1;
                    }
                }
                data.insert("error_counts".to_string(), serde_json::to_value(error_counts).unwrap());
            }
            AggregationRule::PerformanceMetrics => {
                let mut performance_data = HashMap::new();
                let mut total_duration = 0.0;
                let mut count = 0;
                let mut max_duration: f64 = 0.0;
                let mut min_duration = f64::MAX;

                for log in logs {
                    if let Some(metrics) = &log.performance_metrics {
                        total_duration += metrics.duration_ms;
                        count += 1;
                        max_duration = max_duration.max(metrics.duration_ms);
                        min_duration = min_duration.min(metrics.duration_ms);
                    }
                }

                if count > 0 {
                    performance_data.insert("avg_duration_ms".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(total_duration / count as f64).unwrap()));
                    performance_data.insert("max_duration_ms".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(max_duration).unwrap()));
                    performance_data.insert("min_duration_ms".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(min_duration).unwrap()));
                    performance_data.insert("total_operations".to_string(), serde_json::Value::Number(serde_json::Number::from(count)));
                }

                data.insert("performance_metrics".to_string(), serde_json::to_value(performance_data).unwrap());
            }
            AggregationRule::Custom { name, expression: _ } => {
                // TODO: Implement custom expression evaluation
                data.insert("custom_rule".to_string(), serde_json::Value::String(name.clone()));
            }
        }

        let duration = start_time.elapsed();
        let data_size = serde_json::to_string(&data).unwrap_or_default().len() as u64;

        AggregatedLogData {
            timestamp: Utc::now(),
            aggregation_type: aggregation_type.to_string(),
            data,
            metadata: AggregationMetadata {
                source_logs_count: logs.len() as u64,
                aggregation_duration_ms: duration.as_millis() as f64,
                aggregation_rule: format!("{:?}", rule),
                data_size_bytes: data_size,
                compression_ratio: None, // TODO: Implement compression
            },
        }
    }

    /// Get aggregated data
    pub async fn get_aggregated_data(&self) -> Vec<AggregatedLogData> {
        let aggregated_data = self.aggregated_data.read().await;
        aggregated_data.clone()
    }

    /// Get aggregated data by type
    pub async fn get_aggregated_data_by_type(&self, aggregation_type: &str) -> Vec<AggregatedLogData> {
        let aggregated_data = self.aggregated_data.read().await;
        aggregated_data.iter()
            .filter(|data| data.aggregation_type == aggregation_type)
            .cloned()
            .collect()
    }

    /// Get aggregated data by time range
    pub async fn get_aggregated_data_by_time_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<AggregatedLogData> {
        let aggregated_data = self.aggregated_data.read().await;
        aggregated_data.iter()
            .filter(|data| data.timestamp >= start && data.timestamp <= end)
            .cloned()
            .collect()
    }

    /// Export aggregated data to JSON
    pub async fn export_aggregated_data_json(&self) -> String {
        let aggregated_data = self.aggregated_data.read().await;
        serde_json::to_string_pretty(&*aggregated_data).unwrap_or_default()
    }

    /// Export aggregated data to Prometheus format
    pub async fn export_aggregated_data_prometheus(&self) -> String {
        let aggregated_data = self.aggregated_data.read().await;
        let mut prometheus_output = String::new();

        for data in aggregated_data.iter() {
            for (key, value) in &data.data {
                if let Some(num_value) = value.as_f64() {
                    prometheus_output.push_str(&format!(
                        "ippan_log_aggregation_{}{{aggregation_type=\"{}\",rule=\"{}\"}} {}\n",
                        key.replace('-', "_"),
                        data.aggregation_type,
                        data.metadata.aggregation_rule,
                        num_value
                    ));
                }
            }
        }

        prometheus_output
    }

    /// Get aggregator statistics
    pub async fn get_statistics(&self) -> AggregatorStatistics {
        let aggregated_data = self.aggregated_data.read().await;
        let uptime = self.start_time.elapsed().as_secs();
        
        let mut total_data_processed = 0u64;
        let mut total_aggregation_time = 0.0;
        
        for data in aggregated_data.iter() {
            total_data_processed += data.metadata.data_size_bytes;
            total_aggregation_time += data.metadata.aggregation_duration_ms;
        }

        let average_aggregation_time = if aggregated_data.is_empty() {
            0.0
        } else {
            total_aggregation_time / aggregated_data.len() as f64
        };

        AggregatorStatistics {
            total_aggregated_logs: aggregated_data.len() as u64,
            total_aggregation_operations: aggregated_data.len() as u64,
            average_aggregation_time_ms: average_aggregation_time,
            total_data_processed_bytes: total_data_processed,
            compression_savings_bytes: 0, // TODO: Implement compression tracking
            uptime_seconds: uptime,
            active_aggregation_rules: self.config.aggregation_rules.len() as u32,
            output_formats_used: self.config.output_formats.len() as u32,
        }
    }

    /// Clear old aggregated data
    pub async fn clear_old_data(&self, max_age: Duration) {
        let mut aggregated_data = self.aggregated_data.write().await;
        let cutoff_time = Utc::now() - chrono::Duration::from_std(max_age).unwrap();
        aggregated_data.retain(|data| data.timestamp > cutoff_time);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_log_aggregator_creation() {
        let structured_logger = Arc::new(StructuredLogger::new());
        let aggregator = LogAggregator::new(structured_logger);
        let stats = aggregator.get_statistics().await;
        assert_eq!(stats.total_aggregated_logs, 0);
    }

    #[tokio::test]
    async fn test_aggregation_rule_count_by_level() {
        let structured_logger = Arc::new(StructuredLogger::new());
        let aggregator = LogAggregator::new(structured_logger);
        
        // Create test logs
        let test_logs = vec![
            LogEntry {
                id: "1".to_string(),
                timestamp: Utc::now(),
                level: LogLevel::Info,
                target: "test".to_string(),
                message: "Test message 1".to_string(),
                fields: HashMap::new(),
                error_details: None,
                performance_metrics: None,
                correlation_id: None,
                session_id: None,
                user_id: None,
                request_id: None,
            },
            LogEntry {
                id: "2".to_string(),
                timestamp: Utc::now(),
                level: LogLevel::Error,
                target: "test".to_string(),
                message: "Test error".to_string(),
                fields: HashMap::new(),
                error_details: None,
                performance_metrics: None,
                correlation_id: None,
                session_id: None,
                user_id: None,
                request_id: None,
            },
        ];

        let aggregated_data = LogAggregator::apply_aggregation_rule(
            &AggregationRule::CountByLevel,
            &test_logs,
            "test",
        ).await;

        assert_eq!(aggregated_data.aggregation_type, "test");
        assert!(aggregated_data.data.contains_key("level_counts"));
    }

    #[tokio::test]
    async fn test_export_prometheus_format() {
        let structured_logger = Arc::new(StructuredLogger::new());
        let aggregator = LogAggregator::new(structured_logger);
        
        let prometheus_output = aggregator.export_aggregated_data_prometheus().await;
        assert!(prometheus_output.is_empty()); // Should be empty initially
    }
}
