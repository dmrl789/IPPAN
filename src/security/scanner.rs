use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use anyhow::Result;

/// Security threat types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SecurityThreat {
    BruteForceAttack,
    DDoS,
    MaliciousTransaction,
    UnauthorizedAccess,
    DataExfiltration,
    NetworkIntrusion,
    ResourceExhaustion,
    ConsensusManipulation,
    StorageTampering,
    WalletCompromise,
}

/// Security event severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SecuritySeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Security event details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub id: String,
    pub threat_type: SecurityThreat,
    pub severity: SecuritySeverity,
    pub description: String,
    pub source_ip: Option<String>,
    pub source_node: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub evidence: HashMap<String, String>,
    pub action_taken: Option<String>,
    pub resolved: bool,
}

/// Security metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityMetrics {
    pub total_events: u64,
    pub critical_events: u64,
    pub high_events: u64,
    pub medium_events: u64,
    pub low_events: u64,
    pub blocked_attacks: u64,
    pub active_threats: u64,
    pub last_event_time: Option<chrono::DateTime<chrono::Utc>>,
}

/// Runtime security scanner
pub struct SecurityScanner {
    events: Arc<Mutex<Vec<SecurityEvent>>>,
    metrics: Arc<Mutex<SecurityMetrics>>,
    threat_patterns: HashMap<SecurityThreat, Vec<String>>,
    rate_limiters: Arc<Mutex<HashMap<String, RateLimiter>>>,
    blacklist: Arc<Mutex<HashMap<String, Instant>>>,
    config: ScannerConfig,
    event_sender: mpsc::Sender<SecurityEvent>,
}

#[derive(Debug, Clone)]
pub struct ScannerConfig {
    pub enable_real_time_monitoring: bool,
    pub max_events_history: usize,
    pub blacklist_duration: Duration,
    pub rate_limit_window: Duration,
    pub max_requests_per_window: u32,
    pub alert_threshold: SecuritySeverity,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            enable_real_time_monitoring: true,
            max_events_history: 10000,
            blacklist_duration: Duration::from_secs(3600), // 1 hour
            rate_limit_window: Duration::from_secs(60), // 1 minute
            max_requests_per_window: 100,
            alert_threshold: SecuritySeverity::Medium,
        }
    }
}

/// Rate limiter for security monitoring
struct RateLimiter {
    requests: Vec<Instant>,
    window_duration: Duration,
    max_requests: u32,
}

impl RateLimiter {
    fn new(window_duration: Duration, max_requests: u32) -> Self {
        Self {
            requests: Vec::new(),
            window_duration,
            max_requests,
        }
    }

    fn check_rate_limit(&mut self) -> bool {
        let now = Instant::now();
        
        // Remove expired requests
        self.requests.retain(|&time| now.duration_since(time) < self.window_duration);
        
        if self.requests.len() >= self.max_requests as usize {
            false
        } else {
            self.requests.push(now);
            true
        }
    }
}

impl SecurityScanner {
    /// Create a new security scanner
    pub fn new(config: ScannerConfig) -> (Self, mpsc::Receiver<SecurityEvent>) {
        let (event_sender, event_receiver) = mpsc::channel(1000);
        
        let scanner = Self {
            events: Arc::new(Mutex::new(Vec::new())),
            metrics: Arc::new(Mutex::new(SecurityMetrics {
                total_events: 0,
                critical_events: 0,
                high_events: 0,
                medium_events: 0,
                low_events: 0,
                blocked_attacks: 0,
                active_threats: 0,
                last_event_time: None,
            })),
            threat_patterns: Self::initialize_threat_patterns(),
            rate_limiters: Arc::new(Mutex::new(HashMap::new())),
            blacklist: Arc::new(Mutex::new(HashMap::new())),
            config,
            event_sender,
        };
        
        (scanner, event_receiver)
    }

    /// Initialize threat detection patterns
    fn initialize_threat_patterns() -> HashMap<SecurityThreat, Vec<String>> {
        let mut patterns = HashMap::new();
        
        // Brute force attack patterns
        patterns.insert(SecurityThreat::BruteForceAttack, vec![
            "multiple_failed_logins".to_string(),
            "rapid_password_attempts".to_string(),
            "credential_stuffing".to_string(),
        ]);
        
        // DDoS patterns
        patterns.insert(SecurityThreat::DDoS, vec![
            "high_request_rate".to_string(),
            "resource_exhaustion".to_string(),
            "network_flooding".to_string(),
        ]);
        
        // Malicious transaction patterns
        patterns.insert(SecurityThreat::MaliciousTransaction, vec![
            "double_spending".to_string(),
            "invalid_signature".to_string(),
            "suspicious_amount".to_string(),
        ]);
        
        // Unauthorized access patterns
        patterns.insert(SecurityThreat::UnauthorizedAccess, vec![
            "invalid_permissions".to_string(),
            "privilege_escalation".to_string(),
            "unauthorized_api_call".to_string(),
        ]);
        
        // Data exfiltration patterns
        patterns.insert(SecurityThreat::DataExfiltration, vec![
            "large_data_transfer".to_string(),
            "suspicious_export".to_string(),
            "unauthorized_access".to_string(),
        ]);
        
        patterns
    }

