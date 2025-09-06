//! Security Integration Tests for IPPAN
//! 
//! Tests the complete security system integration including quantum-resistant cryptography,
//! key management, network security, and audit logging

use crate::{
    security::{
        key_management::{KeyManagementService, KeyRotationScheduler, KeyAuditLog},
        quantum::{QuantumResistantCrypto, QuantumKeyDistribution},
    },
    network::security::{
        NetworkSecurityManager, NetworkSecurityConfig, HandshakeMessage,
        CertificateManager, DDoSProtection, ThreatDetectionSystem,
    },
    wallet::ed25519::{Ed25519Manager, Ed25519KeyPair},
    consensus::{BFTProposal, BFTVote, BFTPhase},
};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};
use ed25519_dalek::{SigningKey, VerifyingKey, Signature};
use rand::rngs::OsRng;
use rand::RngCore;

/// Security integration test configuration
pub struct SecurityIntegrationConfig {
    pub key_rotation_interval: Duration,
    pub max_failed_attempts: u32,
    pub lockout_duration: Duration,
    pub enable_quantum_crypto: bool,
    pub enable_network_security: bool,
    pub enable_audit_logging: bool,
}

impl Default for SecurityIntegrationConfig {
    fn default() -> Self {
        Self {
            key_rotation_interval: Duration::from_secs(3600), // 1 hour
            max_failed_attempts: 5,
            lockout_duration: Duration::from_secs(300), // 5 minutes
            enable_quantum_crypto: true,
            enable_network_security: true,
            enable_audit_logging: true,
        }
    }
}

/// Security integration test suite
pub struct SecurityIntegrationTestSuite {
    config: SecurityIntegrationConfig,
    key_manager: Arc<RwLock<Ed25519Manager>>,
    security_manager: Arc<RwLock<NetworkSecurityManager>>,
    audit_log: Arc<RwLock<KeyAuditLog>>,
}

impl SecurityIntegrationTestSuite {
    /// Create a new security integration test suite
    pub fn new(config: SecurityIntegrationConfig) -> Self {
        let key_manager = Arc::new(RwLock::new(Ed25519Manager::new()));
        let security_config = NetworkSecurityConfig::default();
        let security_manager = Arc::new(RwLock::new(NetworkSecurityManager::new(security_config)));
        let audit_log = Arc::new(RwLock::new(KeyAuditLog::new()));
        
        Self {
            config,
            key_manager,
            security_manager,
            audit_log,
        }
    }

    /// Run all security integration tests
    pub async fn run_all_tests(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("🔒 Starting security integration test suite...");

        // Test key management system
        self.test_key_management().await?;
        log::info!("✅ Key management tests passed");

        // Test quantum-resistant cryptography
        self.test_quantum_cryptography().await?;
        log::info!("✅ Quantum-resistant cryptography tests passed");

        // Test network security
        self.test_network_security().await?;
        log::info!("✅ Network security tests passed");

        // Test BFT consensus security
        self.test_bft_security().await?;
        log::info!("✅ BFT consensus security tests passed");

        // Test key rotation
        self.test_key_rotation().await?;
        log::info!("✅ Key rotation tests passed");

        // Test audit logging
        self.test_audit_logging().await?;
        log::info!("✅ Audit logging tests passed");

        // Test threat detection
        self.test_threat_detection().await?;
        log::info!("✅ Threat detection tests passed");

        // Test DDoS protection
        self.test_ddos_protection().await?;
        log::info!("✅ DDoS protection tests passed");

        // Test end-to-end security
        self.test_end_to_end_security().await?;
        log::info!("✅ End-to-end security tests passed");

        log::info!("🎉 All security integration tests passed!");
        Ok(())
    }

    /// Test key management system integration
    async fn test_key_management(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Testing key management system...");

        let key_manager = self.key_manager.clone();

        // Test key generation
        let keypair = key_manager.write().await.generate_key_pair().await?;
        assert_eq!(keypair.public_key.len(), 32);
        assert_eq!(keypair.private_key.len(), 32);

        // Test key storage
        let key_id = keypair.public_key.clone();
        key_manager.write().await.add_key_pair(keypair.clone())?;

        // Test key retrieval
        let retrieved_keypair = key_manager.read().await.get_key_pair(&key_id)?;
        assert_eq!(retrieved_keypair.public_key, key_id);

        // Test key statistics
        let stats = key_manager.read().await.get_key_stats();
        assert!(stats.total_keys > 0);

        // Test key expiration
        assert!(keypair.expires_at > Instant::now());

        // Test key versioning
        assert_eq!(keypair.version, 1);

        Ok(())
    }

