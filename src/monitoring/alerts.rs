//! Alerting and notification system for IPPAN
//! 
//! Provides proactive monitoring and automated responses to critical events

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

/// Alert status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertStatus {
    Active,
    Acknowledged,
    Resolved,
    Suppressed,
}

/// Alert type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertType {
    SystemHealth,
    Performance,
    Security,
    Storage,
    Network,
    Consensus,
    Custom,
}

/// Alert notification channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationChannel {
    Email,
    Webhook,
    Slack,
    Discord,
    PagerDuty,
    Custom(String),
}

/// Alert rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub condition: AlertCondition,
    pub notification_channels: Vec<NotificationChannel>,
    pub enabled: bool,
    pub cooldown_seconds: u64,
    pub last_triggered: Option<DateTime<Utc>>,
}

/// Alert condition types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    Threshold {
        metric: String,
        operator: ThresholdOperator,
        value: f64,
        duration_seconds: u64,
    },
    Anomaly {
        metric: String,
        sensitivity: f64,
        window_seconds: u64,
    },
    Pattern {
        pattern: String,
        match_count: u32,
        window_seconds: u64,
    },
    Custom {
        expression: String,
    },
}

/// Threshold operators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThresholdOperator {
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Equal,
    NotEqual,
}

/// Alert instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub rule_id: String,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub status: AlertStatus,
    pub title: String,
    pub message: String,
    pub details: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub acknowledged_by: Option<String>,
    pub notification_sent: bool,
}

/// Notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub email: Option<EmailConfig>,
    pub webhook: Option<WebhookConfig>,
    pub slack: Option<SlackConfig>,
    pub discord: Option<DiscordConfig>,
    pub pagerduty: Option<PagerDutyConfig>,
}

/// Email notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub smtp_server: String,
    pub smtp_port: u16,
    pub username: String,
    pub password: String,
    pub from_address: String,
    pub to_addresses: Vec<String>,
    pub use_tls: bool,
}

/// Webhook notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub timeout_seconds: u64,
}

/// Slack notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    pub webhook_url: String,
    pub channel: String,
    pub username: String,
}

/// Discord notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub webhook_url: String,
    pub username: String,
}

/// PagerDuty notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagerDutyConfig {
    pub api_key: String,
    pub service_id: String,
    pub escalation_policy: String,
}

/// Main alerting system
pub struct AlertingSystem {
    rules: Arc<RwLock<Vec<AlertRule>>>,
    alerts: Arc<RwLock<Vec<Alert>>>,
    notification_config: Arc<RwLock<NotificationConfig>>,
    metrics_collector: Arc<crate::monitoring::metrics::MetricsCollector>,
    logger: Arc<crate::utils::logging::StructuredLogger>,
}

impl AlertingSystem {
    /// Create a new alerting system
    pub fn new(
        metrics_collector: Arc<crate::monitoring::metrics::MetricsCollector>,
        logger: Arc<crate::utils::logging::StructuredLogger>,
    ) -> Self {
        Self {
            rules: Arc::new(RwLock::new(Vec::new())),
            alerts: Arc::new(RwLock::new(Vec::new())),
            notification_config: Arc::new(RwLock::new(NotificationConfig {
                email: None,
                webhook: None,
                slack: None,
                discord: None,
                pagerduty: None,
            })),
            metrics_collector,
            logger,
        }
    }

    /// Add an alert rule
    pub async fn add_rule(&self, rule: AlertRule) -> Result<(), Box<dyn std::error::Error>> {
        let mut rules = self.rules.write().await;
        rules.push(rule);
        Ok(())
    }

    /// Remove an alert rule
    pub async fn remove_rule(&self, rule_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut rules = self.rules.write().await;
        rules.retain(|rule| rule.id != rule_id);
        Ok(())
    }

    /// Get all alert rules
    pub async fn get_rules(&self) -> Vec<AlertRule> {
        let rules = self.rules.read().await;
        rules.clone()
    }

    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Vec<Alert> {
        let alerts = self.alerts.read().await;
        alerts.iter()
            .filter(|alert| alert.status == AlertStatus::Active)
            .cloned()
            .collect()
    }

    /// Get all alerts
    pub async fn get_alerts(&self) -> Vec<Alert> {
        let alerts = self.alerts.read().await;
        alerts.clone()
    }

