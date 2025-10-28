//! Security system for AI Core
//!
//! Provides sandboxing, rate limiting, audit logging, and execution validation
//! for deterministic AI inference (used in IPPAN/FinDAG’s GBDT modules).

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable security checks
    pub enabled: bool,
    /// Enable input validation
    pub enable_input_validation: bool,
    /// Enable integrity checking
    pub enable_integrity_checking: bool,
    /// Enable rate limiting
    pub enable_rate_limiting: bool,
    /// Maximum requests per minute
    pub max_requests_per_minute: u32,
    /// Maximum execution time (seconds)
    pub max_execution_time: u64,
    /// Maximum memory usage (bytes)
    pub max_memory_usage: u64,
    /// Allowed model sources
    pub allowed_sources: Vec<String>,
    /// Blocked model sources
    pub blocked_sources: Vec<String>,
    /// Enable sandboxing
    pub enable_sandboxing: bool,
    /// Security policies
    pub policies: SecurityPolicies,
}

/// Security policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicies {
    /// Allow network access
    pub allow_network: bool,
    /// Allow file system access
    pub allow_filesystem: bool,
    /// Allow system calls
    pub allow_system_calls: bool,
    /// Require model signing
    pub require_model_signing: bool,
    /// Enable audit logging
    pub enable_audit_logging: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            enable_input_validation: true,
            enable_integrity_checking: true,
            enable_rate_limiting: true,
            max_requests_per_minute: 1000,
            max_execution_time: 30,
            max_memory_usage: 1024 * 1024 * 1024, // 1 GB
            allowed_sources: vec!["local".to_string()],
            blocked_sources: vec![],
            enable_sandboxing: true,
            policies: SecurityPolicies {
                allow_network: false,
                allow_filesystem: false,
                allow_system_calls: false,
                require_model_signing: true,
                enable_audit_logging: true,
            },
        }
    }
}

/// Security system for AI Core
#[derive(Debug)]
pub struct SecuritySystem {
    config: SecurityConfig,
    audit_log: Vec<AuditEntry>,
}

/// Audit entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: u64,
    pub event_type: String,
    pub details: String,
    pub severity: SecuritySeverity,
    pub user_id: Option<String>,
    pub resource: Option<String>,
}

/// Security severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecuritySeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl SecuritySystem {
    /// Create a new security system
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            config,
            audit_log: Vec::new(),
        }
    }

    /// Log an audit entry
    pub fn log_audit(
        &mut self,
        event_type: String,
        details: String,
        severity: SecuritySeverity,
        user_id: Option<String>,
        resource: Option<String>,
    ) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.audit_log.push(AuditEntry {
            timestamp,
            event_type,
            details,
            severity,
            user_id,
            resource,
        });
    }

    /// Check if a model source is allowed
    pub fn is_source_allowed(&self, source: &str) -> bool {
        if self.config.blocked_sources.contains(&source.to_string()) {
            return false;
        }

        if self.config.allowed_sources.is_empty() {
            return true;
        }

        self.config.allowed_sources.contains(&source.to_string())
    }

    /// Validate execution parameters (time + memory)
    pub fn validate_execution(
        &self,
        execution_time: u64,
        memory_usage: u64,
    ) -> Result<(), SecurityError> {
        if execution_time > self.config.max_execution_time {
            return Err(SecurityError::ExecutionTimeExceeded {
                actual: execution_time,
                max: self.config.max_execution_time,
            });
        }

        if memory_usage > self.config.max_memory_usage {
            return Err(SecurityError::MemoryUsageExceeded {
                actual: memory_usage,
                max: self.config.max_memory_usage,
            });
        }

        Ok(())
    }

    /// Retrieve all audit log entries
    pub fn get_audit_log(&self) -> &Vec<AuditEntry> {
        &self.audit_log
    }

    /// Retrieve only high or critical severity violations
    pub fn get_violations(&self) -> Vec<&AuditEntry> {
        self.audit_log
            .iter()
            .filter(|entry| matches!(entry.severity, SecuritySeverity::High | SecuritySeverity::Critical))
            .collect()
    }
}

/// Security error definitions
#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("Execution time exceeded: {actual}s > {max}s")]
    ExecutionTimeExceeded { actual: u64, max: u64 },

    #[error("Memory usage exceeded: {actual} bytes > {max} bytes")]
    MemoryUsageExceeded { actual: u64, max: u64 },

    #[error("Source not allowed: {src}")]
    SourceNotAllowed { src: String },

    #[error("Model not signed")]
    ModelNotSigned,

    #[error("Security policy violation: {policy}")]
    PolicyViolation { policy: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_security_config() {
        let cfg = SecurityConfig::default();
        assert!(cfg.enabled);
        assert!(cfg.enable_input_validation);
        assert_eq!(cfg.max_requests_per_minute, 1000);
        assert!(cfg.policies.require_model_signing);
    }

    #[test]
    fn test_is_source_allowed() {
        let cfg = SecurityConfig::default();
        let sys = SecuritySystem::new(cfg);
        assert!(sys.is_source_allowed("local"));
        assert!(!sys.is_source_allowed("unauthorized"));
    }

    #[test]
    fn test_validate_execution_limits() {
        let sys = SecuritySystem::new(SecurityConfig::default());
        assert!(sys.validate_execution(5, 512 * 1024 * 1024).is_ok());
        assert!(sys.validate_execution(100, 512 * 1024 * 1024).is_err());
    }

    #[test]
    fn test_audit_logging() {
        let mut sys = SecuritySystem::new(SecurityConfig::default());
        sys.log_audit(
            "TestEvent".into(),
            "Testing log".into(),
            SecuritySeverity::Low,
            Some("user1".into()),
            Some("resourceA".into()),
        );
        assert_eq!(sys.get_audit_log().len(), 1);
    }

    #[test]
    fn test_violations_filter() {
        let mut sys = SecuritySystem::new(SecurityConfig::default());
        sys.log_audit(
            "Warning".into(),
            "Minor issue".into(),
            SecuritySeverity::Low,
            None,
            None,
        );
        sys.log_audit(
            "Critical breach".into(),
            "Unauthorized model load".into(),
            SecuritySeverity::Critical,
            None,
            None,
        );
        let v = sys.get_violations();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].event_type, "Critical breach");
    }
}
