//! AI-powered monitoring and alerting module

use crate::types::{
    MonitoringAlert, AlertStatus, SeverityLevel, MonitoringConfig
};
use crate::errors::AIServiceError;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

/// Monitoring service
pub struct MonitoringService {
    config: MonitoringConfig,
    alerts: Vec<MonitoringAlert>,
    metrics_history: HashMap<String, Vec<f64>>,
    anomaly_detector: AnomalyDetector,
}

/// Anomaly detector for monitoring
struct AnomalyDetector {
    window_size: usize,
    threshold_multiplier: f64,
}

impl AnomalyDetector {
    fn new() -> Self {
        Self {
            window_size: 20,
            threshold_multiplier: 2.0,
        }
    }

    /// Detect anomalies using statistical methods
    fn detect_anomalies(&self, metric_name: &str, values: &[f64]) -> Vec<Anomaly> {
        if values.len() < self.window_size {
            return Vec::new();
        }

        let recent_values = &values[values.len().saturating_sub(self.window_size)..];
        let mean = recent_values.iter().sum::<f64>() / recent_values.len() as f64;
        let variance = recent_values.iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>() / recent_values.len() as f64;
        let std_dev = variance.sqrt();

        let threshold = self.threshold_multiplier * std_dev;
        let mut anomalies = Vec::new();

        for (i, &value) in recent_values.iter().enumerate() {
            if (value - mean).abs() > threshold {
                anomalies.push(Anomaly {
                    metric: metric_name.to_string(),
                    value,
                    expected_range: (mean - threshold, mean + threshold),
                    severity: self.calculate_anomaly_severity(value, mean, std_dev),
                    timestamp: Utc::now(),
                });
            }
        }

        anomalies
    }

    fn calculate_anomaly_severity(&self, value: f64, mean: f64, std_dev: f64) -> SeverityLevel {
        let deviation = (value - mean).abs() / std_dev;
        
        if deviation > 4.0 {
            SeverityLevel::Critical
        } else if deviation > 3.0 {
            SeverityLevel::High
        } else if deviation > 2.0 {
            SeverityLevel::Medium
        } else {
            SeverityLevel::Low
        }
    }
}

/// Anomaly detection result
#[derive(Debug, Clone)]
struct Anomaly {
    metric: String,
    value: f64,
    expected_range: (f64, f64),
    severity: SeverityLevel,
    timestamp: DateTime<Utc>,
}

