#![allow(clippy::type_complexity)]
#![allow(clippy::vec_init_then_push)]
#![allow(clippy::collapsible_match)]
pub mod audit;
pub mod circuit_breaker;
pub mod rate_limiter;
pub mod validation;

pub use audit::{AuditEvent, AuditLogger, SecurityEvent};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerState};
pub use rate_limiter::{RateLimitConfig, RateLimitError, RateLimiter};
pub use validation::{InputValidator, ValidationError, ValidationRule};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::time::{Duration, SystemTime};

/// Security configuration for the IPPAN node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,
    /// Maximum request size in bytes
    pub max_request_size: usize,
    /// Request timeout in seconds
    pub request_timeout: u64,
    /// Enable audit logging
    pub enable_audit_logging: bool,
    /// Audit log file path
    pub audit_log_path: String,
    /// Maximum failed attempts before blocking
    pub max_failed_attempts: u32,
    /// Block duration in seconds
    pub block_duration: u64,
    /// Enable IP whitelisting
    pub enable_ip_whitelist: bool,
    /// Whitelisted IP addresses
    pub whitelisted_ips: Vec<IpAddr>,
    /// Enable DDoS protection
    pub enable_ddos_protection: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            rate_limit: RateLimitConfig::default(),
            max_request_size: 1024 * 1024, // 1MB
            request_timeout: 30,
            enable_audit_logging: true,
            audit_log_path: "/var/log/ippan/audit.log".to_string(),
            max_failed_attempts: 5,
            block_duration: 300, // 5 minutes
            enable_ip_whitelist: false,
            whitelisted_ips: vec![],
            enable_ddos_protection: true,
        }
    }
}

/// Security manager for coordinating all security features
pub struct SecurityManager {
    config: SecurityConfig,
    rate_limiter: RateLimiter,
    audit_logger: AuditLogger,
    validator: InputValidator,
    circuit_breaker: CircuitBreaker,
    failed_attempts: parking_lot::RwLock<std::collections::HashMap<IpAddr, (u32, SystemTime)>>,
}

impl SecurityManager {
    pub fn new(config: SecurityConfig) -> Result<Self> {
        let rate_limiter = RateLimiter::new(config.rate_limit.clone())?;
        let audit_logger = AuditLogger::new(&config.audit_log_path)?;
        let validator = InputValidator::new();
        let circuit_breaker = CircuitBreaker::new(CircuitBreakerConfig::default());

        Ok(Self {
            config,
            rate_limiter,
            audit_logger,
            validator,
            circuit_breaker,
            failed_attempts: parking_lot::RwLock::new(std::collections::HashMap::new()),
        })
    }

    /// Check if a request should be allowed
    pub async fn check_request(&self, ip: IpAddr, endpoint: &str) -> Result<bool, SecurityError> {
        // Check if IP is blocked due to failed attempts
        if self.is_ip_blocked(ip) {
            self.audit_logger
                .log_security_event(SecurityEvent::BlockedRequest {
                    ip,
                    endpoint: endpoint.to_string(),
                    reason: "Too many failed attempts".to_string(),
                    timestamp: SystemTime::now(),
                })
                .await?;
            return Err(SecurityError::IpBlocked);
        }

        // Check IP whitelist if enabled
        if self.config.enable_ip_whitelist && !self.config.whitelisted_ips.contains(&ip) {
            self.audit_logger
                .log_security_event(SecurityEvent::UnauthorizedAccess {
                    ip,
                    endpoint: endpoint.to_string(),
                    timestamp: SystemTime::now(),
                })
                .await?;
            return Err(SecurityError::IpNotWhitelisted);
        }

        // Check rate limits
        if !self.rate_limiter.check_rate_limit(ip, endpoint).await? {
            self.audit_logger
                .log_security_event(SecurityEvent::RateLimitExceeded {
                    ip,
                    endpoint: endpoint.to_string(),
                    timestamp: SystemTime::now(),
                })
                .await?;
            return Err(SecurityError::RateLimitExceeded);
        }

        // Check circuit breaker
        if !self.circuit_breaker.can_execute().await {
            return Err(SecurityError::CircuitBreakerOpen);
        }

        Ok(true)
    }

    /// Record a failed attempt
    pub async fn record_failed_attempt(
        &self,
        ip: IpAddr,
        endpoint: &str,
        reason: &str,
    ) -> Result<()> {
        let attempt_count = {
            let mut attempts = self.failed_attempts.write();
            let (count, _) = attempts.entry(ip).or_insert((0, SystemTime::now()));
            *count += 1;
            *count
        };

        self.audit_logger
            .log_security_event(SecurityEvent::FailedAttempt {
                ip,
                endpoint: endpoint.to_string(),
                reason: reason.to_string(),
                attempt_count,
                timestamp: SystemTime::now(),
            })
            .await?;

        Ok(())
    }