    /// Test quantum-resistant cryptography integration
    async fn test_quantum_cryptography(&self) -> Result<(), Box<dyn std::Error>> {
        log::info!("Testing quantum-resistant cryptography...");

        if !self.config.enable_quantum_crypto {
            log::info!("Quantum cryptography disabled, skipping tests");
            return Ok(());
        }

        // Test quantum key generation
        let quantum_crypto = QuantumResistantCrypto::new();
        let quantum_keypair = quantum_crypto.generate_keypair().await?;
        assert!(!quantum_keypair.public_key.is_empty());
        assert!(!quantum_keypair.private_key.is_empty());

        // Test quantum encryption
        let test_data = b"Hello, quantum world!";
        let encrypted_data = quantum_crypto.encrypt(&quantum_keypair.public_key, test_data).await?;
        assert_ne!(encrypted_data, test_data);

        // Test quantum decryption
        let decrypted_data = quantum_crypto.decrypt(&quantum_keypair.private_key, &encrypted_data).await?;
        assert_eq!(decrypted_data, test_data);

        // Test quantum signatures
        let signature = quantum_crypto.sign(&quantum_keypair.private_key, test_data).await?;
        assert!(!signature.is_empty());

        // Test quantum signature verification
        let is_valid = quantum_crypto.verify(&quantum_keypair.public_key, test_data, &signature).await?;
        assert!(is_valid);

        // Test quantum key distribution
        let qkd = QuantumKeyDistribution::new();
        let shared_key = qkd.generate_shared_key(&quantum_keypair.public_key).await?;
        assert!(!shared_key.is_empty());

        Ok(())
    }

    /// Test network security integration
    async fn test_network_security(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Testing network security...");

        let security_manager = self.security_manager.clone();

        // Test TLS configuration
        let tls_config = security_manager.read().await.get_tls_config();
        assert!(tls_config.enable_tls);

        // Test certificate management
        let cert_manager = security_manager.read().await.get_certificate_manager();
        let certificate = cert_manager.generate_certificate("test.ippan.network").await?;
        assert!(!certificate.is_empty());

        // Test certificate validation
        let is_valid = cert_manager.validate_certificate(&certificate).await?;
        assert!(is_valid);

        // Test mutual authentication
        let auth_result = security_manager.read().await.authenticate_peer("test_peer").await?;
        assert!(auth_result.is_authenticated);

        // Test message encryption
        let test_message = b"Secure message";
        let encrypted_message = security_manager.read().await.encrypt_message(test_message).await?;
        assert_ne!(encrypted_message, test_message);

        // Test message decryption
        let decrypted_message = security_manager.read().await.decrypt_message(&encrypted_message).await?;
        assert_eq!(decrypted_message, test_message);

        Ok(())
    }

    /// Test BFT consensus security integration
    async fn test_bft_security(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Testing BFT consensus security...");

        // Generate test keys
        let mut rng = OsRng;
        let mut signing_key_bytes = [0u8; 32];
        rng.fill_bytes(&mut signing_key_bytes);
        let signing_key = SigningKey::from_bytes(&signing_key_bytes);
        let verifying_key = signing_key.verifying_key();

        // Test BFT proposal creation and validation
        let proposal_data = b"BFT proposal data";
        let signature = signing_key.sign(proposal_data);
        let signature_bytes = signature.to_bytes().to_vec();

        let proposal = BFTProposal {
            proposer_id: [1u8; 32],
            round: 1,
            block_hash: [2u8; 32],
            signature: hex::encode(signature_bytes),
            timestamp: Instant::now(),
        };

        // Test proposal signature validation
        let signature_bytes = hex::decode(&proposal.signature)?;
        let signature = Signature::from_bytes(&signature_bytes[..64].try_into()?);
        let is_valid = verifying_key.verify_strict(proposal_data, &signature).is_ok();
        assert!(is_valid);

        // Test BFT vote creation and validation
        let vote_data = b"BFT vote data";
        let vote_signature = signing_key.sign(vote_data);
        let vote_signature_bytes = vote_signature.to_bytes().to_vec();

        let vote = BFTVote {
            voter_id: [3u8; 32],
            proposal_hash: [4u8; 32],
            is_approval: true,
            signature: hex::encode(vote_signature_bytes),
            timestamp: Instant::now(),
        };

        // Test vote signature validation
        let vote_signature_bytes = hex::decode(&vote.signature)?;
        let vote_signature = Signature::from_bytes(&vote_signature_bytes[..64].try_into()?);
        let is_vote_valid = verifying_key.verify_strict(vote_data, &vote_signature).is_ok();
        assert!(is_vote_valid);

        // Test Byzantine behavior detection
        let malicious_votes = vec![
            BFTVote {
                voter_id: [5u8; 32],
                proposal_hash: [6u8; 32],
                is_approval: true,
                signature: "invalid_signature".to_string(),
                timestamp: Instant::now(),
            },
            BFTVote {
                voter_id: [5u8; 32], // Same voter, different vote
                proposal_hash: [7u8; 32],
                is_approval: false,
                signature: "another_invalid_signature".to_string(),
                timestamp: Instant::now(),
            },
        ];

        // Test double-signing detection
        let malicious_validators = crate::consensus::count_malicious_validators(&malicious_votes);
        assert!(malicious_validators.contains(&[5u8; 32]));

        Ok(())
    }