    /// Start the security scanner
    pub async fn start(&self) -> Result<()> {
        if self.config.enable_real_time_monitoring {
            self.start_monitoring_loop().await?;
        }
        Ok(())
    }

    /// Start the monitoring loop
    async fn start_monitoring_loop(&self) -> Result<()> {
        let scanner = Arc::new(self.clone());
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            
            loop {
                interval.tick().await;
                
                // Clean up expired blacklist entries
                scanner.cleanup_blacklist().await;
                
                // Update active threats count
                scanner.update_active_threats().await;
            }
        });
        
        Ok(())
    }

    /// Clean up expired blacklist entries
    async fn cleanup_blacklist(&self) {
        let mut blacklist = self.blacklist.lock().unwrap();
        let now = Instant::now();
        
        blacklist.retain(|_, &mut timestamp| {
            now.duration_since(timestamp) < self.config.blacklist_duration
        });
    }

    /// Update active threats count
    async fn update_active_threats(&self) {
        let events = self.events.lock().unwrap();
        let recent_events: Vec<_> = events.iter()
            .filter(|event| {
                let event_time = event.timestamp;
                let now = chrono::Utc::now();
                (now - event_time).num_minutes() < 60 // Last hour
            })
            .collect();
        
        let mut metrics = self.metrics.lock().unwrap();
        metrics.active_threats = recent_events.len() as u64;
    }

    /// Check for brute force attacks
    pub async fn check_brute_force_attack(&self, source_ip: &str, success: bool) -> Result<bool> {
        let mut rate_limiters = self.rate_limiters.lock().unwrap();
        let limiter = rate_limiters.entry(source_ip.to_string()).or_insert_with(|| {
            RateLimiter::new(self.config.rate_limit_window, self.config.max_requests_per_window)
        });
        
        if !success && !limiter.check_rate_limit() {
            // Potential brute force attack detected
            let event = SecurityEvent {
                id: uuid::Uuid::new_v4().to_string(),
                threat_type: SecurityThreat::BruteForceAttack,
                severity: SecuritySeverity::High,
                description: format!("Potential brute force attack from {}", source_ip),
                source_ip: Some(source_ip.to_string()),
                source_node: None,
                timestamp: chrono::Utc::now(),
                evidence: HashMap::new(),
                action_taken: Some("Rate limited".to_string()),
                resolved: false,
            };
            
            self.record_event(event).await?;
            self.blacklist_source(source_ip).await;
            return Ok(true);
        }
        
        Ok(false)
    }

    /// Check for DDoS attacks
    pub async fn check_ddos_attack(&self, source_ip: &str, request_count: u32) -> Result<bool> {
        let threshold = self.config.max_requests_per_window * 10; // 10x normal rate
        
        if request_count > threshold {
            let event = SecurityEvent {
                id: uuid::Uuid::new_v4().to_string(),
                threat_type: SecurityThreat::DDoS,
                severity: SecuritySeverity::Critical,
                description: format!("DDoS attack detected from {} ({} requests)", source_ip, request_count),
                source_ip: Some(source_ip.to_string()),
                source_node: None,
                timestamp: chrono::Utc::now(),
                evidence: {
                    let mut evidence = HashMap::new();
                    evidence.insert("request_count".to_string(), request_count.to_string());
                    evidence.insert("threshold".to_string(), threshold.to_string());
                    evidence
                },
                action_taken: Some("Blocked IP".to_string()),
                resolved: false,
            };
            
            self.record_event(event).await?;
            self.blacklist_source(source_ip).await;
            return Ok(true);
        }
        
        Ok(false)
    }

    /// Check for malicious transactions
    pub async fn check_malicious_transaction(&self, transaction_data: &str) -> Result<bool> {
        let suspicious_patterns = [
            "double_spend",
            "invalid_signature",
            "suspicious_amount",
            "malicious_script",
        ];
        
        for pattern in &suspicious_patterns {
            if transaction_data.contains(pattern) {
                let event = SecurityEvent {
                    id: uuid::Uuid::new_v4().to_string(),
                    threat_type: SecurityThreat::MaliciousTransaction,
                    severity: SecuritySeverity::High,
                    description: format!("Malicious transaction detected: {}", pattern),
                    source_ip: None,
                    source_node: None,
                    timestamp: chrono::Utc::now(),
                    evidence: {
                        let mut evidence = HashMap::new();
                        evidence.insert("pattern".to_string(), pattern.to_string());
                        evidence.insert("transaction_data".to_string(), transaction_data.to_string());
                        evidence
                    },
                    action_taken: Some("Transaction rejected".to_string()),
                    resolved: false,
                };
                
                self.record_event(event).await?;
                return Ok(true);
            }
        }
        
        Ok(false)
    }

    /// Check for unauthorized access
    pub async fn check_unauthorized_access(&self, user_id: &str, resource: &str, action: &str) -> Result<bool> {
        // Check if user has permission for this action
        let has_permission = self.check_permissions(user_id, resource, action).await;
        
        if !has_permission {
            let event = SecurityEvent {
                id: uuid::Uuid::new_v4().to_string(),
                threat_type: SecurityThreat::UnauthorizedAccess,
                severity: SecuritySeverity::Medium,
                description: format!("Unauthorized access attempt: {} {} {}", user_id, action, resource),
                source_ip: None,
                source_node: Some(user_id.to_string()),
                timestamp: chrono::Utc::now(),
                evidence: {
                    let mut evidence = HashMap::new();
                    evidence.insert("user_id".to_string(), user_id.to_string());
                    evidence.insert("resource".to_string(), resource.to_string());
                    evidence.insert("action".to_string(), action.to_string());
                    evidence
                },
                action_taken: Some("Access denied".to_string()),
                resolved: false,
            };
            
            self.record_event(event).await?;
            return Ok(true);
        }
        
        Ok(false)
    }

    /// Check permissions (simplified implementation)
    async fn check_permissions(&self, user_id: &str, resource: &str, action: &str) -> bool {
        // This is a simplified permission check
        // In a real implementation, you would check against a permission database
        match (user_id, resource, action) {
            ("admin", _, _) => true,
            ("user", "public", _) => true,
            ("user", "private", "read") => false,
            _ => false,
        }
    }

    /// Check for data exfiltration
    pub async fn check_data_exfiltration(&self, data_size: u64, destination: &str) -> Result<bool> {
        let max_allowed_size = 1024 * 1024 * 100; // 100MB
        
        if data_size > max_allowed_size {
            let event = SecurityEvent {
                id: uuid::Uuid::new_v4().to_string(),
                threat_type: SecurityThreat::DataExfiltration,
                severity: SecuritySeverity::High,
                description: format!("Large data transfer detected: {} bytes to {}", data_size, destination),
                source_ip: None,
                source_node: None,
                timestamp: chrono::Utc::now(),
                evidence: {
                    let mut evidence = HashMap::new();
                    evidence.insert("data_size".to_string(), data_size.to_string());
                    evidence.insert("destination".to_string(), destination.to_string());
                    evidence.insert("max_allowed".to_string(), max_allowed_size.to_string());
                    evidence
                },
                action_taken: Some("Transfer blocked".to_string()),
                resolved: false,
            };
            
            self.record_event(event).await?;
            return Ok(true);
        }
        
        Ok(false)
    }

    /// Record a security event
    pub async fn record_event(&self, event: SecurityEvent) -> Result<()> {
        // Add to events list
        {
            let mut events = self.events.lock().unwrap();
            events.push(event.clone());
            
            // Trim old events if we exceed the limit
            let current_len = events.len();
            if current_len > self.config.max_events_history {
                let to_remove = current_len - self.config.max_events_history;
                events.drain(0..to_remove);
            }
        }
        
        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.total_events += 1;
            metrics.last_event_time = Some(event.timestamp);
            
            match event.severity {
                SecuritySeverity::Critical => metrics.critical_events += 1,
                SecuritySeverity::High => metrics.high_events += 1,
                SecuritySeverity::Medium => metrics.medium_events += 1,
                SecuritySeverity::Low => metrics.low_events += 1,
                SecuritySeverity::Info => {}
            }
            
            if event.action_taken.is_some() {
                metrics.blocked_attacks += 1;
            }
        }
        
        // Send event to monitoring system
        if let Err(_) = self.event_sender.send(event).await {
            tracing::warn!("Failed to send security event to monitoring system");
        }
        
        Ok(())
    }

    /// Blacklist a source
    async fn blacklist_source(&self, source: &str) {
        let mut blacklist = self.blacklist.lock().unwrap();
        blacklist.insert(source.to_string(), Instant::now());
    }

    /// Check if a source is blacklisted
    pub fn is_blacklisted(&self, source: &str) -> bool {
        let blacklist = self.blacklist.lock().unwrap();
        blacklist.contains_key(source)
    }

    /// Get security metrics
    pub fn get_metrics(&self) -> SecurityMetrics {
        self.metrics.lock().unwrap().clone()
    }

    /// Get recent security events
    pub fn get_recent_events(&self, hours: u32) -> Vec<SecurityEvent> {
        let events = self.events.lock().unwrap();
        let cutoff_time = chrono::Utc::now() - chrono::Duration::hours(hours as i64);
        
        events.iter()
            .filter(|event| event.timestamp >= cutoff_time)
            .cloned()
            .collect()
    }

    /// Resolve a security event
    pub fn resolve_event(&self, event_id: &str) -> Result<()> {
        let mut events = self.events.lock().unwrap();
        
        if let Some(event) = events.iter_mut().find(|e| e.id == event_id) {
            event.resolved = true;
        }
        
        Ok(())
    }

    /// Export security events to JSON
    pub fn export_events_json(&self) -> Result<String> {
        let events = self.events.lock().unwrap();
        Ok(serde_json::to_string_pretty(&*events)?)
    }

    /// Generate security report
    pub fn generate_security_report(&self) -> SecurityReport {
        let metrics = self.get_metrics();
        let recent_events = self.get_recent_events(24); // Last 24 hours
        
        let mut threat_counts = HashMap::new();
        for event in &recent_events {
            *threat_counts.entry(event.threat_type.clone()).or_insert(0) += 1;
        }
        
        SecurityReport {
            metrics,
            recent_events,
            threat_counts,
            generated_at: chrono::Utc::now(),
        }
    }
}

