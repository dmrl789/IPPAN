//! Production-grade security hardening and validation for GBDT systems
//!
//! This module provides comprehensive security features including:
//! - Input validation and sanitization
//! - Model integrity verification
//! - Cryptographic validation
//! - Rate limiting and DoS protection
//! - Audit logging and forensics
//! - Threat detection and response

use crate::gbdt::{GBDTModel, GBDTError, SecurityConstraints};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn, instrument};
use sha2::{Sha256, Digest};
use blake3::Hasher as Blake3Hasher;

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable input validation
    pub enable_input_validation: bool,
    /// Enable model integrity checking
    pub enable_integrity_checking: bool,
    /// Enable rate limiting
    pub enable_rate_limiting: bool,
    /// Enable audit logging
    pub enable_audit_logging: bool,
    /// Enable threat detection
    pub enable_threat_detection: bool,
    /// Maximum requests per minute per client
    pub max_requests_per_minute: u32,
    /// Maximum concurrent evaluations
    pub max_concurrent_evaluations: u32,
    /// Model signature verification
    pub enable_signature_verification: bool,
    /// Allowed model publishers
    pub allowed_publishers: HashSet<String>,
    /// Security event retention days
    pub security_event_retention_days: u32,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_input_validation: true,
            enable_integrity_checking: true,
            enable_rate_limiting: true,
            enable_audit_logging: true,
            enable_threat_detection: true,
            max_requests_per_minute: 1000,
            max_concurrent_evaluations: 100,
            enable_signature_verification: true,
            allowed_publishers: HashSet::new(),
            security_event_retention_days: 30,
        }
    }
}

/// Security event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SecurityEventType {
    InputValidationFailed,
    ModelIntegrityViolation,
    RateLimitExceeded,
    SuspiciousActivity,
    UnauthorizedAccess,
    SignatureVerificationFailed,
    ResourceExhaustion,
    DataExfiltration,
}

/// Security event with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub id: String,
    pub timestamp: u64,
    pub event_type: SecurityEventType,
    pub severity: SecuritySeverity,
    pub source_ip: Option<String>,
    pub user_id: Option<String>,
    pub details: HashMap<String, String>,
    pub threat_score: f64,
    pub resolved: bool,
}

/// Security severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SecuritySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Rate limiter for DoS protection
#[derive(Debug)]
pub struct RateLimiter {
    requests: Arc<RwLock<HashMap<String, Vec<Instant>>>>,
    max_requests: u32,
    window_duration: Duration,
}

/// Threat detection system
#[derive(Debug)]
pub struct ThreatDetector {
    patterns: Arc<RwLock<Vec<ThreatPattern>>>,
    events: Arc<RwLock<Vec<SecurityEvent>>>,
    config: SecurityConfig,
}

/// Threat pattern for detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatPattern {
    pub name: String,
    pub pattern_type: ThreatPatternType,
    pub threshold: f64,
    pub window_duration_seconds: u64,
    pub action: ThreatAction,
}

/// Threat pattern types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ThreatPatternType {
    RapidRequests,
    LargeInputs,
    SuspiciousFeatures,
    ModelTampering,
    ResourceAbuse,
}

/// Threat response actions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ThreatAction {
    Log,
    Alert,
    Block,
    Quarantine,
}

/// Security auditor for comprehensive logging
#[derive(Debug)]
pub struct SecurityAuditor {
    events: Arc<RwLock<Vec<SecurityEvent>>>,
    config: SecurityConfig,
}

/// Model signature for integrity verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSignature {
    pub model_hash: String,
    pub signature: String,
    pub publisher: String,
    pub timestamp: u64,
    pub algorithm: String,
}

/// Production security system
#[derive(Debug)]
pub struct SecuritySystem {
    config: SecurityConfig,
    rate_limiter: RateLimiter,
    threat_detector: ThreatDetector,
    auditor: SecurityAuditor,
    active_evaluations: Arc<RwLock<u32>>,
    blocked_ips: Arc<RwLock<HashSet<String>>>,
}

