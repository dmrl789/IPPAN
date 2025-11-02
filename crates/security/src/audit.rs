use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::net::IpAddr;
use std::path::Path;
use std::time::SystemTime;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Audit logger for security events
pub struct AuditLogger {
    log_file: Mutex<std::fs::File>,
    log_path: String,
}

impl AuditLogger {
    pub fn new(log_path: &str) -> Result<Self> {
        // Ensure the directory exists
        if let Some(parent) = Path::new(log_path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;

        Ok(Self {
            log_file: Mutex::new(log_file),
            log_path: log_path.to_string(),
        })
    }

    /// Log a security event
    pub async fn log_security_event(&self, event: SecurityEvent) -> Result<()> {
        let audit_event = AuditEvent {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            event_type: "security".to_string(),
            severity: event.severity(),
            details: serde_json::to_value(&event)?,
        };

        self.log_audit_event(audit_event).await
    }

    /// Log a general audit event
    pub async fn log_audit_event(&self, event: AuditEvent) -> Result<()> {
        let log_entry = serde_json::to_string(&event)?;

        // Log to file
        {
            let mut file = self.log_file.lock().await;
            writeln!(file, "{}", log_entry)?;
            file.flush()?;
        }

        // Also log to tracing based on severity
        match event.severity.as_str() {
            "critical" | "high" => error!("Security event: {}", log_entry),
            "medium" => warn!("Security event: {}", log_entry),
            _ => info!("Security event: {}", log_entry),
        }

        Ok(())
    }

    /// Rotate log file if it gets too large
    pub async fn rotate_if_needed(&self, max_size_bytes: u64) -> Result<()> {
        let metadata = std::fs::metadata(&self.log_path)?;

        if metadata.len() > max_size_bytes {
            let backup_path = format!(
                "{}.{}",
                self.log_path,
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)?
                    .as_secs()
            );

            // Close current file and rename it
            drop(self.log_file.lock().await);
            std::fs::rename(&self.log_path, backup_path)?;

            // Create new file
            let new_file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.log_path)?;

            *self.log_file.lock().await = new_file;

            info!("Rotated audit log file: {}", self.log_path);
        }

        Ok(())
    }
}

/// Generic audit event structure
#[derive(Debug, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub timestamp: SystemTime,
    pub event_type: String,
    pub severity: String,
    pub details: serde_json::Value,
}

/// Security-specific events
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SecurityEvent {
    FailedAttempt {
        ip: IpAddr,
        endpoint: String,
        reason: String,
        attempt_count: u32,
        timestamp: SystemTime,
    },
    BlockedRequest {
        ip: IpAddr,
        endpoint: String,
        reason: String,
        timestamp: SystemTime,
    },
    UnauthorizedAccess {
        ip: IpAddr,
        endpoint: String,
        timestamp: SystemTime,
    },
    RateLimitExceeded {
        ip: IpAddr,
        endpoint: String,
        timestamp: SystemTime,
    },
    SuspiciousActivity {
        ip: IpAddr,
        activity_type: String,
        details: String,
        timestamp: SystemTime,
    },
    SuccessfulRequest {
        ip: IpAddr,
        endpoint: String,
        timestamp: SystemTime,
    },
    RequestFailure {
        ip: IpAddr,
        endpoint: String,
        error: String,
        timestamp: SystemTime,
    },
    ConfigurationChange {
        changed_by: String,
        change_type: String,
        old_value: Option<String>,
        new_value: String,
        timestamp: SystemTime,
    },
    SystemEvent {
        event_type: String,
        description: String,
        timestamp: SystemTime,
    },
}