impl MonitoringService {
    /// Create a new monitoring service
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            config,
            alerts: Vec::new(),
            metrics_history: HashMap::new(),
            anomaly_detector: AnomalyDetector::new(),
        }
    }

    /// Add a metric value
    pub fn add_metric(&mut self, metric_name: String, value: f64) {
        self.metrics_history
            .entry(metric_name.clone())
            .or_insert_with(Vec::new)
            .push(value);

        // Keep only recent history
        if let Some(history) = self.metrics_history.get_mut(&metric_name) {
            if history.len() > 1000 {
                history.drain(0..history.len() - 1000);
            }
        }
    }

    /// Check for alerts and anomalies
    pub async fn check_alerts(&mut self) -> Result<Vec<MonitoringAlert>, AIServiceError> {
        let mut new_alerts = Vec::new();

        if !self.config.enable_anomaly_detection {
            return Ok(new_alerts);
        }

        // Check each metric for anomalies
        for (metric_name, values) in &self.metrics_history.clone() {
            let anomalies = self.anomaly_detector.detect_anomalies(metric_name, values);
            
            for anomaly in anomalies {
                let alert = self.create_anomaly_alert(anomaly);
                new_alerts.push(alert);
            }
        }

        // Check threshold-based alerts
        for (metric_name, values) in &self.metrics_history {
            if let Some(threshold) = self.config.alert_thresholds.get(metric_name) {
                if let Some(&latest_value) = values.last() {
                    if latest_value > *threshold {
                        let alert = self.create_threshold_alert(metric_name, latest_value, *threshold);
                        new_alerts.push(alert);
                    }
                }
            }
        }

        // Store new alerts
        self.alerts.extend(new_alerts.clone());

        // Clean up old alerts
        self.cleanup_old_alerts();

        Ok(new_alerts)
    }

    /// Create an anomaly alert
    fn create_anomaly_alert(&self, anomaly: Anomaly) -> MonitoringAlert {
        let alert_id = Uuid::new_v4().to_string();
        let mut metrics = HashMap::new();
        metrics.insert(anomaly.metric.clone(), anomaly.value);

        MonitoringAlert {
            alert_id,
            alert_type: "anomaly".to_string(),
            severity: anomaly.severity.clone(),
            title: format!("Anomaly detected in {}", anomaly.metric),
            description: format!(
                "Value {} is outside expected range [{:.2}, {:.2}]",
                anomaly.value,
                anomaly.expected_range.0,
                anomaly.expected_range.1
            ),
            metrics,
            timestamp: anomaly.timestamp,
            status: AlertStatus::Active,
            actions_taken: Vec::new(),
        }
    }

    /// Create a threshold alert
    fn create_threshold_alert(&self, metric_name: &str, value: f64, threshold: f64) -> MonitoringAlert {
        let alert_id = Uuid::new_v4().to_string();
        let mut metrics = HashMap::new();
        metrics.insert(metric_name.to_string(), value);

        let severity = if value > threshold * 2.0 {
            SeverityLevel::Critical
        } else if value > threshold * 1.5 {
            SeverityLevel::High
        } else {
            SeverityLevel::Medium
        };

        MonitoringAlert {
            alert_id,
            alert_type: "threshold".to_string(),
            severity,
            title: format!("{} exceeded threshold", metric_name),
            description: format!(
                "Value {} exceeded threshold {} by {:.2}%",
                value,
                threshold,
                ((value - threshold) / threshold) * 100.0
            ),
            metrics,
            timestamp: Utc::now(),
            status: AlertStatus::Active,
            actions_taken: Vec::new(),
        }
    }

    /// Clean up old alerts
    fn cleanup_old_alerts(&mut self) {
        let cutoff_time = Utc::now() - chrono::Duration::days(7);
        self.alerts.retain(|alert| alert.timestamp > cutoff_time);
    }

    /// Get all alerts
    pub fn get_alerts(&self) -> &[MonitoringAlert] {
        &self.alerts
    }

    /// Get active alerts
    pub fn get_active_alerts(&self) -> Vec<&MonitoringAlert> {
        self.alerts
            .iter()
            .filter(|alert| alert.status == AlertStatus::Active)
            .collect()
    }

    /// Get alerts by severity
    pub fn get_alerts_by_severity(&self, severity: SeverityLevel) -> Vec<&MonitoringAlert> {
        self.alerts
            .iter()
            .filter(|alert| std::mem::discriminant(&alert.severity) == std::mem::discriminant(&severity))
            .collect()
    }

    /// Acknowledge an alert
    pub fn acknowledge_alert(&mut self, alert_id: &str) -> Result<(), AIServiceError> {
        if let Some(alert) = self.alerts.iter_mut().find(|a| a.alert_id == alert_id) {
            alert.status = AlertStatus::Acknowledged;
            alert.actions_taken.push("Acknowledged".to_string());
            Ok(())
        } else {
            Err(AIServiceError::ValidationError("Alert not found".to_string()))
        }
    }

    /// Resolve an alert
    pub fn resolve_alert(&mut self, alert_id: &str, resolution: String) -> Result<(), AIServiceError> {
        if let Some(alert) = self.alerts.iter_mut().find(|a| a.alert_id == alert_id) {
            alert.status = AlertStatus::Resolved;
            alert.actions_taken.push(format!("Resolved: {}", resolution));
            Ok(())
        } else {
            Err(AIServiceError::ValidationError("Alert not found".to_string()))
        }
    }

    /// Suppress an alert
    pub fn suppress_alert(&mut self, alert_id: &str, reason: String) -> Result<(), AIServiceError> {
        if let Some(alert) = self.alerts.iter_mut().find(|a| a.alert_id == alert_id) {
            alert.status = AlertStatus::Suppressed;
            alert.actions_taken.push(format!("Suppressed: {}", reason));
            Ok(())
        } else {
            Err(AIServiceError::ValidationError("Alert not found".to_string()))
        }
    }

    /// Get monitoring statistics
    pub fn get_statistics(&self) -> MonitoringStatistics {
        let total_alerts = self.alerts.len();
        let active_alerts = self.get_active_alerts().len();
        let critical_alerts = self.get_alerts_by_severity(SeverityLevel::Critical).len();
        let high_alerts = self.get_alerts_by_severity(SeverityLevel::High).len();
        let medium_alerts = self.get_alerts_by_severity(SeverityLevel::Medium).len();
        let low_alerts = self.get_alerts_by_severity(SeverityLevel::Low).len();

        let mut metrics_count = 0;
        let mut total_data_points = 0;
        for (_, values) in &self.metrics_history {
            metrics_count += 1;
            total_data_points += values.len();
        }

        MonitoringStatistics {
            total_alerts,
            active_alerts,
            critical_alerts,
            high_alerts,
            medium_alerts,
            low_alerts,
            metrics_count,
            total_data_points,
            last_updated: Utc::now(),
        }
    }
}