    /// Test key rotation integration
    async fn test_key_rotation(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Testing key rotation...");

        let key_manager = self.key_manager.clone();
        let audit_log = self.audit_log.clone();

        // Create key management service
        let key_service = KeyManagementService::new(
            key_manager.clone(),
            audit_log.clone(),
        );

        // Generate initial key
        let initial_keypair = key_manager.write().await.generate_key_pair().await?;
        let key_id = initial_keypair.public_key.clone();
        key_manager.write().await.add_key_pair(initial_keypair)?;

        // Test key rotation
        let rotation_result = key_service.rotate_key(&key_id).await?;
        assert!(rotation_result.success);

        // Verify new key was created
        let stats = key_manager.read().await.get_key_stats();
        assert!(stats.total_keys > 1);

        // Test audit log
        let audit_entries = audit_log.read().await.get_entries();
        assert!(!audit_entries.is_empty());

        // Test key rotation scheduler
        let scheduler = KeyRotationScheduler::new(
            key_service,
            self.config.key_rotation_interval,
        );

        // Test scheduler initialization
        scheduler.start().await?;
        assert!(scheduler.is_running());

        // Stop scheduler
        scheduler.stop().await?;
        assert!(!scheduler.is_running());

        Ok(())
    }

    /// Test audit logging integration
    async fn test_audit_logging(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Testing audit logging...");

        let audit_log = self.audit_log.clone();

        // Test audit entry creation
        let entry = crate::security::key_management::AuditEntry {
            timestamp: Instant::now(),
            event_type: "key_generation".to_string(),
            key_id: [1u8; 32],
            user_id: "test_user".to_string(),
            details: "Test key generation".to_string(),
            success: true,
        };

        audit_log.write().await.add_entry(entry.clone());

        // Test audit entry retrieval
        let entries = audit_log.read().await.get_entries();
        assert!(!entries.is_empty());
        assert_eq!(entries[0].event_type, "key_generation");

        // Test audit log filtering
        let filtered_entries = audit_log.read().await.get_entries_by_type("key_generation");
        assert!(!filtered_entries.is_empty());

        // Test audit log export
        let exported_log = audit_log.read().await.export_log();
        assert!(!exported_log.is_empty());

        Ok(())
    }

    /// Test threat detection integration
    async fn test_threat_detection(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Testing threat detection...");

        let security_manager = self.security_manager.clone();

        // Test threat detection system
        let threat_detector = security_manager.read().await.get_threat_detector();

        // Test suspicious activity detection
        let suspicious_activity = threat_detector.detect_suspicious_activity("test_peer").await?;
        assert!(!suspicious_activity.is_threat);

        // Test intrusion detection
        let intrusion_result = threat_detector.detect_intrusion("test_peer").await?;
        assert!(!intrusion_result.is_intrusion);

        // Test anomaly detection
        let anomaly_result = threat_detector.detect_anomaly("test_peer").await?;
        assert!(!anomaly_result.is_anomaly);

        // Test threat response
        let response = threat_detector.respond_to_threat("test_peer", "block").await?;
        assert!(response.success);

        Ok(())
    }

