//! Network security module for IPPAN
//! 
//! This module provides comprehensive network-level security including
//! encryption, authentication, certificate management, and threat detection.

use crate::{Result, IppanError};
use crate::security::key_management::KeyManagementService;
use crate::utils::crypto::{generate_aes_key, encrypt_aes_gcm, decrypt_aes_gcm, generate_nonce};
use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use rand::RngCore;
use rustls::{ServerConfig, ClientConfig, Certificate, PrivateKey, RootCertStore};
use rustls_pemfile::{certs, pkcs8_private_keys};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use tokio::sync::RwLock;
use tokio_rustls::{TlsAcceptor, TlsConnector};
use std::net::SocketAddr;
use chrono::{DateTime, Utc};

/// Network security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSecurityConfig {
    /// Enable TLS encryption
    pub enable_tls: bool,
    /// Enable certificate pinning
    pub enable_certificate_pinning: bool,
    /// Enable mutual TLS authentication
    pub enable_mutual_tls: bool,
    /// Enable message encryption
    pub enable_message_encryption: bool,
    /// Enable connection authentication
    pub enable_connection_auth: bool,
    /// Enable rate limiting
    pub enable_rate_limiting: bool,
    /// Enable DDoS protection
    pub enable_ddos_protection: bool,
    /// Maximum connections per IP
    pub max_connections_per_ip: usize,
    /// Connection timeout (seconds)
    pub connection_timeout: u64,
    /// Certificate validation strictness
    pub strict_certificate_validation: bool,
    /// Enable perfect forward secrecy
    pub enable_pfs: bool,
    /// Enable quantum-resistant encryption
    pub enable_quantum_resistant: bool,
}

impl Default for NetworkSecurityConfig {
    fn default() -> Self {
        Self {
            enable_tls: true,
            enable_certificate_pinning: true,
            enable_mutual_tls: true,
            enable_message_encryption: true,
            enable_connection_auth: true,
            enable_rate_limiting: true,
            enable_ddos_protection: true,
            max_connections_per_ip: 10,
            connection_timeout: 30,
            strict_certificate_validation: true,
            enable_pfs: true,
            enable_quantum_resistant: false, // Will be enabled when quantum module is ready
        }
    }
}

/// Network security manager
pub struct NetworkSecurityManager {
    /// Security configuration
    config: NetworkSecurityConfig,
    /// Key management service
    key_service: Arc<RwLock<KeyManagementService>>,
    /// Certificate store
    certificate_store: Arc<RwLock<CertificateStore>>,
    /// Rate limiter
    rate_limiter: Arc<RwLock<RateLimiter>>,
    /// Connection authenticator
    connection_authenticator: Arc<RwLock<ConnectionAuthenticator>>,
    /// Message encryptor
    message_encryptor: Arc<RwLock<MessageEncryptor>>,
    /// Threat detector
    threat_detector: Arc<RwLock<ThreatDetector>>,
    /// TLS components
    tls_acceptor: Option<TlsAcceptor>,
    tls_connector: Option<TlsConnector>,
}

/// Certificate store for managing network certificates
#[derive(Debug)]
pub struct CertificateStore {
    /// Node's own certificate
    node_certificate: Option<CertificateInfo>,
    /// Trusted peer certificates
    trusted_certificates: HashMap<String, CertificateInfo>,
    /// Certificate revocation list
    revoked_certificates: HashMap<String, DateTime<Utc>>,
    /// Certificate pinning store
    pinned_certificates: HashMap<String, [u8; 32]>,
}

/// Certificate information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateInfo {
    /// Certificate ID
    pub cert_id: String,
    /// Certificate data (PEM encoded)
    pub cert_data: Vec<u8>,
    /// Private key data (encrypted)
    pub private_key_data: Vec<u8>,
    /// Certificate fingerprint
    pub fingerprint: [u8; 32],
    /// Issued timestamp
    pub issued_at: DateTime<Utc>,
    /// Expires at timestamp
    pub expires_at: DateTime<Utc>,
    /// Certificate status
    pub status: CertificateStatus,
    /// Node ID associated with certificate
    pub node_id: [u8; 32],
}

/// Certificate status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CertificateStatus {
    Valid,
    Expired,
    Revoked,
    Compromised,
}