impl SecurityEvent {
    /// Get the severity level of the security event
    pub fn severity(&self) -> String {
        match self {
            SecurityEvent::FailedAttempt { attempt_count, .. } => {
                if *attempt_count >= 5 {
                    "high".to_string()
                } else if *attempt_count >= 3 {
                    "medium".to_string()
                } else {
                    "low".to_string()
                }
            }
            SecurityEvent::BlockedRequest { .. } => "high".to_string(),
            SecurityEvent::UnauthorizedAccess { .. } => "high".to_string(),
            SecurityEvent::RateLimitExceeded { .. } => "medium".to_string(),
            SecurityEvent::SuspiciousActivity { .. } => "high".to_string(),
            SecurityEvent::SuccessfulRequest { .. } => "info".to_string(),
            SecurityEvent::RequestFailure { .. } => "low".to_string(),
            SecurityEvent::ConfigurationChange { .. } => "medium".to_string(),
            SecurityEvent::SystemEvent { .. } => "info".to_string(),
        }
    }

    /// Get the IP address associated with the event, if any
    pub fn ip_address(&self) -> Option<IpAddr> {
        match self {
            SecurityEvent::FailedAttempt { ip, .. } => Some(*ip),
            SecurityEvent::BlockedRequest { ip, .. } => Some(*ip),
            SecurityEvent::UnauthorizedAccess { ip, .. } => Some(*ip),
            SecurityEvent::RateLimitExceeded { ip, .. } => Some(*ip),
            SecurityEvent::SuspiciousActivity { ip, .. } => Some(*ip),
            SecurityEvent::SuccessfulRequest { ip, .. } => Some(*ip),
            SecurityEvent::RequestFailure { ip, .. } => Some(*ip),
            _ => None,
        }
    }

    /// Get the endpoint associated with the event, if any
    pub fn endpoint(&self) -> Option<&str> {
        match self {
            SecurityEvent::FailedAttempt { endpoint, .. } => Some(endpoint),
            SecurityEvent::BlockedRequest { endpoint, .. } => Some(endpoint),
            SecurityEvent::UnauthorizedAccess { endpoint, .. } => Some(endpoint),
            SecurityEvent::RateLimitExceeded { endpoint, .. } => Some(endpoint),
            SecurityEvent::SuccessfulRequest { endpoint, .. } => Some(endpoint),
            SecurityEvent::RequestFailure { endpoint, .. } => Some(endpoint),
            _ => None,
        }
    }
}

/// Audit log analyzer for detecting patterns
pub struct AuditAnalyzer {
    events: Vec<AuditEvent>,
}

impl Default for AuditAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditAnalyzer {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Add an event for analysis
    pub fn add_event(&mut self, event: AuditEvent) {
        self.events.push(event);

        // Keep only recent events to prevent memory bloat
        if self.events.len() > 10000 {
            self.events.drain(0..1000);
        }
    }

    /// Detect suspicious patterns
    pub fn detect_suspicious_patterns(&self) -> Vec<SuspiciousPattern> {
        let mut patterns = Vec::new();

        // Detect brute force attempts
        patterns.extend(self.detect_brute_force_attempts());

        // Detect distributed attacks
        patterns.extend(self.detect_distributed_attacks());

        // Detect unusual access patterns
        patterns.extend(self.detect_unusual_access_patterns());

        patterns
    }

    fn detect_brute_force_attempts(&self) -> Vec<SuspiciousPattern> {
        let mut patterns = Vec::new();
        let mut ip_failures: std::collections::HashMap<IpAddr, u32> =
            std::collections::HashMap::new();

        // Count failed attempts per IP in the last hour
        let one_hour_ago = SystemTime::now() - std::time::Duration::from_secs(3600);

        for event in &self.events {
            if event.timestamp > one_hour_ago {
                if let Ok(security_event) =
                    serde_json::from_value::<SecurityEvent>(event.details.clone())
                {
                    if let SecurityEvent::FailedAttempt { ip, .. } = security_event {
                        *ip_failures.entry(ip).or_insert(0) += 1;
                    }
                }
            }
        }

        for (ip, count) in ip_failures {
            if count > 10 {
                patterns.push(SuspiciousPattern {
                    pattern_type: "brute_force".to_string(),
                    description: format!(
                        "IP {} has {} failed attempts in the last hour",
                        ip, count
                    ),
                    severity: "high".to_string(),
                    ip_address: Some(ip),
                    timestamp: SystemTime::now(),
                });
            }
        }

        patterns
    }