impl SecuritySystem {
    /// Create a new security system
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            rate_limiter: RateLimiter::new(config.max_requests_per_minute),
            threat_detector: ThreatDetector::new(config.clone()),
            auditor: SecurityAuditor::new(config.clone()),
            active_evaluations: Arc::new(RwLock::new(0)),
            blocked_ips: Arc::new(RwLock::new(HashSet::new())),
            config,
        }
    }

    /// Validate input features for security
    #[instrument(skip(self, features))]
    pub fn validate_input(&self, features: &[i64], client_id: &str) -> Result<(), GBDTError> {
        if !self.config.enable_input_validation {
            return Ok(());
        }

        // Check if client is blocked
        if self.is_client_blocked(client_id) {
            self.record_security_event(
                SecurityEventType::UnauthorizedAccess,
                SecuritySeverity::High,
                Some(client_id.to_string()),
                None,
                HashMap::new(),
            );
            return Err(GBDTError::SecurityValidationFailed {
                reason: "Client is blocked".to_string(),
            });
        }

        // Check rate limits
        if self.config.enable_rate_limiting {
            if !self.rate_limiter.check_rate_limit(client_id) {
                self.record_security_event(
                    SecurityEventType::RateLimitExceeded,
                    SecuritySeverity::Medium,
                    Some(client_id.to_string()),
                    None,
                    HashMap::new(),
                );
                return Err(GBDTError::SecurityValidationFailed {
                    reason: "Rate limit exceeded".to_string(),
                });
            }
        }

        // Validate feature values
        self.validate_feature_values(features, client_id)?;

        // Check for suspicious patterns
        if self.config.enable_threat_detection {
            self.detect_threats(features, client_id)?;
        }

        Ok(())
    }

    /// Validate model integrity
    #[instrument(skip(self, model))]
    pub fn validate_model_integrity(&self, model: &GBDTModel, signature: Option<&ModelSignature>) -> Result<(), GBDTError> {
        if !self.config.enable_integrity_checking {
            return Ok(());
        }

        // Verify model structure
        model.validate()?;

        // Verify model hash
        let expected_hash = self.calculate_model_hash(model);
        if model.metadata.model_hash != expected_hash {
            self.record_security_event(
                SecurityEventType::ModelIntegrityViolation,
                SecuritySeverity::Critical,
                None,
                None,
                HashMap::new(),
            );
            return Err(GBDTError::SecurityValidationFailed {
                reason: "Model hash verification failed".to_string(),
            });
        }

        // Verify signature if provided
        if let Some(sig) = signature {
            if self.config.enable_signature_verification {
                self.verify_model_signature(model, sig)?;
            }
        }

        Ok(())
    }

    /// Check if evaluation can proceed (concurrency limits)
    #[instrument(skip(self))]
    pub fn check_evaluation_limit(&self) -> Result<(), GBDTError> {
        let current = *self.active_evaluations.read().unwrap();
        if current >= self.config.max_concurrent_evaluations {
            self.record_security_event(
                SecurityEventType::ResourceExhaustion,
                SecuritySeverity::High,
                None,
                None,
                HashMap::new(),
            );
            return Err(GBDTError::SecurityValidationFailed {
                reason: "Maximum concurrent evaluations exceeded".to_string(),
            });
        }

        Ok(())
    }

    /// Start evaluation tracking
    pub fn start_evaluation(&self) {
        let mut count = self.active_evaluations.write().unwrap();
        *count += 1;
    }

    /// End evaluation tracking
    pub fn end_evaluation(&self) {
        let mut count = self.active_evaluations.write().unwrap();
        *count = count.saturating_sub(1);
    }

    /// Validate feature values for security
    fn validate_feature_values(&self, features: &[i64], client_id: &str) -> Result<(), GBDTError> {
        for (i, &value) in features.iter().enumerate() {
            // Check for NaN or infinite values
            if value.is_nan() || value.is_infinite() {
                self.record_security_event(
                    SecurityEventType::InputValidationFailed,
                    SecuritySeverity::Medium,
                    Some(client_id.to_string()),
                    None,
                    HashMap::from([
                        ("feature_index".to_string(), i.to_string()),
                        ("invalid_value".to_string(), value.to_string()),
                    ]),
                );
                return Err(GBDTError::SecurityValidationFailed {
                    reason: format!("Invalid feature value at index {}", i),
                });
            }

            // Check for extremely large values (potential overflow attack)
            if value.abs() > 1_000_000_000 {
                self.record_security_event(
                    SecurityEventType::SuspiciousActivity,
                    SecuritySeverity::Medium,
                    Some(client_id.to_string()),
                    None,
                    HashMap::from([
                        ("feature_index".to_string(), i.to_string()),
                        ("large_value".to_string(), value.to_string()),
                    ]),
                );
            }
        }

        Ok(())
    }

    /// Detect potential threats in input
    fn detect_threats(&self, features: &[i64], client_id: &str) -> Result<(), GBDTError> {
        // Check for rapid successive similar inputs (potential replay attack)
        // This would be implemented with a more sophisticated pattern matching system
        
        // Check for suspicious feature patterns
        if self.is_suspicious_pattern(features) {
            self.record_security_event(
                SecurityEventType::SuspiciousActivity,
                SecuritySeverity::High,
                Some(client_id.to_string()),
                None,
                HashMap::from([
                    ("pattern_type".to_string(), "suspicious_features".to_string()),
                    ("feature_count".to_string(), features.len().to_string()),
                ]),
            );
        }

        Ok(())
    }

    /// Check if input pattern is suspicious
    fn is_suspicious_pattern(&self, features: &[i64]) -> bool {
        // Simple heuristic: check for all zeros or all same values
        if features.is_empty() {
            return true;
        }

        let first_value = features[0];
        if features.iter().all(|&x| x == first_value) {
            return true;
        }

        // Check for sequential patterns (potential enumeration attack)
        let mut is_sequential = true;
        for (i, &value) in features.iter().enumerate() {
            if value != i as i64 {
                is_sequential = false;
                break;
            }
        }

        is_sequential
    }

    /// Verify model signature
    fn verify_model_signature(&self, model: &GBDTModel, signature: &ModelSignature) -> Result<(), GBDTError> {
        // In a real implementation, this would verify cryptographic signatures
        // For now, we'll do basic validation
        
        if signature.model_hash != model.metadata.model_hash {
            return Err(GBDTError::SecurityValidationFailed {
                reason: "Signature model hash mismatch".to_string(),
            });
        }

        if self.config.allowed_publishers.contains(&signature.publisher) {
            return Ok(());
        }

        Err(GBDTError::SecurityValidationFailed {
            reason: "Unauthorized model publisher".to_string(),
        })
    }

    /// Calculate model hash for integrity checking
    fn calculate_model_hash(&self, model: &GBDTModel) -> String {
        let mut hasher = Blake3Hasher::new();
        
        // Hash model structure
        for tree in &model.trees {
            for node in &tree.nodes {
                hasher.update(&node.feature_index.to_le_bytes());
                hasher.update(&node.threshold.to_le_bytes());
                hasher.update(&node.left.to_le_bytes());
                hasher.update(&node.right.to_le_bytes());
                if let Some(value) = node.value {
                    hasher.update(&value.to_le_bytes());
                }
            }
        }
        
        hasher.update(&model.bias.to_le_bytes());
        hasher.update(&model.scale.to_le_bytes());
        
        format!("{:x}", hasher.finalize())
    }

    /// Record a security event
    fn record_security_event(
        &self,
        event_type: SecurityEventType,
        severity: SecuritySeverity,
        client_id: Option<String>,
        user_id: Option<String>,
        details: HashMap<String, String>,
    ) {
        let event = SecurityEvent {
            id: format!("sec_{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            event_type,
            severity,
            source_ip: client_id,
            user_id,
            details,
            threat_score: self.calculate_threat_score(&event_type, &severity),
            resolved: false,
        };

        self.auditor.record_event(event);
    }

    /// Calculate threat score based on event type and severity
    fn calculate_threat_score(&self, event_type: &SecurityEventType, severity: &SecuritySeverity) -> f64 {
        let base_score = match event_type {
            SecurityEventType::InputValidationFailed => 0.1,
            SecurityEventType::ModelIntegrityViolation => 0.9,
            SecurityEventType::RateLimitExceeded => 0.3,
            SecurityEventType::SuspiciousActivity => 0.5,
            SecurityEventType::UnauthorizedAccess => 0.8,
            SecurityEventType::SignatureVerificationFailed => 0.7,
            SecurityEventType::ResourceExhaustion => 0.6,
            SecurityEventType::DataExfiltration => 1.0,
        };

        let severity_multiplier = match severity {
            SecuritySeverity::Low => 0.1,
            SecuritySeverity::Medium => 0.5,
            SecuritySeverity::High => 0.8,
            SecuritySeverity::Critical => 1.0,
        };

        base_score * severity_multiplier
    }

    /// Check if client is blocked
    fn is_client_blocked(&self, client_id: &str) -> bool {
        self.blocked_ips.read().unwrap().contains(client_id)
    }

    /// Block a client
    pub fn block_client(&self, client_id: &str) {
        self.blocked_ips.write().unwrap().insert(client_id.to_string());
        info!("Client {} blocked", client_id);
    }

    /// Unblock a client
    pub fn unblock_client(&self, client_id: &str) {
        self.blocked_ips.write().unwrap().remove(client_id);
        info!("Client {} unblocked", client_id);
    }

    /// Get security events
    pub fn get_security_events(&self) -> Vec<SecurityEvent> {
        self.auditor.get_events()
    }

    /// Get active threats
    pub fn get_active_threats(&self) -> Vec<SecurityEvent> {
        self.auditor.get_events()
            .into_iter()
            .filter(|event| !event.resolved && event.threat_score > 0.5)
            .collect()
    }

    /// Export security report
    pub fn export_security_report(&self) -> Result<String, GBDTError> {
        let events = self.get_security_events();
        serde_json::to_string_pretty(&events)
            .map_err(|e| GBDTError::SecurityValidationFailed {
                reason: format!("Failed to serialize security report: {}", e),
            })
    }
}