/// Rate limiter for connection protection
#[derive(Debug)]
pub struct RateLimiter {
    /// Connection limits per IP
    connection_limits: HashMap<String, ConnectionLimit>,
    /// Maximum connections per IP
    max_connections_per_ip: usize,
    /// Time window for rate limiting (seconds)
    time_window: u64,
}

/// Connection limit information
#[derive(Debug, Clone)]
pub struct ConnectionLimit {
    /// Current connection count
    pub current_connections: usize,
    /// Last connection time
    pub last_connection: SystemTime,
    /// Blocked until (if rate limited)
    pub blocked_until: Option<SystemTime>,
    /// Connection attempts in time window
    pub attempts_in_window: Vec<SystemTime>,
}

/// Connection authenticator
#[derive(Debug)]
pub struct ConnectionAuthenticator {
    /// Node's signing key
    signing_key: SigningKey,
    /// Node's verifying key
    verifying_key: VerifyingKey,
    /// Authenticated connections
    authenticated_connections: HashMap<String, AuthenticatedConnection>,
}

/// Authenticated connection information
#[derive(Debug, Clone)]
pub struct AuthenticatedConnection {
    /// Connection ID
    pub connection_id: String,
    /// Remote node ID
    pub remote_node_id: [u8; 32],
    /// Remote public key
    pub remote_public_key: [u8; 32],
    /// Authentication timestamp
    pub authenticated_at: DateTime<Utc>,
    /// Session key for encryption
    pub session_key: Option<[u8; 32]>,
    /// Connection security level
    pub security_level: SecurityLevel,
}

/// Security level
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityLevel {
    None,
    Basic,
    Enhanced,
    Maximum,
}

/// Message encryptor for end-to-end encryption
#[derive(Debug)]
pub struct MessageEncryptor {
    /// Session keys for connections
    session_keys: HashMap<String, [u8; 32]>,
    /// Message nonces to prevent replay attacks
    used_nonces: HashMap<String, Vec<[u8; 12]>>,
    /// Nonce window size
    nonce_window_size: usize,
}

/// Threat detector for network security monitoring
#[derive(Debug)]
pub struct ThreatDetector {
    /// Suspicious IP addresses
    suspicious_ips: HashMap<String, SuspiciousActivity>,
    /// Attack patterns
    attack_patterns: HashMap<String, AttackPattern>,
    /// Security events
    security_events: Vec<SecurityEvent>,
}

/// Suspicious activity information
#[derive(Debug, Clone)]
pub struct SuspiciousActivity {
    /// IP address
    pub ip: String,
    /// Activity type
    pub activity_type: SuspiciousActivityType,
    /// First detected
    pub first_detected: DateTime<Utc>,
    /// Last detected
    pub last_detected: DateTime<Utc>,
    /// Severity score
    pub severity_score: u8,
    /// Blocked status
    pub is_blocked: bool,
}

/// Suspicious activity types
#[derive(Debug, Clone, PartialEq)]
pub enum SuspiciousActivityType {
    PortScanning,
    BruteForce,
    DDoS,
    MalformedPackets,
    UnauthorizedAccess,
    CertificateSpoofing,
    ReplayAttack,
}

/// Attack pattern
#[derive(Debug, Clone)]
pub struct AttackPattern {
    /// Pattern name
    pub name: String,
    /// Pattern signature
    pub signature: Vec<u8>,
    /// Severity level
    pub severity: u8,
    /// Detection rules
    pub detection_rules: Vec<String>,
}

/// Security event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event type
    pub event_type: SecurityEventType,
    /// Source IP
    pub source_ip: String,
    /// Event description
    pub description: String,
    /// Severity level
    pub severity: u8,
    /// Event data
    pub event_data: Option<Vec<u8>>,
}

/// Security event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SecurityEventType {
    ConnectionAttempt,
    AuthenticationFailure,
    CertificateValidationFailure,
    RateLimitExceeded,
    SuspiciousActivity,
    AttackDetected,
    SecurityPolicyViolation,
}