    fn detect_distributed_attacks(&self) -> Vec<SuspiciousPattern> {
        let mut patterns = Vec::new();
        let mut endpoint_failures: std::collections::HashMap<String, u32> =
            std::collections::HashMap::new();

        // Count failures per endpoint in the last 10 minutes
        let ten_minutes_ago = SystemTime::now() - std::time::Duration::from_secs(600);

        for event in &self.events {
            if event.timestamp > ten_minutes_ago {
                if let Ok(security_event) =
                    serde_json::from_value::<SecurityEvent>(event.details.clone())
                {
                    if let Some(endpoint) = security_event.endpoint() {
                        if matches!(
                            security_event,
                            SecurityEvent::FailedAttempt { .. }
                                | SecurityEvent::RateLimitExceeded { .. }
                        ) {
                            *endpoint_failures.entry(endpoint.to_string()).or_insert(0) += 1;
                        }
                    }
                }
            }
        }

        for (endpoint, count) in endpoint_failures {
            if count > 100 {
                patterns.push(SuspiciousPattern {
                    pattern_type: "distributed_attack".to_string(),
                    description: format!(
                        "Endpoint {} has {} failures in the last 10 minutes",
                        endpoint, count
                    ),
                    severity: "high".to_string(),
                    ip_address: None,
                    timestamp: SystemTime::now(),
                });
            }
        }

        patterns
    }

    fn detect_unusual_access_patterns(&self) -> Vec<SuspiciousPattern> {
        // This is a simplified implementation
        // In production, you'd want more sophisticated pattern detection
        Vec::new()
    }
}

/// Suspicious pattern detected by the analyzer
#[derive(Debug, Serialize)]
pub struct SuspiciousPattern {
    pub pattern_type: String,
    pub description: String,
    pub severity: String,
    pub ip_address: Option<IpAddr>,
    pub timestamp: SystemTime,
}

#[cfg(all(test, feature = "enable-tests"))]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_audit_logger_creation() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("audit.log");
        let logger = AuditLogger::new(log_path.to_str().unwrap());
        assert!(logger.is_ok());
    }

    #[tokio::test]
    async fn test_security_event_logging() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("audit.log");
        let logger = AuditLogger::new(log_path.to_str().unwrap()).unwrap();

        let event = SecurityEvent::FailedAttempt {
            ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            endpoint: "/api".to_string(),
            reason: "Invalid credentials".to_string(),
            attempt_count: 1,
            timestamp: SystemTime::now(),
        };

        let result = logger.log_security_event(event).await;
        assert!(result.is_ok());

        // Check that the log file was created and contains data
        let log_content = std::fs::read_to_string(&log_path).unwrap();
        assert!(!log_content.is_empty());
        assert!(log_content.contains("FailedAttempt"));
    }

    #[test]
    fn test_security_event_severity() {
        let event = SecurityEvent::FailedAttempt {
            ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            endpoint: "/api".to_string(),
            reason: "Invalid credentials".to_string(),
            attempt_count: 5,
            timestamp: SystemTime::now(),
        };

        assert_eq!(event.severity(), "high");
    }

    #[test]
    fn test_audit_analyzer() {
        let mut analyzer = AuditAnalyzer::new();

        let event = AuditEvent {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            event_type: "security".to_string(),
            severity: "high".to_string(),
            details: serde_json::to_value(SecurityEvent::FailedAttempt {
                ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                endpoint: "/api".to_string(),
                reason: "Invalid credentials".to_string(),
                attempt_count: 1,
                timestamp: SystemTime::now(),
            })
            .unwrap(),
        };

        analyzer.add_event(event);
        let patterns = analyzer.detect_suspicious_patterns();

        // With just one event, no patterns should be detected
        assert!(patterns.is_empty());
    }
}