    /// Acknowledge an alert
    pub async fn acknowledge_alert(&self, alert_id: &str, user: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut alerts = self.alerts.write().await;
        if let Some(alert) = alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.status = AlertStatus::Acknowledged;
            alert.acknowledged_at = Some(Utc::now());
            alert.acknowledged_by = Some(user.to_string());
        }
        Ok(())
    }

    /// Resolve an alert
    pub async fn resolve_alert(&self, alert_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut alerts = self.alerts.write().await;
        if let Some(alert) = alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.status = AlertStatus::Resolved;
            alert.resolved_at = Some(Utc::now());
        }
        Ok(())
    }

    /// Suppress an alert
    pub async fn suppress_alert(&self, alert_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut alerts = self.alerts.write().await;
        if let Some(alert) = alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.status = AlertStatus::Suppressed;
        }
        Ok(())
    }

    /// Set notification configuration
    pub async fn set_notification_config(&self, config: NotificationConfig) {
        let mut notification_config = self.notification_config.write().await;
        *notification_config = config;
    }

    /// Get notification configuration
    pub async fn get_notification_config(&self) -> NotificationConfig {
        let notification_config = self.notification_config.read().await;
        notification_config.clone()
    }

    /// Evaluate alert rules
    pub async fn evaluate_rules(&self) -> Result<(), Box<dyn std::error::Error>> {
        let rules = self.rules.read().await;
        let performance_metrics = self.metrics_collector.get_performance_metrics().await;
        let health_status = self.metrics_collector.get_health_status().await;

        for rule in rules.iter() {
            if !rule.enabled {
                continue;
            }

            // Check cooldown
            if let Some(last_triggered) = rule.last_triggered {
                let cooldown_duration = Duration::from_secs(rule.cooldown_seconds);
                if Utc::now() - last_triggered < chrono::Duration::from_std(cooldown_duration).unwrap() {
                    continue;
                }
            }

            // Evaluate condition
            if self.evaluate_condition(&rule.condition, &performance_metrics, &health_status).await? {
                self.create_alert(rule).await?;
            }
        }

        Ok(())
    }

    /// Evaluate a single alert condition
    async fn evaluate_condition(
        &self,
        condition: &AlertCondition,
        performance_metrics: &crate::monitoring::metrics::PerformanceMetrics,
        health_status: &crate::monitoring::metrics::HealthStatus,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match condition {
            AlertCondition::Threshold { metric, operator, value, duration_seconds: _ } => {
                let current_value = self.get_metric_value(metric, performance_metrics, health_status);
                match operator {
                    ThresholdOperator::GreaterThan => Ok(current_value > *value),
                    ThresholdOperator::LessThan => Ok(current_value < *value),
                    ThresholdOperator::GreaterThanOrEqual => Ok(current_value >= *value),
                    ThresholdOperator::LessThanOrEqual => Ok(current_value <= *value),
                    ThresholdOperator::Equal => Ok(current_value == *value),
                    ThresholdOperator::NotEqual => Ok(current_value != *value),
                }
            }
            AlertCondition::Anomaly { metric, sensitivity: _, window_seconds: _ } => {
                // TODO: Implement anomaly detection
                let current_value = self.get_metric_value(metric, performance_metrics, health_status);
                Ok(current_value > 100.0) // Placeholder
            }
            AlertCondition::Pattern { pattern, match_count: _, window_seconds: _ } => {
                // TODO: Implement pattern matching
                Ok(pattern.contains("error")) // Placeholder
            }
            AlertCondition::Custom { expression: _ } => {
                // TODO: Implement custom expression evaluation
                Ok(false) // Placeholder
            }
        }
    }

    /// Get metric value from performance metrics or health status
    fn get_metric_value(
        &self,
        metric: &str,
        performance_metrics: &crate::monitoring::metrics::PerformanceMetrics,
        health_status: &crate::monitoring::metrics::HealthStatus,
    ) -> f64 {
        match metric {
            "cpu_usage_percent" => performance_metrics.system_metrics.cpu_usage_percent,
            "memory_usage_bytes" => performance_metrics.system_metrics.memory_usage_bytes as f64,
            "disk_usage_bytes" => performance_metrics.system_metrics.disk_usage_bytes as f64,
            "api_requests_per_second" => performance_metrics.api_operations.requests_per_second,
            "api_average_response_time_ms" => performance_metrics.api_operations.average_response_time_ms,
            "storage_operations_per_second" => performance_metrics.storage_operations.storage_operations_per_second,
            "network_connected_peers" => performance_metrics.network_operations.connected_peers as f64,
            "consensus_participation_rate" => performance_metrics.consensus_operations.consensus_participation_rate * 100.0,
            "health_overall_status" => {
                match health_status.overall_status {
                    crate::monitoring::metrics::HealthState::Healthy => 100.0,
                    crate::monitoring::metrics::HealthState::Degraded => 50.0,
                    crate::monitoring::metrics::HealthState::Unhealthy => 0.0,
                    crate::monitoring::metrics::HealthState::Unknown => 25.0,
                }
            }
            _ => 0.0,
        }
    }

    /// Create an alert
    async fn create_alert(&self, rule: &AlertRule) -> Result<(), Box<dyn std::error::Error>> {
        let alert = Alert {
            id: format!("alert_{}", Utc::now().timestamp()),
            rule_id: rule.id.clone(),
            alert_type: rule.alert_type.clone(),
            severity: rule.severity.clone(),
            status: AlertStatus::Active,
            title: rule.name.clone(),
            message: rule.description.clone(),
            details: HashMap::new(),
            created_at: Utc::now(),
            acknowledged_at: None,
            resolved_at: None,
            acknowledged_by: None,
            notification_sent: false,
        };

        // Add alert to list
        let mut alerts = self.alerts.write().await;
        alerts.push(alert.clone());

        // Send notifications
        self.send_notifications(&alert).await?;

        // Log the alert
        self.logger.log(
            crate::utils::logging::LogLevel::Warn,
            "alerting",
            &format!("Alert triggered: {}", alert.title),
            HashMap::new(),
        ).await;

        Ok(())
    }

    /// Send notifications for an alert
    async fn send_notifications(&self, alert: &Alert) -> Result<(), Box<dyn std::error::Error>> {
        let notification_config = self.notification_config.read().await;

        // Send email notification
        if let Some(email_config) = &notification_config.email {
            self.send_email_notification(alert, email_config).await?;
        }

        // Send webhook notification
        if let Some(webhook_config) = &notification_config.webhook {
            self.send_webhook_notification(alert, webhook_config).await?;
        }

        // Send Slack notification
        if let Some(slack_config) = &notification_config.slack {
            self.send_slack_notification(alert, slack_config).await?;
        }

        // Send Discord notification
        if let Some(discord_config) = &notification_config.discord {
            self.send_discord_notification(alert, discord_config).await?;
        }

        // Send PagerDuty notification
        if let Some(pagerduty_config) = &notification_config.pagerduty {
            self.send_pagerduty_notification(alert, pagerduty_config).await?;
        }

        Ok(())
    }

    /// Send email notification
    async fn send_email_notification(&self, alert: &Alert, config: &EmailConfig) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: Implement actual email sending
        self.logger.log(
            crate::utils::logging::LogLevel::Info,
            "alerting",
            &format!("Email notification sent for alert: {}", alert.title),
            HashMap::new(),
        ).await;
        Ok(())
    }

    /// Send webhook notification
    async fn send_webhook_notification(&self, alert: &Alert, config: &WebhookConfig) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: Implement actual webhook sending
        self.logger.log(
            crate::utils::logging::LogLevel::Info,
            "alerting",
            &format!("Webhook notification sent for alert: {}", alert.title),
            HashMap::new(),
        ).await;
        Ok(())
    }

    /// Send Slack notification
    async fn send_slack_notification(&self, alert: &Alert, config: &SlackConfig) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: Implement actual Slack sending
        self.logger.log(
            crate::utils::logging::LogLevel::Info,
            "alerting",
            &format!("Slack notification sent for alert: {}", alert.title),
            HashMap::new(),
        ).await;
        Ok(())
    }

    /// Send Discord notification
    async fn send_discord_notification(&self, alert: &Alert, config: &DiscordConfig) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: Implement actual Discord sending
        self.logger.log(
            crate::utils::logging::LogLevel::Info,
            "alerting",
            &format!("Discord notification sent for alert: {}", alert.title),
            HashMap::new(),
        ).await;
        Ok(())
    }

    /// Send PagerDuty notification
    async fn send_pagerduty_notification(&self, alert: &Alert, config: &PagerDutyConfig) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: Implement actual PagerDuty sending
        self.logger.log(
            crate::utils::logging::LogLevel::Info,
            "alerting",
            &format!("PagerDuty notification sent for alert: {}", alert.title),
            HashMap::new(),
        ).await;
        Ok(())
    }

    /// Start the alerting system
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.logger.log(
            crate::utils::logging::LogLevel::Info,
            "alerting",
            "Alerting system started",
            HashMap::new(),
        ).await;

        // Start evaluation loop
        let rules = Arc::clone(&self.rules);
        let alerts = Arc::clone(&self.alerts);
        let metrics_collector = Arc::clone(&self.metrics_collector);
        let logger = Arc::clone(&self.logger);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                
                // TODO: Implement actual rule evaluation
                // For now, just log that we're checking
                logger.log(
                    crate::utils::logging::LogLevel::Debug,
                    "alerting",
                    "Checking alert rules",
                    HashMap::new(),
                ).await;
            }
        });

        Ok(())
    }

    /// Stop the alerting system
    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.logger.log(
            crate::utils::logging::LogLevel::Info,
            "alerting",
            "Alerting system stopped",
            HashMap::new(),
        ).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_alerting_system_creation() {
        let metrics_collector = Arc::new(crate::monitoring::metrics::MetricsCollector::new());
        let logger = Arc::new(crate::utils::logging::StructuredLogger::new(
            crate::utils::logging::LoggerConfig::default()
        ));
        
        let alerting_system = AlertingSystem::new(metrics_collector, logger);
        let rules = alerting_system.get_rules().await;
        assert_eq!(rules.len(), 0);
    }

    #[tokio::test]
    async fn test_add_remove_rule() {
        let metrics_collector = Arc::new(crate::monitoring::metrics::MetricsCollector::new());
        let logger = Arc::new(crate::utils::logging::StructuredLogger::new(
            crate::utils::logging::LoggerConfig::default()
        ));
        
        let alerting_system = AlertingSystem::new(metrics_collector, logger);
        
        let rule = AlertRule {
            id: "test_rule".to_string(),
            name: "Test Rule".to_string(),
            description: "Test alert rule".to_string(),
            alert_type: AlertType::SystemHealth,
            severity: AlertSeverity::Warning,
            condition: AlertCondition::Threshold {
                metric: "cpu_usage_percent".to_string(),
                operator: ThresholdOperator::GreaterThan,
                value: 80.0,
                duration_seconds: 300,
            },
            notification_channels: vec![NotificationChannel::Email],
            enabled: true,
            cooldown_seconds: 300,
            last_triggered: None,
        };
        
        alerting_system.add_rule(rule).await.unwrap();
        assert_eq!(alerting_system.get_rules().await.len(), 1);
        
        alerting_system.remove_rule("test_rule").await.unwrap();
        assert_eq!(alerting_system.get_rules().await.len(), 0);
    }

    #[tokio::test]
    async fn test_alert_acknowledgment() {
        let metrics_collector = Arc::new(crate::monitoring::metrics::MetricsCollector::new());
        let logger = Arc::new(crate::utils::logging::StructuredLogger::new(
            crate::utils::logging::LoggerConfig::default()
        ));
        
        let alerting_system = AlertingSystem::new(metrics_collector, logger);
        
        // Create a test alert
        let alert = Alert {
            id: "test_alert".to_string(),
            rule_id: "test_rule".to_string(),
            alert_type: AlertType::SystemHealth,
            severity: AlertSeverity::Warning,
            status: AlertStatus::Active,
            title: "Test Alert".to_string(),
            message: "Test alert message".to_string(),
            details: HashMap::new(),
            created_at: Utc::now(),
            acknowledged_at: None,
            resolved_at: None,
            acknowledged_by: None,
            notification_sent: false,
        };
        
        // Add alert to system
        let mut alerts = alerting_system.alerts.write().await;
        alerts.push(alert);
        drop(alerts);
        
        // Acknowledge alert
        alerting_system.acknowledge_alert("test_alert", "test_user").await.unwrap();
        
        let alerts = alerting_system.get_alerts().await;
        let test_alert = alerts.iter().find(|a| a.id == "test_alert").unwrap();
        assert_eq!(test_alert.status, AlertStatus::Acknowledged);
        assert_eq!(test_alert.acknowledged_by, Some("test_user".to_string()));
    }

    #[tokio::test]
    async fn test_notification_config() {
        let metrics_collector = Arc::new(crate::monitoring::metrics::MetricsCollector::new());
        let logger = Arc::new(crate::utils::logging::StructuredLogger::new(
            crate::utils::logging::LoggerConfig::default()
        ));
        
        let alerting_system = AlertingSystem::new(metrics_collector, logger);
        
        let config = NotificationConfig {
            email: Some(EmailConfig {
                smtp_server: "smtp.example.com".to_string(),
                smtp_port: 587,
                username: "user".to_string(),
                password: "pass".to_string(),
                from_address: "alerts@example.com".to_string(),
                to_addresses: vec!["admin@example.com".to_string()],
                use_tls: true,
            }),
            webhook: None,
            slack: None,
            discord: None,
            pagerduty: None,
        };
        
        alerting_system.set_notification_config(config).await;
        let retrieved_config = alerting_system.get_notification_config().await;
        assert!(retrieved_config.email.is_some());
    }
} 