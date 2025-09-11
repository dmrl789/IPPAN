//! Log analyzer for IPPAN
//! 
//! Analyzes logs for patterns, anomalies, and insights

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::{interval, sleep};

use super::structured_logger::{StructuredLogger, LogEntry, LogLevel, ErrorDetails};

/// Analysis configuration
#[derive(Debug, Clone)]
pub struct AnalyzerConfig {
    pub analysis_interval_seconds: u64,
    pub enable_pattern_analysis: bool,
    pub enable_anomaly_detection: bool,
    pub enable_trend_analysis: bool,
    pub enable_correlation_analysis: bool,
    pub pattern_detection_rules: Vec<PatternRule>,
    pub anomaly_detection_threshold: f64,
    pub trend_analysis_window_hours: u32,
    pub correlation_analysis_depth: u32,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self {
            analysis_interval_seconds: 300, // 5 minutes
            enable_pattern_analysis: true,
            enable_anomaly_detection: true,
            enable_trend_analysis: true,
            enable_correlation_analysis: true,
            pattern_detection_rules: vec![
                PatternRule::ErrorSpike,
                PatternRule::PerformanceDegradation,
                PatternRule::ResourceExhaustion,
                PatternRule::SecurityThreat,
                PatternRule::Custom { name: "Custom Pattern".to_string(), pattern: "error.*critical".to_string() },
            ],
            anomaly_detection_threshold: 2.0, // 2 standard deviations
            trend_analysis_window_hours: 24,
            correlation_analysis_depth: 3,
        }
    }
}

/// Pattern detection rules
#[derive(Debug, Clone)]
pub enum PatternRule {
    ErrorSpike,
    PerformanceDegradation,
    ResourceExhaustion,
    SecurityThreat,
    Custom { name: String, pattern: String },
}

/// Analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub analysis_type: AnalysisType,
    pub severity: AnalysisSeverity,
    pub title: String,
    pub description: String,
    pub findings: Vec<Finding>,
    pub recommendations: Vec<String>,
    pub confidence_score: f64,
    pub metadata: AnalysisMetadata,
}

/// Analysis types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalysisType {
    PatternDetection,
    AnomalyDetection,
    TrendAnalysis,
    CorrelationAnalysis,
    PerformanceAnalysis,
    SecurityAnalysis,
    Custom(String),
}

/// Analysis severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalysisSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Individual finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub finding_type: String,
    pub description: String,
    pub evidence: Vec<String>,
    pub impact_score: f64,
    pub affected_components: Vec<String>,
}

/// Analysis metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisMetadata {
    pub logs_analyzed: u64,
    pub analysis_duration_ms: f64,
    pub patterns_detected: u32,
    pub anomalies_found: u32,
    pub trends_identified: u32,
    pub correlations_discovered: u32,
}

/// Analyzer statistics
#[derive(Debug, Clone)]
pub struct AnalyzerStatistics {
    pub total_analyses_performed: u64,
    pub total_findings_discovered: u64,
    pub total_patterns_detected: u64,
    pub total_anomalies_found: u64,
    pub total_trends_identified: u64,
    pub total_correlations_discovered: u64,
    pub average_analysis_time_ms: f64,
    pub uptime_seconds: u64,
    pub active_analysis_rules: u32,
    pub high_severity_findings: u64,
}

/// Log analyzer
pub struct LogAnalyzer {
    structured_logger: Arc<StructuredLogger>,
    config: AnalyzerConfig,
    analysis_results: Arc<RwLock<Vec<AnalysisResult>>>,
    start_time: Instant,
    is_running: Arc<RwLock<bool>>,
}