    /// Test DDoS protection integration
    async fn test_ddos_protection(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Testing DDoS protection...");

        let security_manager = self.security_manager.clone();

        // Test DDoS protection system
        let ddos_protection = security_manager.read().await.get_ddos_protection();

        // Test rate limiting
        let rate_limit_result = ddos_protection.check_rate_limit("test_peer").await?;
        assert!(rate_limit_result.allowed);

        // Test connection limiting
        let connection_result = ddos_protection.check_connection_limit("test_peer").await?;
        assert!(connection_result.allowed);

        // Test bandwidth limiting
        let bandwidth_result = ddos_protection.check_bandwidth_limit("test_peer", 1024).await?;
        assert!(bandwidth_result.allowed);

        // Test DDoS mitigation
        let mitigation_result = ddos_protection.mitigate_ddos("test_peer").await?;
        assert!(mitigation_result.success);

        Ok(())
    }

    /// Test end-to-end security integration
    async fn test_end_to_end_security(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Testing end-to-end security...");

        let key_manager = self.key_manager.clone();
        let security_manager = self.security_manager.clone();
        let audit_log = self.audit_log.clone();

        // Test secure communication flow
        let alice_keypair = key_manager.write().await.generate_key_pair().await?;
        let bob_keypair = key_manager.write().await.generate_key_pair().await?;

        // Test secure handshake
        let handshake = HandshakeMessage {
            peer_id: [1u8; 32],
            public_key: alice_keypair.public_key.clone(),
            timestamp: Instant::now(),
            signature: vec![], // Will be filled by security manager
        };

        let handshake_result = security_manager.read().await.process_handshake(handshake).await?;
        assert!(handshake_result.success);

        // Test secure message exchange
        let message = b"Secure end-to-end message";
        let encrypted_message = security_manager.read().await.encrypt_message(message).await?;
        let decrypted_message = security_manager.read().await.decrypt_message(&encrypted_message).await?;
        assert_eq!(message, decrypted_message.as_slice());

        // Test audit trail
        let audit_entries = audit_log.read().await.get_entries();
        assert!(!audit_entries.is_empty());

        // Test security metrics
        let security_metrics = security_manager.read().await.get_metrics();
        assert!(security_metrics.connections_established > 0);

        Ok(())
    }
}

/// Run security integration tests
pub async fn run_security_integration_tests() -> Result<(), Box<dyn std::error::Error>> {
    log::info!("🔒 Starting IPPAN security integration tests...");

    let config = SecurityIntegrationConfig::default();
    let test_suite = SecurityIntegrationTestSuite::new(config);
    
    test_suite.run_all_tests().await?;

    log::info!("🎉 All security integration tests completed successfully!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_security_integration_suite() {
        let config = SecurityIntegrationConfig::default();
        let test_suite = SecurityIntegrationTestSuite::new(config);
        
        test_suite.run_all_tests().await.unwrap();
    }

    #[tokio::test]
    async fn test_key_management() {
        let config = SecurityIntegrationConfig::default();
        let test_suite = SecurityIntegrationTestSuite::new(config);
        
        test_suite.test_key_management().await.unwrap();
    }

    #[tokio::test]
    async fn test_network_security() {
        let config = SecurityIntegrationConfig::default();
        let test_suite = SecurityIntegrationTestSuite::new(config);
        
        test_suite.test_network_security().await.unwrap();
    }

    #[tokio::test]
    async fn test_bft_security() {
        let config = SecurityIntegrationConfig::default();
        let test_suite = SecurityIntegrationTestSuite::new(config);
        
        test_suite.test_bft_security().await.unwrap();
    }

    #[tokio::test]
    async fn test_key_rotation() {
        let config = SecurityIntegrationConfig::default();
        let test_suite = SecurityIntegrationTestSuite::new(config);
        
        test_suite.test_key_rotation().await.unwrap();
    }

    #[tokio::test]
    async fn test_audit_logging() {
        let config = SecurityIntegrationConfig::default();
        let test_suite = SecurityIntegrationTestSuite::new(config);
        
        test_suite.test_audit_logging().await.unwrap();
    }

    #[tokio::test]
    async fn test_threat_detection() {
        let config = SecurityIntegrationConfig::default();
        let test_suite = SecurityIntegrationTestSuite::new(config);
        
        test_suite.test_threat_detection().await.unwrap();
    }

    #[tokio::test]
    async fn test_ddos_protection() {
        let config = SecurityIntegrationConfig::default();
        let test_suite = SecurityIntegrationTestSuite::new(config);
        
        test_suite.test_ddos_protection().await.unwrap();
    }

    #[tokio::test]
    async fn test_end_to_end_security() {
        let config = SecurityIntegrationConfig::default();
        let test_suite = SecurityIntegrationTestSuite::new(config);
        
        test_suite.test_end_to_end_security().await.unwrap();
    }
}