impl NetworkSecurityManager {
    /// Create a new network security manager
    pub async fn new(config: NetworkSecurityConfig) -> Result<Self> {
        // Initialize key management service
        let mut key_service = KeyManagementService::new();
        key_service.initialize().await?;

        // Initialize certificate store
        let certificate_store = Arc::new(RwLock::new(CertificateStore::new()));

        // Initialize rate limiter
        let rate_limiter = Arc::new(RwLock::new(RateLimiter::new(
            config.max_connections_per_ip,
            config.connection_timeout,
        )));

        // Initialize connection authenticator
        let connection_authenticator = Arc::new(RwLock::new(ConnectionAuthenticator::new().await?));

        // Initialize message encryptor
        let message_encryptor = Arc::new(RwLock::new(MessageEncryptor::new()));

        // Initialize threat detector
        let threat_detector = Arc::new(RwLock::new(ThreatDetector::new()));

        // Initialize TLS components if enabled
        let (tls_acceptor, tls_connector) = if config.enable_tls {
            Self::initialize_tls(&config, &certificate_store).await?
        } else {
            (None, None)
        };

        Ok(Self {
            config,
            key_service: Arc::new(RwLock::new(key_service)),
            certificate_store,
            rate_limiter,
            connection_authenticator,
            message_encryptor,
            threat_detector,
            tls_acceptor,
            tls_connector,
        })
    }

    /// Initialize TLS components
    async fn initialize_tls(
        config: &NetworkSecurityConfig,
        cert_store: &Arc<RwLock<CertificateStore>>,
    ) -> Result<(Option<TlsAcceptor>, Option<TlsConnector>)> {
        let cert_store = cert_store.read().await;
        
        // Create server configuration
        let server_config = if let Some(node_cert) = &cert_store.node_certificate {
            let cert_chain = vec![Certificate(node_cert.cert_data.clone())];
            let private_key = PrivateKey(node_cert.private_key_data.clone());
            
            let server_config = ServerConfig::builder()
                .with_safe_defaults()
                .with_no_client_auth()
                .with_single_cert(cert_chain, private_key)?;
            
            Some(server_config)
        } else {
            None
        };

        // Create client configuration
        let mut root_store = RootCertStore::empty();
        
        // Add trusted certificates
        for (_, cert_info) in &cert_store.trusted_certificates {
            if cert_info.status == CertificateStatus::Valid {
                let cert = Certificate(cert_info.cert_data.clone());
                root_store.add(&cert)?;
            }
        }

        let client_config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let tls_acceptor = server_config.map(|config| TlsAcceptor::from(Arc::new(config)));
        let tls_connector = Some(TlsConnector::from(Arc::new(client_config)));

        Ok((tls_acceptor, tls_connector))
    }

    /// Authenticate a connection
    pub async fn authenticate_connection(
        &self,
        connection_id: &str,
        remote_addr: SocketAddr,
        handshake_data: &[u8],
    ) -> Result<AuthenticatedConnection> {
        // Check rate limiting
        if !self.is_connection_allowed(&remote_addr.ip().to_string()).await? {
            return Err(IppanError::Security("Connection rate limited".to_string()));
        }

        // Parse handshake data
        let handshake: HandshakeMessage = serde_json::from_slice(handshake_data)
            .map_err(|e| IppanError::Serialization(format!("Invalid handshake: {}", e)))?;

        // Verify handshake signature
        let authenticator = self.connection_authenticator.read().await;
        let is_valid = authenticator.verify_handshake(&handshake).await?;
        drop(authenticator);

        if !is_valid {
            // Record security event
            self.record_security_event(SecurityEventType::AuthenticationFailure, 
                &remote_addr.ip().to_string(), "Invalid handshake signature").await;
            return Err(IppanError::Security("Invalid handshake signature".to_string()));
        }

        // Create authenticated connection
        let authenticated_conn = AuthenticatedConnection {
            connection_id: connection_id.to_string(),
            remote_node_id: handshake.node_id,
            remote_public_key: handshake.public_key,
            authenticated_at: Utc::now(),
            session_key: None,
            security_level: SecurityLevel::Enhanced,
        };

        // Store authenticated connection
        let mut authenticator = self.connection_authenticator.write().await;
        authenticator.authenticated_connections.insert(
            connection_id.to_string(),
            authenticated_conn.clone(),
        );

        Ok(authenticated_conn)
    }

    /// Encrypt a message for a connection
    pub async fn encrypt_message(
        &self,
        connection_id: &str,
        message: &[u8],
    ) -> Result<Vec<u8>> {
        if !self.config.enable_message_encryption {
            return Ok(message.to_vec());
        }

        let encryptor = self.message_encryptor.read().await;
        encryptor.encrypt_message(connection_id, message).await
    }