/// Monitoring statistics
#[derive(Debug, Clone)]
pub struct MonitoringStatistics {
    pub total_alerts: usize,
    pub active_alerts: usize,
    pub critical_alerts: usize,
    pub high_alerts: usize,
    pub medium_alerts: usize,
    pub low_alerts: usize,
    pub metrics_count: usize,
    pub total_data_points: usize,
    pub last_updated: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitoring_service_creation() {
        let config = MonitoringConfig {
            enable_anomaly_detection: true,
            alert_thresholds: HashMap::new(),
            monitoring_interval: 30,
            enable_auto_remediation: false,
        };

        let service = MonitoringService::new(config);
        assert_eq!(service.alerts.len(), 0);
        assert_eq!(service.metrics_history.len(), 0);
    }

    #[test]
    fn test_add_metric() {
        let mut service = MonitoringService::new(Default::default());
        service.add_metric("cpu_usage".to_string(), 75.0);
        
        assert_eq!(service.metrics_history.len(), 1);
        assert_eq!(service.metrics_history["cpu_usage"].len(), 1);
        assert_eq!(service.metrics_history["cpu_usage"][0], 75.0);
    }

    #[tokio::test]
    async fn test_anomaly_detection() {
        let mut service = MonitoringService::new(Default::default());
        
        // Add normal values
        for i in 0..20 {
            service.add_metric("test_metric".to_string(), 50.0 + (i as f64 * 0.1));
        }
        
        // Add an anomaly
        service.add_metric("test_metric".to_string(), 200.0);
        
        let alerts = service.check_alerts().await.unwrap();
        assert!(!alerts.is_empty());
        assert_eq!(alerts[0].alert_type, "anomaly");
    }

    #[test]
    fn test_alert_acknowledgment() {
        let mut service = MonitoringService::new(Default::default());
        
        // Create a test alert
        let alert = MonitoringAlert {
            alert_id: "test_alert".to_string(),
            alert_type: "test".to_string(),
            severity: SeverityLevel::Medium,
            title: "Test Alert".to_string(),
            description: "Test Description".to_string(),
            metrics: HashMap::new(),
            timestamp: Utc::now(),
            status: AlertStatus::Active,
            actions_taken: Vec::new(),
        };
        
        service.alerts.push(alert);
        
        // Acknowledge the alert
        let result = service.acknowledge_alert("test_alert");
        assert!(result.is_ok());
        assert_eq!(service.alerts[0].status, AlertStatus::Acknowledged);
    }
}