/// Security report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityReport {
    pub metrics: SecurityMetrics,
    pub recent_events: Vec<SecurityEvent>,
    pub threat_counts: HashMap<SecurityThreat, u32>,
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

impl Clone for SecurityScanner {
    fn clone(&self) -> Self {
        Self {
            events: Arc::clone(&self.events),
            metrics: Arc::clone(&self.metrics),
            threat_patterns: self.threat_patterns.clone(),
            rate_limiters: Arc::clone(&self.rate_limiters),
            blacklist: Arc::clone(&self.blacklist),
            config: self.config.clone(),
            event_sender: self.event_sender.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_security_scanner_creation() {
        let config = ScannerConfig::default();
        let (scanner, _receiver) = SecurityScanner::new(config);
        
        let metrics = scanner.get_metrics();
        assert_eq!(metrics.total_events, 0);
    }

    #[tokio::test]
    async fn test_brute_force_detection() {
        let config = ScannerConfig::default();
        let (scanner, _receiver) = SecurityScanner::new(config);
        
        // Simulate multiple failed login attempts
        for _ in 0..150 {
            let is_attack = scanner.check_brute_force_attack("192.168.1.1", false).await.unwrap();
            if is_attack {
                break;
            }
        }
        
        let metrics = scanner.get_metrics();
        assert!(metrics.total_events > 0);
    }

    #[tokio::test]
    async fn test_ddos_detection() {
        let config = ScannerConfig::default();
        let (scanner, _receiver) = SecurityScanner::new(config);
        
        let is_attack = scanner.check_ddos_attack("192.168.1.1", 1500).await.unwrap();
        assert!(is_attack);
        
        let metrics = scanner.get_metrics();
        assert!(metrics.blocked_attacks > 0);
    }

    #[tokio::test]
    async fn test_malicious_transaction_detection() {
        let config = ScannerConfig::default();
        let (scanner, _receiver) = SecurityScanner::new(config);
        
        let is_malicious = scanner.check_malicious_transaction("double_spend_attempt").await.unwrap();
        assert!(is_malicious);
    }

    #[tokio::test]
    async fn test_unauthorized_access_detection() {
        let config = ScannerConfig::default();
        let (scanner, _receiver) = SecurityScanner::new(config);
        
        let is_unauthorized = scanner.check_unauthorized_access("user", "private", "write").await.unwrap();
        assert!(is_unauthorized);
    }

    #[tokio::test]
    async fn test_blacklist_functionality() {
        let config = ScannerConfig::default();
        let (scanner, _receiver) = SecurityScanner::new(config);
        
        // Trigger blacklist
        scanner.check_ddos_attack("192.168.1.1", 1500).await.unwrap();
        
        assert!(scanner.is_blacklisted("192.168.1.1"));
    }
} 