    /// Decrypt a message from a connection
    pub async fn decrypt_message(
        &self,
        connection_id: &str,
        encrypted_message: &[u8],
    ) -> Result<Vec<u8>> {
        if !self.config.enable_message_encryption {
            return Ok(encrypted_message.to_vec());
        }

        let encryptor = self.message_encryptor.read().await;
        encryptor.decrypt_message(connection_id, encrypted_message).await
    }

    /// Check if connection is allowed (rate limiting)
    async fn is_connection_allowed(&self, ip: &str) -> Result<bool> {
        let rate_limiter = self.rate_limiter.read().await;
        Ok(rate_limiter.is_connection_allowed(ip))
    }

    /// Record a security event
    async fn record_security_event(
        &self,
        event_type: SecurityEventType,
        source_ip: &str,
        description: &str,
    ) {
        let event = SecurityEvent {
            timestamp: Utc::now(),
            event_type,
            source_ip: source_ip.to_string(),
            description: description.to_string(),
            severity: 5, // Default severity
            event_data: None,
        };

        let mut threat_detector = self.threat_detector.write().await;
        threat_detector.record_security_event(event);
    }

    /// Get security statistics
    pub async fn get_security_stats(&self) -> Result<SecurityStats> {
        let rate_limiter = self.rate_limiter.read().await;
        let authenticator = self.connection_authenticator.read().await;
        let threat_detector = self.threat_detector.read().await;

        Ok(SecurityStats {
            total_connections: authenticator.authenticated_connections.len(),
            rate_limited_ips: rate_limiter.connection_limits.len(),
            security_events: threat_detector.security_events.len(),
            suspicious_ips: threat_detector.suspicious_ips.len(),
            tls_enabled: self.config.enable_tls,
            mutual_tls_enabled: self.config.enable_mutual_tls,
            message_encryption_enabled: self.config.enable_message_encryption,
        })
    }
}

/// Handshake message for connection authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeMessage {
    /// Node ID
    pub node_id: [u8; 32],
    /// Public key
    pub public_key: [u8; 32],
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Nonce
    pub nonce: [u8; 32],
    /// Signature (as Vec<u8> for serde compatibility)
    pub signature: Vec<u8>,
}

/// Security statistics
#[derive(Debug, Serialize)]
pub struct SecurityStats {
    pub total_connections: usize,
    pub rate_limited_ips: usize,
    pub security_events: usize,
    pub suspicious_ips: usize,
    pub tls_enabled: bool,
    pub mutual_tls_enabled: bool,
    pub message_encryption_enabled: bool,
}

// Implementation for CertificateStore
impl CertificateStore {
    fn new() -> Self {
        Self {
            node_certificate: None,
            trusted_certificates: HashMap::new(),
            revoked_certificates: HashMap::new(),
            pinned_certificates: HashMap::new(),
        }
    }
}

// Implementation for RateLimiter
impl RateLimiter {
    fn new(max_connections_per_ip: usize, time_window: u64) -> Self {
        Self {
            connection_limits: HashMap::new(),
            max_connections_per_ip,
            time_window,
        }
    }

    fn is_connection_allowed(&self, ip: &str) -> bool {
        if let Some(limit) = self.connection_limits.get(ip) {
            // Check if currently blocked
            if let Some(blocked_until) = limit.blocked_until {
                if SystemTime::now() < blocked_until {
                    return false;
                }
            }

            // Check connection count
            if limit.current_connections >= self.max_connections_per_ip {
                return false;
            }

            // Check rate limiting in time window
            let now = SystemTime::now();
            let window_start = now - Duration::from_secs(self.time_window);
            
            let recent_attempts = limit.attempts_in_window
                .iter()
                .filter(|&&attempt_time| attempt_time > window_start)
                .count();

            if recent_attempts >= self.max_connections_per_ip {
                return false;
            }
        }

        true
    }
}

// Implementation for ConnectionAuthenticator
impl ConnectionAuthenticator {
    async fn new() -> Result<Self> {
        let mut rng = rand::thread_rng();
        let mut signing_key_bytes = [0u8; 32];
        rng.fill_bytes(&mut signing_key_bytes);
        let signing_key = SigningKey::from_bytes(&signing_key_bytes);
        let verifying_key = signing_key.verifying_key();

        Ok(Self {
            signing_key,
            verifying_key,
            authenticated_connections: HashMap::new(),
        })
    }

