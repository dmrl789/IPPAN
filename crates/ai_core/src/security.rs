//! Security system for AI Core

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable security checks
    pub enabled: bool,
    /// Maximum execution time (microseconds)
    pub max_execution_time: u64,
    /// Maximum memory usage (bytes)
    pub max_memory_usage: u64,
    /// Allowed model sources
    pub allowed_sources: Vec<String>,
    /// Blocked model sources
    pub blocked_sources: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_execution_time: 10_000_000, // 10 seconds
            max_memory_usage: 1_000_000_000, // 1 GB
            allowed_sources: vec!["local".to_string(), "trusted".to_string()],
            blocked_sources: vec![],
        }
    }
}

/// Security system
#[derive(Debug, Clone)]
pub struct SecuritySystem {
    config: SecurityConfig,
    violations: Vec<SecurityViolation>,
}

/// Security violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityViolation {
    /// Violation type
    pub violation_type: ViolationType,
    /// Violation message
    pub message: String,
    /// Timestamp
    pub timestamp: u64,
    /// Severity
    pub severity: ViolationSeverity,
}

/// Violation type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationType {
    /// Execution time exceeded
    ExecutionTimeExceeded,
    /// Memory usage exceeded
    MemoryUsageExceeded,
    /// Unauthorized source
    UnauthorizedSource,
    /// Invalid model
    InvalidModel,
    /// Other violation
    Other(String),
}

/// Violation severity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationSeverity {
    /// Low severity
    Low,
    /// Medium severity
    Medium,
    /// High severity
    High,
    /// Critical severity
    Critical,
}

impl SecuritySystem {
    /// Create a new security system
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            config,
            violations: Vec::new(),
        }
    }

    /// Check if a source is allowed
    pub fn is_source_allowed(&self, source: &str) -> bool {
        if self.config.blocked_sources.contains(&source.to_string()) {
            return false;
        }
        
        if self.config.allowed_sources.is_empty() {
            return true;
        }
        
        self.config.allowed_sources.contains(&source.to_string())
    }

    /// Check execution time
    pub fn check_execution_time(&self, execution_time: u64) -> bool {
        execution_time <= self.config.max_execution_time
    }

    /// Check memory usage
    pub fn check_memory_usage(&self, memory_usage: u64) -> bool {
        memory_usage <= self.config.max_memory_usage
    }

    /// Record a security violation
    pub fn record_violation(&mut self, violation: SecurityViolation) {
        self.violations.push(violation);
    }

    /// Get all violations
    pub fn get_violations(&self) -> &[SecurityViolation] {
        &self.violations
    }

    /// Clear violations
    pub fn clear_violations(&mut self) {
        self.violations.clear();
    }
}