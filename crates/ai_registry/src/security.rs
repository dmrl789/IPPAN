//! Security and authentication for AI Registry

use crate::errors::RegistryError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

/// Authentication token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    /// Token value
    pub token: String,
    /// Token type
    pub token_type: String,
    /// Expiration time
    pub expires_at: u64,
    /// Token scope
    pub scope: Vec<String>,
    /// User ID
    pub user_id: String,
    /// Issued at
    pub issued_at: u64,
}

/// User permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPermissions {
    /// Can register models
    pub can_register: bool,
    /// Can update models
    pub can_update: bool,
    /// Can delete models
    pub can_delete: bool,
    /// Can vote on proposals
    pub can_vote: bool,
    /// Can create proposals
    pub can_propose: bool,
    /// Can access admin functions
    pub can_admin: bool,
    /// Rate limit (requests per minute)
    pub rate_limit: u64,
}

/// Rate limiter
pub struct RateLimiter {
    /// Request counts per user
    requests: HashMap<String, Vec<Instant>>,
    /// Rate limit window
    window: Duration,
    /// Max requests per window
    max_requests: u64,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(window: Duration, max_requests: u64) -> Self {
        Self {
            requests: HashMap::new(),
            window,
            max_requests,
        }
    }

    /// Check if request is allowed
    pub fn is_allowed(&mut self, user_id: &str) -> bool {
        let now = Instant::now();
        let cutoff = now - self.window;

        // Get user's request history
        let user_requests = self
            .requests
            .entry(user_id.to_string())
            .or_insert_with(Vec::new);

        // Remove old requests
        user_requests.retain(|&time| time > cutoff);

        // Check if under limit
        if user_requests.len() < self.max_requests as usize {
            user_requests.push(now);
            true
        } else {
            false
        }
    }

    /// Get remaining requests for user
    pub fn remaining_requests(&self, user_id: &str) -> u64 {
        let now = Instant::now();
        let cutoff = now - self.window;

        if let Some(user_requests) = self.requests.get(user_id) {
            let recent_requests = user_requests.iter().filter(|&&time| time > cutoff).count();
            self.max_requests.saturating_sub(recent_requests as u64)
        } else {
            self.max_requests
        }
    }
}

/// Security manager for AI Registry
pub struct SecurityManager {
    /// Rate limiter
    rate_limiter: Arc<tokio::sync::RwLock<RateLimiter>>,
    /// User permissions
    permissions: HashMap<String, UserPermissions>,
    /// Active tokens
    tokens: HashMap<String, AuthToken>,
    /// Security configuration
    config: SecurityConfig,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable authentication
    pub enable_auth: bool,
    /// Enable rate limiting
    pub enable_rate_limiting: bool,
    /// Rate limit window (seconds)
    pub rate_limit_window: u64,
    /// Max requests per window
    pub max_requests_per_window: u64,
    /// Token expiration time (seconds)
    pub token_expiration: u64,
    /// Enable IP whitelisting
    pub enable_ip_whitelist: bool,
    /// Allowed IPs
    pub allowed_ips: Vec<String>,
    /// Enable audit logging
    pub enable_audit_logging: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_auth: true,
            enable_rate_limiting: true,
            rate_limit_window: 60, // 1 minute
            max_requests_per_window: 100,
            token_expiration: 3600, // 1 hour
            enable_ip_whitelist: false,
            allowed_ips: Vec::new(),
            enable_audit_logging: true,
        }
    }
}

impl SecurityManager {
    /// Create a new security manager
    pub fn new(config: SecurityConfig) -> Self {
        let rate_limiter = RateLimiter::new(
            Duration::from_secs(config.rate_limit_window),
            config.max_requests_per_window,
        );

        Self {
            rate_limiter: Arc::new(tokio::sync::RwLock::new(rate_limiter)),
            permissions: HashMap::new(),
            tokens: HashMap::new(),
            config,
        }
    }