    async fn verify_handshake(&self, handshake: &HandshakeMessage) -> Result<bool> {
        // Create message to verify
        let message = format!("{}:{}:{}:{}",
            hex::encode(handshake.node_id),
            hex::encode(handshake.public_key),
            handshake.timestamp.to_rfc3339(),
            hex::encode(handshake.nonce)
        );

        // Verify signature
        let verifying_key = VerifyingKey::from_bytes(&handshake.public_key)
            .map_err(|e| IppanError::Crypto(format!("Invalid public key: {}", e)))?;
        
        let signature = if handshake.signature.len() == 64 {
            let mut sig_bytes = [0u8; 64];
            sig_bytes.copy_from_slice(&handshake.signature);
            Signature::from_bytes(&sig_bytes)
        } else {
            return Ok(false);
        };
        
        Ok(verifying_key.verify(message.as_bytes(), &signature).is_ok())
    }
}

// Implementation for MessageEncryptor
impl MessageEncryptor {
    fn new() -> Self {
        Self {
            session_keys: HashMap::new(),
            used_nonces: HashMap::new(),
            nonce_window_size: 1000,
        }
    }

    async fn encrypt_message(&self, connection_id: &str, message: &[u8]) -> Result<Vec<u8>> {
        if let Some(session_key) = self.session_keys.get(connection_id) {
            let nonce = generate_nonce();
            let encrypted = encrypt_aes_gcm(session_key, &nonce, message)
                .map_err(|e| IppanError::Crypto(format!("Encryption failed: {}", e)))?;

            // Prepend nonce
            let mut result = Vec::new();
            result.extend_from_slice(&nonce);
            result.extend_from_slice(&encrypted);
            Ok(result)
        } else {
            Err(IppanError::Security("No session key for connection".to_string()))
        }
    }

    async fn decrypt_message(&self, connection_id: &str, encrypted_message: &[u8]) -> Result<Vec<u8>> {
        if let Some(session_key) = self.session_keys.get(connection_id) {
            if encrypted_message.len() < 12 {
                return Err(IppanError::Crypto("Invalid encrypted message length".to_string()));
            }

            let nonce = &encrypted_message[0..12];
            let encrypted = &encrypted_message[12..];

            let mut nonce_array = [0u8; 12];
            nonce_array.copy_from_slice(nonce);

            // Check for replay attacks
            if self.is_nonce_used(connection_id, &nonce_array) {
                return Err(IppanError::Security("Replay attack detected".to_string()));
            }

            let decrypted = decrypt_aes_gcm(session_key, &nonce_array, encrypted)
                .map_err(|e| IppanError::Crypto(format!("Decryption failed: {}", e)))?;

            Ok(decrypted)
        } else {
            Err(IppanError::Security("No session key for connection".to_string()))
        }
    }

    fn is_nonce_used(&self, connection_id: &str, nonce: &[u8; 12]) -> bool {
        if let Some(used_nonces) = self.used_nonces.get(connection_id) {
            used_nonces.contains(nonce)
        } else {
            false
        }
    }
}

// Implementation for ThreatDetector
impl ThreatDetector {
    fn new() -> Self {
        Self {
            suspicious_ips: HashMap::new(),
            attack_patterns: HashMap::new(),
            security_events: Vec::new(),
        }
    }

    fn record_security_event(&mut self, event: SecurityEvent) {
        self.security_events.push(event);
        
        // Keep only last 10000 events
        if self.security_events.len() > 10000 {
            self.security_events.drain(0..1000);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_security_manager() {
        let config = NetworkSecurityConfig::default();
        let manager = NetworkSecurityManager::new(config).await.unwrap();
        
        let stats = manager.get_security_stats().await.unwrap();
        assert!(stats.tls_enabled);
        assert!(stats.mutual_tls_enabled);
        assert!(stats.message_encryption_enabled);
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let rate_limiter = RateLimiter::new(5, 60);
        
        // Test normal connections
        assert!(rate_limiter.is_connection_allowed("192.168.1.1"));
        assert!(rate_limiter.is_connection_allowed("192.168.1.2"));
    }

    #[tokio::test]
    async fn test_message_encryption() {
        let encryptor = MessageEncryptor::new();
        let connection_id = "test_connection";
        
        // This would need a session key to work properly
        // For now, just test the structure
        assert_eq!(encryptor.session_keys.len(), 0);
    }
}