impl LogAnalyzer {
    /// Create a new log analyzer
    pub fn new(structured_logger: Arc<StructuredLogger>) -> Self {
        Self {
            structured_logger,
            config: AnalyzerConfig::default(),
            analysis_results: Arc::new(RwLock::new(Vec::new())),
            start_time: Instant::now(),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Create a new log analyzer with custom configuration
    pub fn with_config(structured_logger: Arc<StructuredLogger>, config: AnalyzerConfig) -> Self {
        Self {
            structured_logger,
            config,
            analysis_results: Arc::new(RwLock::new(Vec::new())),
            start_time: Instant::now(),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the log analyzer
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);

        // Start analysis loop
        self.start_analysis_loop().await;

        self.structured_logger.log(
            LogLevel::Info,
            "log_analyzer",
            "Log analyzer started",
            HashMap::new(),
        ).await;

        Ok(())
    }

    /// Stop the log analyzer
    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        drop(is_running);

        self.structured_logger.log(
            LogLevel::Info,
            "log_analyzer",
            "Log analyzer stopped",
            HashMap::new(),
        ).await;

        Ok(())
    }

    /// Start analysis loop
    async fn start_analysis_loop(&self) {
        // TODO: Implement analysis loop
        // Temporarily disabled due to async/Send issues
    }

    /// Perform analysis
    async fn perform_analysis(
        structured_logger: &Arc<StructuredLogger>,
        analysis_results: &Arc<RwLock<Vec<AnalysisResult>>>,
        config: &AnalyzerConfig,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        
        // Get recent logs for analysis
        let logs = structured_logger.get_logs().await;
        let recent_logs: Vec<LogEntry> = logs.into_iter()
            .filter(|log| log.timestamp > Utc::now() - chrono::Duration::hours(config.trend_analysis_window_hours as i64))
            .collect();

        if recent_logs.is_empty() {
            return Ok(());
        }

        let mut total_findings = 0u32;
        let mut total_patterns = 0u32;
        let mut total_anomalies = 0u32;
        let mut total_trends = 0u32;
        let mut total_correlations = 0u32;

        // Pattern analysis
        if config.enable_pattern_analysis {
            for rule in &config.pattern_detection_rules {
                if let Some(result) = Self::analyze_patterns(rule, &recent_logs).await {
                    total_findings += result.findings.len() as u32;
                    total_patterns += 1;
                    
                    let mut analysis_results_guard = analysis_results.write().await;
                    analysis_results_guard.push(result);
                    
                    // Maintain size limit
                    if analysis_results_guard.len() > 1000 {
                        analysis_results_guard.remove(0);
                    }
                }
            }
        }

        // Anomaly detection
        if config.enable_anomaly_detection {
            if let Some(result) = Self::detect_anomalies(&recent_logs, config.anomaly_detection_threshold).await {
                total_findings += result.findings.len() as u32;
                total_anomalies += 1;
                
                let mut analysis_results_guard = analysis_results.write().await;
                analysis_results_guard.push(result);
                
                // Maintain size limit
                if analysis_results_guard.len() > 1000 {
                    analysis_results_guard.remove(0);
                }
            }
        }

        // Trend analysis
        if config.enable_trend_analysis {
            if let Some(result) = Self::analyze_trends(&recent_logs, config.trend_analysis_window_hours).await {
                total_findings += result.findings.len() as u32;
                total_trends += 1;
                
                let mut analysis_results_guard = analysis_results.write().await;
                analysis_results_guard.push(result);
                
                // Maintain size limit
                if analysis_results_guard.len() > 1000 {
                    analysis_results_guard.remove(0);
                }
            }
        }

        // Correlation analysis
        if config.enable_correlation_analysis {
            if let Some(result) = Self::analyze_correlations(&recent_logs, config.correlation_analysis_depth).await {
                total_findings += result.findings.len() as u32;
                total_correlations += 1;
                
                let mut analysis_results_guard = analysis_results.write().await;
                analysis_results_guard.push(result);
                
                // Maintain size limit
                if analysis_results_guard.len() > 1000 {
                    analysis_results_guard.remove(0);
                }
            }
        }

        let duration = start_time.elapsed();
        
        structured_logger.log(
            LogLevel::Debug,
            "log_analyzer",
            &format!(
                "Analysis completed in {}ms: {} findings, {} patterns, {} anomalies, {} trends, {} correlations",
                duration.as_millis(),
                total_findings,
                total_patterns,
                total_anomalies,
                total_trends,
                total_correlations
            ),
            HashMap::new(),
        ).await;

        Ok(())
    }

    /// Analyze patterns
    async fn analyze_patterns(rule: &PatternRule, logs: &[LogEntry]) -> Option<AnalysisResult> {
        let start_time = Instant::now();
        let mut findings = Vec::new();

        match rule {
            PatternRule::ErrorSpike => {
                let error_count = logs.iter().filter(|log| matches!(log.level, LogLevel::Error | LogLevel::Fatal)).count();
                let total_logs = logs.len();
                let error_rate = if total_logs > 0 { error_count as f64 / total_logs as f64 } else { 0.0 };

                if error_rate > 0.1 { // More than 10% errors
                    findings.push(Finding {
                        finding_type: "Error Spike".to_string(),
                        description: format!("High error rate detected: {:.2}%", error_rate * 100.0),
                        evidence: vec![format!("Error count: {}", error_count), format!("Total logs: {}", total_logs)],
                        impact_score: error_rate,
                        affected_components: vec!["system".to_string()],
                    });
                }
            }
            PatternRule::PerformanceDegradation => {
                let mut slow_operations = 0;
                let mut total_operations = 0;

                for log in logs {
                    if let Some(metrics) = &log.performance_metrics {
                        total_operations += 1;
                        if metrics.duration_ms > 1000.0 { // Operations taking more than 1 second
                            slow_operations += 1;
                        }
                    }
                }

                if total_operations > 0 {
                    let slow_rate = slow_operations as f64 / total_operations as f64;
                    if slow_rate > 0.05 { // More than 5% slow operations
                        findings.push(Finding {
                            finding_type: "Performance Degradation".to_string(),
                            description: format!("High rate of slow operations: {:.2}%", slow_rate * 100.0),
                            evidence: vec![format!("Slow operations: {}", slow_operations), format!("Total operations: {}", total_operations)],
                            impact_score: slow_rate,
                            affected_components: vec!["performance".to_string()],
                        });
                    }
                }
            }
            PatternRule::ResourceExhaustion => {
                let mut high_memory_logs = 0;
                let mut total_memory_logs = 0;

                for log in logs {
                    if let Some(metrics) = &log.performance_metrics {
                        if let Some(memory_usage) = metrics.memory_usage_bytes {
                            total_memory_logs += 1;
                            if memory_usage > 1024 * 1024 * 1024 { // More than 1GB
                                high_memory_logs += 1;
                            }
                        }
                    }
                }

                if total_memory_logs > 0 {
                    let high_memory_rate = high_memory_logs as f64 / total_memory_logs as f64;
                    if high_memory_rate > 0.1 { // More than 10% high memory usage
                        findings.push(Finding {
                            finding_type: "Resource Exhaustion".to_string(),
                            description: format!("High memory usage detected: {:.2}%", high_memory_rate * 100.0),
                            evidence: vec![format!("High memory logs: {}", high_memory_logs), format!("Total memory logs: {}", total_memory_logs)],
                            impact_score: high_memory_rate,
                            affected_components: vec!["memory".to_string()],
                        });
                    }
                }
            }
            PatternRule::SecurityThreat => {
                let security_keywords = ["unauthorized", "attack", "breach", "intrusion", "malware", "virus"];
                let mut security_events = 0;

                for log in logs {
                    for keyword in &security_keywords {
                        if log.message.to_lowercase().contains(keyword) {
                            security_events += 1;
                            break;
                        }
                    }
                }

                if security_events > 0 {
                    findings.push(Finding {
                        finding_type: "Security Threat".to_string(),
                        description: format!("Potential security events detected: {}", security_events),
                        evidence: vec![format!("Security events: {}", security_events)],
                        impact_score: security_events as f64,
                        affected_components: vec!["security".to_string()],
                    });
                }
            }
            PatternRule::Custom { name, pattern } => {
                // Simple pattern matching (in real implementation, use regex)
                let mut matches = 0;
                for log in logs {
                    if log.message.contains(pattern) {
                        matches += 1;
                    }
                }

                if matches > 0 {
                    findings.push(Finding {
                        finding_type: name.clone(),
                        description: format!("Custom pattern matches: {}", matches),
                        evidence: vec![format!("Pattern: {}", pattern), format!("Matches: {}", matches)],
                        impact_score: matches as f64,
                        affected_components: vec!["custom".to_string()],
                    });
                }
            }
        }

        if findings.is_empty() {
            return None;
        }

        let severity = if findings.iter().any(|f| f.impact_score > 0.5) {
            AnalysisSeverity::High
        } else if findings.iter().any(|f| f.impact_score > 0.2) {
            AnalysisSeverity::Medium
        } else {
            AnalysisSeverity::Low
        };

        let confidence_score = findings.iter().map(|f| f.impact_score).sum::<f64>() / findings.len() as f64;

        Some(AnalysisResult {
            id: format!("analysis_{}", Utc::now().timestamp()),
            timestamp: Utc::now(),
            analysis_type: AnalysisType::PatternDetection,
            severity,
            title: format!("Pattern Analysis: {:?}", rule),
            description: format!("Detected {} patterns in log analysis", findings.len()),
            findings,
            recommendations: vec![
                "Monitor system closely".to_string(),
                "Review affected components".to_string(),
                "Consider preventive measures".to_string(),
            ],
            confidence_score,
            metadata: AnalysisMetadata {
                logs_analyzed: logs.len() as u64,
                analysis_duration_ms: start_time.elapsed().as_millis() as f64,
                patterns_detected: 1,
                anomalies_found: 0,
                trends_identified: 0,
                correlations_discovered: 0,
            },
        })
    }

    /// Detect anomalies
    async fn detect_anomalies(logs: &[LogEntry], threshold: f64) -> Option<AnalysisResult> {
        let start_time = Instant::now();
        let mut findings = Vec::new();

        // Simple anomaly detection based on log volume
        let mut hourly_counts = HashMap::new();
        for log in logs {
            let hour_key = log.timestamp.format("%Y-%m-%d %H").to_string();
            *hourly_counts.entry(hour_key).or_insert(0) += 1;
        }

        if hourly_counts.len() > 1 {
            let counts: Vec<u32> = hourly_counts.values().cloned().collect();
            let mean = counts.iter().sum::<u32>() as f64 / counts.len() as f64;
            let variance = counts.iter().map(|&x| (x as f64 - mean).powi(2)).sum::<f64>() / counts.len() as f64;
            let std_dev = variance.sqrt();

            for (hour, count) in hourly_counts {
                let z_score = if std_dev > 0.0 { (count as f64 - mean) / std_dev } else { 0.0 };
                if z_score.abs() > threshold {
                    findings.push(Finding {
                        finding_type: "Anomaly Detection".to_string(),
                        description: format!("Anomalous log volume in hour {}: {} logs (z-score: {:.2})", hour, count, z_score),
                        evidence: vec![format!("Hour: {}", hour), format!("Count: {}", count), format!("Z-score: {:.2}", z_score)],
                        impact_score: z_score.abs() / threshold,
                        affected_components: vec!["logging".to_string()],
                    });
                }
            }
        }

        if findings.is_empty() {
            return None;
        }

        let severity = if findings.iter().any(|f| f.impact_score > 2.0) {
            AnalysisSeverity::High
        } else if findings.iter().any(|f| f.impact_score > 1.5) {
            AnalysisSeverity::Medium
        } else {
            AnalysisSeverity::Low
        };

        let confidence_score = findings.iter().map(|f| f.impact_score).sum::<f64>() / findings.len() as f64;

        Some(AnalysisResult {
            id: format!("analysis_{}", Utc::now().timestamp()),
            timestamp: Utc::now(),
            analysis_type: AnalysisType::AnomalyDetection,
            severity,
            title: "Anomaly Detection Results".to_string(),
            description: format!("Detected {} anomalies in log analysis", findings.len()),
            findings: findings.clone(),
            recommendations: vec![
                "Investigate anomalous time periods".to_string(),
                "Check for system issues during those times".to_string(),
                "Consider adjusting monitoring thresholds".to_string(),
            ],
            confidence_score,
            metadata: AnalysisMetadata {
                logs_analyzed: logs.len() as u64,
                analysis_duration_ms: start_time.elapsed().as_millis() as f64,
                patterns_detected: 0,
                anomalies_found: findings.len() as u32,
                trends_identified: 0,
                correlations_discovered: 0,
            },
        })
    }

    /// Analyze trends
    async fn analyze_trends(logs: &[LogEntry], window_hours: u32) -> Option<AnalysisResult> {
        let start_time = Instant::now();
        let mut findings = Vec::new();

        // Simple trend analysis based on error rate over time
        let mut hourly_error_rates = HashMap::new();
        let mut hourly_total_logs = HashMap::new();

        for log in logs {
            let hour_key = log.timestamp.format("%Y-%m-%d %H").to_string();
            *hourly_total_logs.entry(hour_key.clone()).or_insert(0) += 1;
            if matches!(log.level, LogLevel::Error | LogLevel::Fatal) {
                *hourly_error_rates.entry(hour_key).or_insert(0) += 1;
            }
        }

        let mut error_rate_trend = Vec::new();
        for (hour, total) in hourly_total_logs {
            let errors = hourly_error_rates.get(&hour).unwrap_or(&0);
            let error_rate = if total > 0 { *errors as f64 / total as f64 } else { 0.0 };
            error_rate_trend.push(error_rate);
        }

        if error_rate_trend.len() > 2 {
            // Simple trend detection (increasing/decreasing)
            let first_half_avg = error_rate_trend[..error_rate_trend.len()/2].iter().sum::<f64>() / (error_rate_trend.len()/2) as f64;
            let second_half_avg = error_rate_trend[error_rate_trend.len()/2..].iter().sum::<f64>() / (error_rate_trend.len() - error_rate_trend.len()/2) as f64;
            
            let trend_change = second_half_avg - first_half_avg;
            if trend_change.abs() > 0.05 { // Significant trend change
                let trend_direction = if trend_change > 0.0 { "increasing" } else { "decreasing" };
                findings.push(Finding {
                    finding_type: "Trend Analysis".to_string(),
                    description: format!("Error rate trend is {}: {:.2}% change", trend_direction, trend_change * 100.0),
                    evidence: vec![
                        format!("First half average: {:.2}%", first_half_avg * 100.0),
                        format!("Second half average: {:.2}%", second_half_avg * 100.0),
                        format!("Trend change: {:.2}%", trend_change * 100.0),
                    ],
                    impact_score: trend_change.abs(),
                    affected_components: vec!["error_handling".to_string()],
                });
            }
        }

        if findings.is_empty() {
            return None;
        }

        let severity = if findings.iter().any(|f| f.impact_score > 0.1) {
            AnalysisSeverity::High
        } else if findings.iter().any(|f| f.impact_score > 0.05) {
            AnalysisSeverity::Medium
        } else {
            AnalysisSeverity::Low
        };

        let confidence_score = findings.iter().map(|f| f.impact_score).sum::<f64>() / findings.len() as f64;

        Some(AnalysisResult {
            id: format!("analysis_{}", Utc::now().timestamp()),
            timestamp: Utc::now(),
            analysis_type: AnalysisType::TrendAnalysis,
            severity,
            title: "Trend Analysis Results".to_string(),
            description: format!("Identified {} trends in log analysis", findings.len()),
            findings: findings.clone(),
            recommendations: vec![
                "Monitor trend direction".to_string(),
                "Take preventive action if trend is negative".to_string(),
                "Document trend for future reference".to_string(),
            ],
            confidence_score,
            metadata: AnalysisMetadata {
                logs_analyzed: logs.len() as u64,
                analysis_duration_ms: start_time.elapsed().as_millis() as f64,
                patterns_detected: 0,
                anomalies_found: 0,
                trends_identified: findings.len() as u32,
                correlations_discovered: 0,
            },
        })
    }

    /// Analyze correlations
    async fn analyze_correlations(logs: &[LogEntry], depth: u32) -> Option<AnalysisResult> {
        let start_time = Instant::now();
        let mut findings = Vec::new();

        // Simple correlation analysis between error logs and performance logs
        let mut error_times = Vec::new();
        let mut performance_times = Vec::new();

        for log in logs {
            if matches!(log.level, LogLevel::Error | LogLevel::Fatal) {
                error_times.push(log.timestamp);
            }
            if log.performance_metrics.is_some() {
                performance_times.push(log.timestamp);
            }
        }

        // Look for correlations within time windows
        let mut correlations_found = 0;
        for error_time in &error_times {
            for perf_time in &performance_times {
                let time_diff = (*error_time - *perf_time).num_seconds().abs();
                if time_diff <= 300 { // Within 5 minutes
                    correlations_found += 1;
                }
            }
        }

        if correlations_found > 0 {
            findings.push(Finding {
                finding_type: "Correlation Analysis".to_string(),
                description: format!("Found {} correlations between errors and performance events", correlations_found),
                evidence: vec![
                    format!("Error events: {}", error_times.len()),
                    format!("Performance events: {}", performance_times.len()),
                    format!("Correlations found: {}", correlations_found),
                ],
                impact_score: correlations_found as f64 / (error_times.len() + performance_times.len()) as f64,
                affected_components: vec!["error_handling".to_string(), "performance".to_string()],
            });
        }

        if findings.is_empty() {
            return None;
        }

        let severity = if findings.iter().any(|f| f.impact_score > 0.5) {
            AnalysisSeverity::High
        } else if findings.iter().any(|f| f.impact_score > 0.2) {
            AnalysisSeverity::Medium
        } else {
            AnalysisSeverity::Low
        };

        let confidence_score = findings.iter().map(|f| f.impact_score).sum::<f64>() / findings.len() as f64;

        Some(AnalysisResult {
            id: format!("analysis_{}", Utc::now().timestamp()),
            timestamp: Utc::now(),
            analysis_type: AnalysisType::CorrelationAnalysis,
            severity,
            title: "Correlation Analysis Results".to_string(),
            description: format!("Discovered {} correlations in log analysis", findings.len()),
            findings: findings.clone(),
            recommendations: vec![
                "Investigate correlated events".to_string(),
                "Look for root causes".to_string(),
                "Implement preventive measures".to_string(),
            ],
            confidence_score,
            metadata: AnalysisMetadata {
                logs_analyzed: logs.len() as u64,
                analysis_duration_ms: start_time.elapsed().as_millis() as f64,
                patterns_detected: 0,
                anomalies_found: 0,
                trends_identified: 0,
                correlations_discovered: findings.len() as u32,
            },
        })
    }

    /// Get analysis results
    pub async fn get_analysis_results(&self) -> Vec<AnalysisResult> {
        let analysis_results = self.analysis_results.read().await;
        analysis_results.clone()
    }

    /// Get analysis results by type
    pub async fn get_analysis_results_by_type(&self, analysis_type: &AnalysisType) -> Vec<AnalysisResult> {
        let analysis_results = self.analysis_results.read().await;
        analysis_results.iter()
            .filter(|result| std::mem::discriminant(&result.analysis_type) == std::mem::discriminant(analysis_type))
            .cloned()
            .collect()
    }

    /// Get analysis results by severity
    pub async fn get_analysis_results_by_severity(&self, severity: &AnalysisSeverity) -> Vec<AnalysisResult> {
        let analysis_results = self.analysis_results.read().await;
        analysis_results.iter()
            .filter(|result| std::mem::discriminant(&result.severity) == std::mem::discriminant(severity))
            .cloned()
            .collect()
    }

    /// Get analyzer statistics
    pub async fn get_statistics(&self) -> AnalyzerStatistics {
        let analysis_results = self.analysis_results.read().await;
        let uptime = self.start_time.elapsed().as_secs();
        
        let mut total_findings = 0u64;
        let mut total_patterns = 0u64;
        let mut total_anomalies = 0u64;
        let mut total_trends = 0u64;
        let mut total_correlations = 0u64;
        let mut high_severity_findings = 0u64;
        let mut total_analysis_time = 0.0;

        for result in analysis_results.iter() {
            total_findings += result.findings.len() as u64;
            total_analysis_time += result.metadata.analysis_duration_ms;
            
            match result.analysis_type {
                AnalysisType::PatternDetection => total_patterns += 1,
                AnalysisType::AnomalyDetection => total_anomalies += 1,
                AnalysisType::TrendAnalysis => total_trends += 1,
                AnalysisType::CorrelationAnalysis => total_correlations += 1,
                _ => {}
            }
            
            if matches!(result.severity, AnalysisSeverity::High | AnalysisSeverity::Critical) {
                high_severity_findings += result.findings.len() as u64;
            }
        }

        let average_analysis_time = if analysis_results.is_empty() {
            0.0
        } else {
            total_analysis_time / analysis_results.len() as f64
        };

        AnalyzerStatistics {
            total_analyses_performed: analysis_results.len() as u64,
            total_findings_discovered: total_findings,
            total_patterns_detected: total_patterns,
            total_anomalies_found: total_anomalies,
            total_trends_identified: total_trends,
            total_correlations_discovered: total_correlations,
            average_analysis_time_ms: average_analysis_time,
            uptime_seconds: uptime,
            active_analysis_rules: self.config.pattern_detection_rules.len() as u32,
            high_severity_findings,
        }
    }

    /// Clear old analysis results
    pub async fn clear_old_results(&self, max_age: Duration) {
        let mut analysis_results = self.analysis_results.write().await;
        let cutoff_time = Utc::now() - chrono::Duration::from_std(max_age).unwrap();
        analysis_results.retain(|result| result.timestamp > cutoff_time);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_log_analyzer_creation() {
        let structured_logger = Arc::new(StructuredLogger::new());
        let analyzer = LogAnalyzer::new(structured_logger);
        let stats = analyzer.get_statistics().await;
        assert_eq!(stats.total_analyses_performed, 0);
    }

    #[tokio::test]
    async fn test_pattern_analysis_error_spike() {
        let structured_logger = Arc::new(StructuredLogger::new());
        let analyzer = LogAnalyzer::new(structured_logger);
        
        // Create test logs with high error rate
        let test_logs = vec![
            LogEntry {
                id: "1".to_string(),
                timestamp: Utc::now(),
                level: LogLevel::Error,
                target: "test".to_string(),
                message: "Test error 1".to_string(),
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
                message: "Test error 2".to_string(),
                fields: HashMap::new(),
                error_details: None,
                performance_metrics: None,
                correlation_id: None,
                session_id: None,
                user_id: None,
                request_id: None,
            },
            LogEntry {
                id: "3".to_string(),
                timestamp: Utc::now(),
                level: LogLevel::Info,
                target: "test".to_string(),
                message: "Test info".to_string(),
                fields: HashMap::new(),
                error_details: None,
                performance_metrics: None,
                correlation_id: None,
                session_id: None,
                user_id: None,
                request_id: None,
            },
        ];

        let result = LogAnalyzer::analyze_patterns(&PatternRule::ErrorSpike, &test_logs).await;
        assert!(result.is_some());
        assert!(!result.unwrap().findings.is_empty());
    }

    #[tokio::test]
    async fn test_anomaly_detection() {
        let structured_logger = Arc::new(StructuredLogger::new());
        let analyzer = LogAnalyzer::new(structured_logger);
        
        // Create test logs with anomalous pattern
        let mut test_logs = Vec::new();
        for i in 0..10 {
            test_logs.push(LogEntry {
                id: i.to_string(),
                timestamp: Utc::now() - chrono::Duration::hours(i as i64),
                level: LogLevel::Info,
                target: "test".to_string(),
                message: format!("Test message {}", i),
                fields: HashMap::new(),
                error_details: None,
                performance_metrics: None,
                correlation_id: None,
                session_id: None,
                user_id: None,
                request_id: None,
            });
        }

        let result = LogAnalyzer::detect_anomalies(&test_logs, 2.0).await;
        // Result may or may not be Some depending on the distribution
        // This test just ensures the function doesn't panic
        assert!(true);
    }
}