    /// Authenticate user with token
    pub async fn authenticate(&self, token: &str) -> Result<Option<String>, RegistryError> {
        if !self.config.enable_auth {
            return Ok(Some("anonymous".to_string()));
        }

        if let Some(auth_token) = self.tokens.get(token) {
            // Check if token is expired
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if now > auth_token.expires_at {
                warn!("Token expired for user: {}", auth_token.user_id);
                return Ok(None);
            }

            Ok(Some(auth_token.user_id.clone()))
        } else {
            warn!("Invalid token provided");
            Ok(None)
        }
    }

    /// Check if user has permission
    pub fn has_permission(&self, user_id: &str, permission: &str) -> bool {
        if let Some(perms) = self.permissions.get(user_id) {
            match permission {
                "register" => perms.can_register,
                "update" => perms.can_update,
                "delete" => perms.can_delete,
                "vote" => perms.can_vote,
                "propose" => perms.can_propose,
                "admin" => perms.can_admin,
                _ => false,
            }
        } else {
            false
        }
    }

    /// Check rate limit for user
    pub async fn check_rate_limit(&self, user_id: &str) -> Result<bool, RegistryError> {
        if !self.config.enable_rate_limiting {
            return Ok(true);
        }

        let mut rate_limiter = self.rate_limiter.write().await;
        Ok(rate_limiter.is_allowed(user_id))
    }

    /// Get remaining requests for user
    pub async fn get_remaining_requests(&self, user_id: &str) -> u64 {
        let rate_limiter = self.rate_limiter.read().await;
        rate_limiter.remaining_requests(user_id)
    }

    /// Generate authentication token
    pub fn generate_token(
        &mut self,
        user_id: String,
        scope: Vec<String>,
    ) -> Result<AuthToken, RegistryError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let expires_at = now + self.config.token_expiration;

        let token = AuthToken {
            token: format!("ai_registry_{}_{}", user_id, now),
            token_type: "Bearer".to_string(),
            expires_at,
            scope: scope.clone(),
            user_id: user_id.clone(),
            issued_at: now,
        };