    /// Record a successful request
    pub async fn record_success(&self, ip: IpAddr, endpoint: &str) -> Result<()> {
        // Reset failed attempts on success
        {
            let mut attempts = self.failed_attempts.write();
            attempts.remove(&ip);
        }

        self.circuit_breaker.record_success().await;

        self.audit_logger
            .log_security_event(SecurityEvent::SuccessfulRequest {
                ip,
                endpoint: endpoint.to_string(),
                timestamp: SystemTime::now(),
            })
            .await?;

        Ok(())
    }

    /// Record a failure
    pub async fn record_failure(&self, ip: IpAddr, endpoint: &str, error: &str) -> Result<()> {
        self.circuit_breaker.record_failure().await;

        self.audit_logger
            .log_security_event(SecurityEvent::RequestFailure {
                ip,
                endpoint: endpoint.to_string(),
                error: error.to_string(),
                timestamp: SystemTime::now(),
            })
            .await?;

        Ok(())
    }

    /// Validate input data
    pub fn validate_input<T>(
        &self,
        data: &T,
        rules: &[ValidationRule],
    ) -> Result<(), ValidationError>
    where
        T: Serialize,
    {
        self.validator.validate(data, rules)
    }

    /// Check if an IP is currently blocked
    fn is_ip_blocked(&self, ip: IpAddr) -> bool {
        let attempts = self.failed_attempts.read();
        if let Some((count, timestamp)) = attempts.get(&ip) {
            if *count >= self.config.max_failed_attempts {
                let block_duration = Duration::from_secs(self.config.block_duration);
                return timestamp.elapsed().unwrap_or(Duration::ZERO) < block_duration;
            }
        }
        false
    }

    /// Clean up expired blocked IPs
    pub async fn cleanup_expired_blocks(&self) {
        let mut attempts = self.failed_attempts.write();
        let block_duration = Duration::from_secs(self.config.block_duration);

        attempts.retain(|_, (_, timestamp)| {
            timestamp.elapsed().unwrap_or(Duration::ZERO) < block_duration
        });
    }

    /// Get security statistics
    pub fn get_stats(&self) -> SecurityStats {
        let blocked_ips = self.failed_attempts.read().len();
        let circuit_breaker_state = self.circuit_breaker.get_state();

        SecurityStats {
            blocked_ips,
            circuit_breaker_state,
            rate_limit_stats: self.rate_limiter.get_stats(),
        }
    }
}

/// Security-related errors
#[derive(thiserror::Error, Debug)]
pub enum SecurityError {
    #[error("IP address is blocked due to too many failed attempts")]
    IpBlocked,
    #[error("IP address is not whitelisted")]
    IpNotWhitelisted,
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Circuit breaker is open")]
    CircuitBreakerOpen,
    #[error("Input validation failed: {0}")]
    ValidationFailed(#[from] ValidationError),
    #[error("Audit logging failed: {0}")]
    AuditFailed(#[from] anyhow::Error),
}

/// Security statistics
#[derive(Debug, Serialize)]
pub struct SecurityStats {
    pub blocked_ips: usize,
    pub circuit_breaker_state: CircuitBreakerState,
    pub rate_limit_stats: serde_json::Value,
}

#[cfg(all(test, feature = "enable-tests"))]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_security_manager_creation() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(config);
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_request_checking() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(config).unwrap();
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        let result = manager.check_request(ip, "/health").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_failed_attempts_blocking() {
        let mut config = SecurityConfig::default();
        config.max_failed_attempts = 2;

        let manager = SecurityManager::new(config).unwrap();
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        // Record failed attempts
        manager
            .record_failed_attempt(ip, "/api", "Invalid credentials")
            .await
            .unwrap();
        manager
            .record_failed_attempt(ip, "/api", "Invalid credentials")
            .await
            .unwrap();

        // Should be blocked now
        let result = manager.check_request(ip, "/api").await;
        assert!(matches!(result, Err(SecurityError::IpBlocked)));
    }

    #[tokio::test]
    async fn test_ip_whitelist() {
        let mut config = SecurityConfig::default();
        config.enable_ip_whitelist = true;
        config.whitelisted_ips = vec![IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))];

        let manager = SecurityManager::new(config).unwrap();

        // Whitelisted IP should pass
        let result = manager
            .check_request(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), "/api")
            .await;
        assert!(result.is_ok());

        // Non-whitelisted IP should fail
        let result = manager
            .check_request(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), "/api")
            .await;
        assert!(matches!(result, Err(SecurityError::IpNotWhitelisted)));
    }
}