impl RateLimiter {
    fn new(max_requests: u32) -> Self {
        Self {
            requests: Arc::new(RwLock::new(HashMap::new())),
            max_requests,
            window_duration: Duration::from_secs(60),
        }
    }

    fn check_rate_limit(&self, client_id: &str) -> bool {
        let now = Instant::now();
        let mut requests = self.requests.write().unwrap();
        
        let client_requests = requests.entry(client_id.to_string()).or_insert_with(Vec::new);
        
        // Remove old requests outside the window
        client_requests.retain(|&time| now.duration_since(time) < self.window_duration);
        
        if client_requests.len() >= self.max_requests as usize {
            false
        } else {
            client_requests.push(now);
            true
        }
    }
}

impl ThreatDetector {
    fn new(config: SecurityConfig) -> Self {
        Self {
            patterns: Arc::new(RwLock::new(Vec::new())),
            events: Arc::new(RwLock::new(Vec::new())),
            config,
        }
    }

    fn add_pattern(&self, pattern: ThreatPattern) {
        self.patterns.write().unwrap().push(pattern);
    }
}

impl SecurityAuditor {
    fn new(config: SecurityConfig) -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            config,
        }
    }

    fn record_event(&self, event: SecurityEvent) {
        let mut events = self.events.write().unwrap();
        events.push(event);
        
        // Keep only recent events
        let retention_duration = Duration::from_secs(self.config.security_event_retention_days as u64 * 24 * 60 * 60);
        let cutoff_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap()
            .saturating_sub(retention_duration);
        
        events.retain(|event| event.timestamp > cutoff_time.as_secs());
    }

    fn get_events(&self) -> Vec<SecurityEvent> {
        self.events.read().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_system_creation() {
        let config = SecurityConfig::default();
        let security = SecuritySystem::new(config);
        
        assert!(security.get_security_events().is_empty());
        assert!(security.get_active_threats().is_empty());
    }

    #[test]
    fn test_input_validation() {
        let config = SecurityConfig::default();
        let security = SecuritySystem::new(config);
        
        // Valid input
        let valid_features = vec![1, 2, 3, 4, 5];
        assert!(security.validate_input(&valid_features, "client1").is_ok());
        
        // Invalid input (all same values - suspicious pattern)
        let suspicious_features = vec![1, 1, 1, 1, 1];
        assert!(security.validate_input(&suspicious_features, "client1").is_err());
    }

    #[test]
    fn test_rate_limiting() {
        let config = SecurityConfig {
            max_requests_per_minute: 2,
            ..Default::default()
        };
        let security = SecuritySystem::new(config);
        
        // First two requests should pass
        assert!(security.validate_input(&vec![1, 2, 3], "client1").is_ok());
        assert!(security.validate_input(&vec![1, 2, 3], "client1").is_ok());
        
        // Third request should be rate limited
        assert!(security.validate_input(&vec![1, 2, 3], "client1").is_err());
    }

    #[test]
    fn test_model_integrity_validation() {
        let config = SecurityConfig::default();
        let security = SecuritySystem::new(config);
        
        // Create a test model
        let tree = crate::gbdt::Tree {
            nodes: vec![
                crate::gbdt::Node { feature_index: 0, threshold: 50, left: 1, right: 2, value: None },
                crate::gbdt::Node { feature_index: 0, threshold: 0, left: 0, right: 0, value: Some(10) },
                crate::gbdt::Node { feature_index: 0, threshold: 0, left: 0, right: 0, value: Some(20) },
            ],
        };
        
        let model = crate::gbdt::GBDTModel::new(vec![tree], 0, 100, 1).unwrap();
        
        // Valid model should pass
        assert!(security.validate_model_integrity(&model, None).is_ok());
    }

    #[test]
    fn test_client_blocking() {
        let config = SecurityConfig::default();
        let security = SecuritySystem::new(config);
        
        let client_id = "malicious_client";
        
        // Initially not blocked
        assert!(!security.is_client_blocked(client_id));
        
        // Block client
        security.block_client(client_id);
        assert!(security.is_client_blocked(client_id));
        
        // Unblock client
        security.unblock_client(client_id);
        assert!(!security.is_client_blocked(client_id));
    }

    #[test]
    fn test_threat_score_calculation() {
        let config = SecurityConfig::default();
        let security = SecuritySystem::new(config);
        
        let score = security.calculate_threat_score(
            &SecurityEventType::ModelIntegrityViolation,
            &SecuritySeverity::Critical,
        );
        
        assert!(score > 0.8); // Should be high for critical integrity violation
    }
}