        self.tokens.insert(token.token.clone(), token.clone());
        info!("Generated token for user: {}", user_id);
        Ok(token)
    }

    /// Revoke authentication token
    pub fn revoke_token(&mut self, token: &str) -> Result<(), RegistryError> {
        if self.tokens.remove(token).is_some() {
            info!("Revoked token: {}", token);
            Ok(())
        } else {
            Err(RegistryError::InvalidToken("Token not found".to_string()))
        }
    }

    /// Set user permissions
    pub fn set_user_permissions(&mut self, user_id: String, permissions: UserPermissions) {
        self.permissions.insert(user_id.clone(), permissions);
        info!("Set permissions for user: {}", user_id);
    }

    /// Check IP whitelist
    pub fn is_ip_allowed(&self, ip: &str) -> bool {
        if !self.config.enable_ip_whitelist {
            return true;
        }

        self.config.allowed_ips.contains(&ip.to_string())
    }

    /// Log security event
    pub fn log_security_event(&self, event: &str, user_id: &str, details: &str) {
        if self.config.enable_audit_logging {
            info!(
                "Security event: {} - User: {} - Details: {}",
                event, user_id, details
            );
        }
    }

    /// Validate input for security
    pub fn validate_input(&self, input: &str, max_length: usize) -> Result<(), RegistryError> {
        if input.len() > max_length {
            return Err(RegistryError::InvalidInput(format!(
                "Input too long: {} > {}",
                input.len(),
                max_length
            )));
        }

        // Check for potential injection attacks
        if input.contains("script") || input.contains("javascript") || input.contains("onload") {
            return Err(RegistryError::InvalidInput(
                "Potentially malicious input detected".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate model data for security
    pub fn validate_model_data(&self, data: &[u8], max_size: usize) -> Result<(), RegistryError> {
        if data.len() > max_size {
            return Err(RegistryError::InvalidInput(format!(
                "Model data too large: {} > {}",
                data.len(),
                max_size
            )));
        }

        // Check for executable content
        if data.starts_with(b"MZ") || data.starts_with(b"\x7fELF") {
            return Err(RegistryError::InvalidInput(
                "Executable content detected in model data".to_string(),
            ));
        }

        Ok(())
    }

    /// Get security statistics
    pub fn get_security_stats(&self) -> SecurityStats {
        SecurityStats {
            total_tokens: self.tokens.len(),
            active_users: self.permissions.len(),
            rate_limiting_enabled: self.config.enable_rate_limiting,
            auth_enabled: self.config.enable_auth,
            ip_whitelist_enabled: self.config.enable_ip_whitelist,
        }
    }
}

/// Security statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityStats {
    /// Total number of active tokens
    pub total_tokens: usize,
    /// Number of active users
    pub active_users: usize,
    /// Whether rate limiting is enabled
    pub rate_limiting_enabled: bool,
    /// Whether authentication is enabled
    pub auth_enabled: bool,
    /// Whether IP whitelist is enabled
    pub ip_whitelist_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_security_manager_creation() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(config);
        assert!(manager.config.enable_auth);
        assert!(manager.config.enable_rate_limiting);
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let config = SecurityConfig {
            max_requests_per_window: 2,
            rate_limit_window: 60,
            ..Default::default()
        };
        let manager = SecurityManager::new(config);

        // First two requests should be allowed
        assert!(manager.check_rate_limit("user1").await.unwrap());
        assert!(manager.check_rate_limit("user1").await.unwrap());

        // Third request should be rate limited
        assert!(!manager.check_rate_limit("user1").await.unwrap());
    }

    #[tokio::test]
    async fn test_authentication() {
        let config = SecurityConfig::default();
        let mut manager = SecurityManager::new(config);

        // Generate token
        let token = manager
            .generate_token("user1".to_string(), vec!["read".to_string()])
            .unwrap();

        // Authenticate with valid token
        let user_id = manager.authenticate(&token.token).await.unwrap();
        assert_eq!(user_id, Some("user1".to_string()));

        // Revoke token
        manager.revoke_token(&token.token).unwrap();

        // Authentication should fail
        let user_id = manager.authenticate(&token.token).await.unwrap();
        assert_eq!(user_id, None);
    }

    #[tokio::test]
    async fn test_permissions() {
        let config = SecurityConfig::default();
        let mut manager = SecurityManager::new(config);

        // Set permissions
        let permissions = UserPermissions {
            can_register: true,
            can_update: false,
            can_delete: false,
            can_vote: true,
            can_propose: false,
            can_admin: false,
            rate_limit: 100,
        };
        manager.set_user_permissions("user1".to_string(), permissions);

        // Check permissions
        assert!(manager.has_permission("user1", "register"));
        assert!(!manager.has_permission("user1", "update"));
        assert!(manager.has_permission("user1", "vote"));
        assert!(!manager.has_permission("user1", "admin"));
    }

    #[tokio::test]
    async fn test_input_validation() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(config);

        // Valid input
        assert!(manager.validate_input("valid input", 100).is_ok());

        // Input too long
        assert!(manager
            .validate_input("x".repeat(101).as_str(), 100)
            .is_err());

        // Potentially malicious input
        assert!(manager.validate_input("script alert('xss')", 100).is_err());
    }

    #[tokio::test]
    async fn test_model_data_validation() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(config);

        // Valid data
        let valid_data = b"valid model data";
        assert!(manager.validate_model_data(valid_data, 1000).is_ok());

        // Data too large
        let large_data = vec![0u8; 1001];
        assert!(manager.validate_model_data(&large_data, 1000).is_err());

        // Executable content
        let exe_data = b"MZ\x90\x00\x03\x00\x00\x00\x04\x00\x00\x00";
        assert!(manager.validate_model_data(exe_data, 1000).is_err());
    }
}
