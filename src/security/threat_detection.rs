//! Advanced threat detection system for IPPAN
//! 
//! Provides real-time security monitoring, threat analysis, and automated response

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use std::time::Duration;

/// Threat severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ThreatSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Threat types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ThreatType {
    DDoS,
    BruteForce,
    UnauthorizedAccess,
    MaliciousTransaction,
    DataBreach,
    NetworkIntrusion,
    Malware,
    InsiderThreat,
    ConfigurationBreach,
    ConsensusAttack,
    StorageAttack,
    CrossChainAttack,
}

/// Threat source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatSource {
    pub ip_address: String,
    pub user_agent: Option<String>,
    pub node_id: Option<String>,
    pub location: Option<String>,
    pub reputation_score: f64,
    pub previous_incidents: u32,
}

/// Threat detection rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub threat_type: ThreatType,
    pub severity: ThreatSeverity,
    pub conditions: Vec<ThreatCondition>,
    pub actions: Vec<ThreatAction>,
    pub enabled: bool,
    pub cooldown_seconds: u64,
    pub threshold: u32,
    pub time_window_seconds: u64,
}

/// Threat condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatCondition {
    pub field: String,
    pub operator: ConditionOperator,
    pub value: serde_json::Value,
    pub time_window_seconds: Option<u64>,
}

/// Condition operators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    Contains,
    NotContains,
    Regex,
    In,
    NotIn,
}

/// Threat action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatAction {
    pub action_type: ActionType,
    pub parameters: HashMap<String, serde_json::Value>,
    pub delay_seconds: Option<u64>,
}

/// Action types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    BlockIP,
    RateLimit,
    Alert,
    Log,
    Quarantine,
    Shutdown,
    Notify,
    Blacklist,
    Whitelist,
    Custom,
}

/// Detected threat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedThreat {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub threat_type: ThreatType,
    pub severity: ThreatSeverity,
    pub source: ThreatSource,
    pub rule_id: String,
    pub description: String,
    pub evidence: HashMap<String, serde_json::Value>,
    pub status: ThreatStatus,
    pub response_actions: Vec<ThreatResponse>,
    pub false_positive: bool,
    pub resolved: bool,
    pub resolution_time: Option<DateTime<Utc>>,
}

/// Threat status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThreatStatus {
    Active,
    Investigating,
    Responding,
    Resolved,
    FalsePositive,
    Escalated,
}

/// Threat response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatResponse {
    pub action_type: ActionType,
    pub timestamp: DateTime<Utc>,
    pub success: bool,
    pub error_message: Option<String>,
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Threat statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatStats {
    pub total_threats: u64,
    pub threats_by_severity: HashMap<ThreatSeverity, u64>,
    pub threats_by_type: HashMap<ThreatType, u64>,
    pub active_threats: u64,
    pub resolved_threats: u64,
    pub false_positives: u64,
    pub average_response_time_ms: u64,
    pub blocked_ips: u64,
    pub rate_limited_requests: u64,
}

/// Threat detection engine
pub struct ThreatDetectionEngine {
    rules: Arc<RwLock<HashMap<String, ThreatRule>>>,
    detected_threats: Arc<RwLock<HashMap<String, DetectedThreat>>>,
    blacklisted_ips: Arc<RwLock<HashSet<String>>>,
    rate_limits: Arc<RwLock<HashMap<String, RateLimitInfo>>>,
    stats: Arc<RwLock<ThreatStats>>,
    enabled: bool,
}

/// Rate limit information
#[derive(Debug, Clone)]
struct RateLimitInfo {
    requests: u32,
    window_start: DateTime<Utc>,
    blocked_until: Option<DateTime<Utc>>,
}

impl ThreatDetectionEngine {
    /// Create a new threat detection engine
    pub fn new() -> Self {
        Self {
            rules: Arc::new(RwLock::new(HashMap::new())),
            detected_threats: Arc::new(RwLock::new(HashMap::new())),
            blacklisted_ips: Arc::new(RwLock::new(HashSet::new())),
            rate_limits: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ThreatStats {
                total_threats: 0,
                threats_by_severity: HashMap::new(),
                threats_by_type: HashMap::new(),
                active_threats: 0,
                resolved_threats: 0,
                false_positives: 0,
                average_response_time_ms: 0,
                blocked_ips: 0,
                rate_limited_requests: 0,
            })),
            enabled: true,
        }
    }

    /// Add a threat detection rule
    pub async fn add_rule(&self, rule: ThreatRule) {
        let mut rules = self.rules.write().await;
        rules.insert(rule.id.clone(), rule);
    }

    /// Remove a threat detection rule
    pub async fn remove_rule(&self, rule_id: &str) {
        let mut rules = self.rules.write().await;
        rules.remove(rule_id);
    }

    /// Get all threat detection rules
    pub async fn get_rules(&self) -> Vec<ThreatRule> {
        let rules = self.rules.read().await;
        rules.values().cloned().collect()
    }

    /// Analyze an event for threats
    pub async fn analyze_event(&self, event: SecurityEvent) -> Vec<DetectedThreat> {
        if !self.enabled {
            return Vec::new();
        }

        let mut detected_threats = Vec::new();
        let rules = self.rules.read().await;

        for rule in rules.values() {
            if !rule.enabled {
                continue;
            }

            if self.evaluate_rule(rule, &event).await {
                let threat = self.create_threat(rule, &event).await;
                detected_threats.push(threat.clone());
                
                // Execute threat actions
                self.execute_threat_actions(&threat).await;
                
                // Store detected threat
                let mut threats = self.detected_threats.write().await;
                threats.insert(threat.id.clone(), threat);
                
                // Update statistics
                self.update_stats(&threat).await;
            }
        }

        detected_threats
    }

    /// Evaluate a rule against an event
    async fn evaluate_rule(&self, rule: &ThreatRule, event: &SecurityEvent) -> bool {
        let mut condition_matches = 0;
        
        for condition in &rule.conditions {
            if self.evaluate_condition(condition, event).await {
                condition_matches += 1;
            }
        }

        // Check if enough conditions match and threshold is met
        condition_matches >= rule.threshold
    }

    /// Evaluate a condition against an event
    async fn evaluate_condition(&self, condition: &ThreatCondition, event: &SecurityEvent) -> bool {
        let event_value = self.get_event_field_value(&condition.field, event);
        
        match condition.operator {
            ConditionOperator::Equals => event_value == condition.value,
            ConditionOperator::NotEquals => event_value != condition.value,
            ConditionOperator::GreaterThan => {
                if let (Some(a), Some(b)) = (event_value.as_f64(), condition.value.as_f64()) {
                    a > b
                } else {
                    false
                }
            }
            ConditionOperator::LessThan => {
                if let (Some(a), Some(b)) = (event_value.as_f64(), condition.value.as_f64()) {
                    a < b
                } else {
                    false
                }
            }
            ConditionOperator::Contains => {
                if let (Some(a), Some(b)) = (event_value.as_str(), condition.value.as_str()) {
                    a.contains(b)
                } else {
                    false
                }
            }
            ConditionOperator::NotContains => {
                if let (Some(a), Some(b)) = (event_value.as_str(), condition.value.as_str()) {
                    !a.contains(b)
                } else {
                    false
                }
            }
            ConditionOperator::Regex => {
                if let (Some(a), Some(b)) = (event_value.as_str(), condition.value.as_str()) {
                    regex::Regex::new(b).map(|re| re.is_match(a)).unwrap_or(false)
                } else {
                    false
                }
            }
            ConditionOperator::In => {
                if let Some(array) = condition.value.as_array() {
                    array.contains(&event_value)
                } else {
                    false
                }
            }
            ConditionOperator::NotIn => {
                if let Some(array) = condition.value.as_array() {
                    !array.contains(&event_value)
                } else {
                    false
                }
            }
        }
    }

    /// Get event field value
    fn get_event_field_value(&self, field: &str, event: &SecurityEvent) -> serde_json::Value {
        match field {
            "ip_address" => serde_json::Value::String(event.ip_address.clone()),
            "user_agent" => event.user_agent.clone().map(serde_json::Value::String).unwrap_or_default(),
            "request_count" => serde_json::Value::Number(serde_json::Number::from(event.request_count)),
            "error_count" => serde_json::Value::Number(serde_json::Number::from(event.error_count)),
            "response_time_ms" => serde_json::Value::Number(serde_json::Number::from(event.response_time_ms)),
            "endpoint" => serde_json::Value::String(event.endpoint.clone()),
            "method" => serde_json::Value::String(event.method.clone()),
            "status_code" => serde_json::Value::Number(serde_json::Number::from(event.status_code)),
            "payload_size" => serde_json::Value::Number(serde_json::Number::from(event.payload_size)),
            _ => serde_json::Value::Null,
        }
    }

    /// Create a threat from a rule and event
    async fn create_threat(&self, rule: &ThreatRule, event: &SecurityEvent) -> DetectedThreat {
        let threat_id = format!("threat_{}", Utc::now().timestamp_millis());
        
        let source = ThreatSource {
            ip_address: event.ip_address.clone(),
            user_agent: event.user_agent.clone(),
            node_id: None,
            location: None,
            reputation_score: 0.0,
            previous_incidents: 0,
        };

        let evidence = HashMap::from([
            ("ip_address".to_string(), serde_json::Value::String(event.ip_address.clone())),
            ("endpoint".to_string(), serde_json::Value::String(event.endpoint.clone())),
            ("method".to_string(), serde_json::Value::String(event.method.clone())),
            ("request_count".to_string(), serde_json::Value::Number(serde_json::Number::from(event.request_count))),
            ("error_count".to_string(), serde_json::Value::Number(serde_json::Number::from(event.error_count))),
        ]);

        DetectedThreat {
            id: threat_id,
            timestamp: Utc::now(),
            threat_type: rule.threat_type.clone(),
            severity: rule.severity.clone(),
            source,
            rule_id: rule.id.clone(),
            description: rule.description.clone(),
            evidence,
            status: ThreatStatus::Active,
            response_actions: Vec::new(),
            false_positive: false,
            resolved: false,
            resolution_time: None,
        }
    }

    /// Execute threat actions
    async fn execute_threat_actions(&self, threat: &DetectedThreat) {
        let rules = self.rules.read().await;
        if let Some(rule) = rules.get(&threat.rule_id) {
            for action in &rule.actions {
                self.execute_action(action, threat).await;
            }
        }
    }

    /// Execute a threat action
    async fn execute_action(&self, action: &ThreatAction, threat: &DetectedThreat) {
        match action.action_type {
            ActionType::BlockIP => {
                let mut blacklist = self.blacklisted_ips.write().await;
                blacklist.insert(threat.source.ip_address.clone());
                
                let mut stats = self.stats.write().await;
                stats.blocked_ips += 1;
            }
            ActionType::RateLimit => {
                let mut rate_limits = self.rate_limits.write().await;
                rate_limits.insert(threat.source.ip_address.clone(), RateLimitInfo {
                    requests: 0,
                    window_start: Utc::now(),
                    blocked_until: Some(Utc::now() + chrono::Duration::seconds(300)),
                });
                
                let mut stats = self.stats.write().await;
                stats.rate_limited_requests += 1;
            }
            ActionType::Alert => {
                // TODO: Send alert notification
                println!("ALERT: Threat detected - {}", threat.description);
            }
            ActionType::Log => {
                // TODO: Log threat to security log
                println!("SECURITY LOG: Threat logged - {}", threat.description);
            }
            ActionType::Quarantine => {
                // TODO: Quarantine affected resources
                println!("QUARANTINE: Resources quarantined for threat - {}", threat.description);
            }
            ActionType::Shutdown => {
                // TODO: Graceful shutdown of affected services
                println!("SHUTDOWN: Services shutdown due to threat - {}", threat.description);
            }
            ActionType::Notify => {
                // TODO: Send notification to security team
                println!("NOTIFY: Security team notified of threat - {}", threat.description);
            }
            ActionType::Blacklist => {
                // TODO: Add to global blacklist
                println!("BLACKLIST: Added to global blacklist - {}", threat.description);
            }
            ActionType::Whitelist => {
                // TODO: Add to whitelist
                println!("WHITELIST: Added to whitelist - {}", threat.description);
            }
            ActionType::Custom => {
                // TODO: Execute custom action
                println!("CUSTOM: Custom action executed - {}", threat.description);
            }
        }
    }

    /// Update threat statistics
    async fn update_stats(&self, threat: &DetectedThreat) {
        let mut stats = self.stats.write().await;
        stats.total_threats += 1;
        
        *stats.threats_by_severity.entry(threat.severity.clone()).or_insert(0) += 1;
        *stats.threats_by_type.entry(threat.threat_type.clone()).or_insert(0) += 1;
        
        if threat.status == ThreatStatus::Active {
            stats.active_threats += 1;
        }
    }

    /// Get threat statistics
    pub async fn get_stats(&self) -> ThreatStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Get detected threats
    pub async fn get_detected_threats(&self) -> Vec<DetectedThreat> {
        let threats = self.detected_threats.read().await;
        threats.values().cloned().collect()
    }

    /// Get active threats
    pub async fn get_active_threats(&self) -> Vec<DetectedThreat> {
        let threats = self.detected_threats.read().await;
        threats.values()
            .filter(|t| t.status == ThreatStatus::Active)
            .cloned()
            .collect()
    }

    /// Resolve a threat
    pub async fn resolve_threat(&self, threat_id: &str) -> Result<(), String> {
        let mut threats = self.detected_threats.write().await;
        if let Some(threat) = threats.get_mut(threat_id) {
            threat.status = ThreatStatus::Resolved;
            threat.resolved = true;
            threat.resolution_time = Some(Utc::now());
            
            let mut stats = self.stats.write().await;
            stats.resolved_threats += 1;
            stats.active_threats = stats.active_threats.saturating_sub(1);
            
            Ok(())
        } else {
            Err("Threat not found".to_string())
        }
    }

    /// Mark threat as false positive
    pub async fn mark_false_positive(&self, threat_id: &str) -> Result<(), String> {
        let mut threats = self.detected_threats.write().await;
        if let Some(threat) = threats.get_mut(threat_id) {
            threat.false_positive = true;
            threat.status = ThreatStatus::FalsePositive;
            
            let mut stats = self.stats.write().await;
            stats.false_positives += 1;
            
            Ok(())
        } else {
            Err("Threat not found".to_string())
        }
    }

    /// Check if IP is blacklisted
    pub async fn is_ip_blacklisted(&self, ip: &str) -> bool {
        let blacklist = self.blacklisted_ips.read().await;
        blacklist.contains(ip)
    }

    /// Check if IP is rate limited
    pub async fn is_ip_rate_limited(&self, ip: &str) -> bool {
        let rate_limits = self.rate_limits.read().await;
        if let Some(rate_limit) = rate_limits.get(ip) {
            if let Some(blocked_until) = rate_limit.blocked_until {
                return Utc::now() < blocked_until;
            }
        }
        false
    }

    /// Enable threat detection
    pub async fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable threat detection
    pub async fn disable(&mut self) {
        self.enabled = false;
    }

    /// Clear all threats
    pub async fn clear_threats(&self) {
        let mut threats = self.detected_threats.write().await;
        threats.clear();
        
        let mut stats = self.stats.write().await;
        stats.active_threats = 0;
    }

    /// Clear blacklist
    pub async fn clear_blacklist(&self) {
        let mut blacklist = self.blacklisted_ips.write().await;
        blacklist.clear();
        
        let mut stats = self.stats.write().await;
        stats.blocked_ips = 0;
    }
}

/// Security event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub timestamp: DateTime<Utc>,
    pub ip_address: String,
    pub user_agent: Option<String>,
    pub endpoint: String,
    pub method: String,
    pub status_code: u16,
    pub request_count: u32,
    pub error_count: u32,
    pub response_time_ms: u64,
    pub payload_size: u64,
    pub headers: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
}

impl Default for ThreatDetectionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_threat_detection_engine_creation() {
        let engine = ThreatDetectionEngine::new();
        assert!(engine.enabled);
    }

    #[tokio::test]
    async fn test_add_remove_rule() {
        let engine = ThreatDetectionEngine::new();
        
        let rule = ThreatRule {
            id: "test_rule".to_string(),
            name: "Test Rule".to_string(),
            description: "Test threat rule".to_string(),
            threat_type: ThreatType::DDoS,
            severity: ThreatSeverity::High,
            conditions: vec![
                ThreatCondition {
                    field: "request_count".to_string(),
                    operator: ConditionOperator::GreaterThan,
                    value: serde_json::Value::Number(serde_json::Number::from(100)),
                    time_window_seconds: Some(60),
                }
            ],
            actions: vec![
                ThreatAction {
                    action_type: ActionType::BlockIP,
                    parameters: HashMap::new(),
                    delay_seconds: None,
                }
            ],
            enabled: true,
            cooldown_seconds: 300,
            threshold: 1,
            time_window_seconds: 60,
        };
        
        engine.add_rule(rule.clone()).await;
        
        let rules = engine.get_rules().await;
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, "test_rule");
        
        engine.remove_rule("test_rule").await;
        
        let rules = engine.get_rules().await;
        assert_eq!(rules.len(), 0);
    }

    #[tokio::test]
    async fn test_threat_analysis() {
        let engine = ThreatDetectionEngine::new();
        
        let rule = ThreatRule {
            id: "ddos_rule".to_string(),
            name: "DDoS Detection".to_string(),
            description: "Detect DDoS attacks".to_string(),
            threat_type: ThreatType::DDoS,
            severity: ThreatSeverity::Critical,
            conditions: vec![
                ThreatCondition {
                    field: "request_count".to_string(),
                    operator: ConditionOperator::GreaterThan,
                    value: serde_json::Value::Number(serde_json::Number::from(50)),
                    time_window_seconds: Some(60),
                }
            ],
            actions: vec![
                ThreatAction {
                    action_type: ActionType::BlockIP,
                    parameters: HashMap::new(),
                    delay_seconds: None,
                }
            ],
            enabled: true,
            cooldown_seconds: 300,
            threshold: 1,
            time_window_seconds: 60,
        };
        
        engine.add_rule(rule).await;
        
        let event = SecurityEvent {
            timestamp: Utc::now(),
            ip_address: "192.168.1.100".to_string(),
            user_agent: Some("Mozilla/5.0".to_string()),
            endpoint: "/api/health".to_string(),
            method: "GET".to_string(),
            status_code: 200,
            request_count: 100,
            error_count: 0,
            response_time_ms: 50,
            payload_size: 1024,
            headers: HashMap::new(),
            query_params: HashMap::new(),
        };
        
        let threats = engine.analyze_event(event).await;
        assert_eq!(threats.len(), 1);
        assert_eq!(threats[0].threat_type, ThreatType::DDoS);
        assert_eq!(threats[0].severity, ThreatSeverity::Critical);
    }

    #[tokio::test]
    async fn test_ip_blacklisting() {
        let engine = ThreatDetectionEngine::new();
        
        let rule = ThreatRule {
            id: "block_rule".to_string(),
            name: "Block IP".to_string(),
            description: "Block malicious IP".to_string(),
            threat_type: ThreatType::UnauthorizedAccess,
            severity: ThreatSeverity::High,
            conditions: vec![
                ThreatCondition {
                    field: "error_count".to_string(),
                    operator: ConditionOperator::GreaterThan,
                    value: serde_json::Value::Number(serde_json::Number::from(10)),
                    time_window_seconds: Some(60),
                }
            ],
            actions: vec![
                ThreatAction {
                    action_type: ActionType::BlockIP,
                    parameters: HashMap::new(),
                    delay_seconds: None,
                }
            ],
            enabled: true,
            cooldown_seconds: 300,
            threshold: 1,
            time_window_seconds: 60,
        };
        
        engine.add_rule(rule).await;
        
        let event = SecurityEvent {
            timestamp: Utc::now(),
            ip_address: "192.168.1.200".to_string(),
            user_agent: None,
            endpoint: "/api/login".to_string(),
            method: "POST".to_string(),
            status_code: 401,
            request_count: 15,
            error_count: 15,
            response_time_ms: 100,
            payload_size: 512,
            headers: HashMap::new(),
            query_params: HashMap::new(),
        };
        
        engine.analyze_event(event).await;
        
        assert!(engine.is_ip_blacklisted("192.168.1.200").await);
    }

    #[tokio::test]
    async fn test_threat_resolution() {
        let engine = ThreatDetectionEngine::new();
        
        let rule = ThreatRule {
            id: "test_rule".to_string(),
            name: "Test Rule".to_string(),
            description: "Test threat rule".to_string(),
            threat_type: ThreatType::BruteForce,
            severity: ThreatSeverity::Medium,
            conditions: vec![
                ThreatCondition {
                    field: "error_count".to_string(),
                    operator: ConditionOperator::GreaterThan,
                    value: serde_json::Value::Number(serde_json::Number::from(5)),
                    time_window_seconds: Some(60),
                }
            ],
            actions: vec![],
            enabled: true,
            cooldown_seconds: 300,
            threshold: 1,
            time_window_seconds: 60,
        };
        
        engine.add_rule(rule).await;
        
        let event = SecurityEvent {
            timestamp: Utc::now(),
            ip_address: "192.168.1.300".to_string(),
            user_agent: None,
            endpoint: "/api/auth".to_string(),
            method: "POST".to_string(),
            status_code: 401,
            request_count: 10,
            error_count: 10,
            response_time_ms: 200,
            payload_size: 256,
            headers: HashMap::new(),
            query_params: HashMap::new(),
        };
        
        let threats = engine.analyze_event(event).await;
        assert_eq!(threats.len(), 1);
        
        let threat_id = &threats[0].id;
        engine.resolve_threat(threat_id).await.unwrap();
        
        let active_threats = engine.get_active_threats().await;
        assert_eq!(active_threats.len(), 0);
    }

    #[tokio::test]
    async fn test_false_positive_marking() {
        let engine = ThreatDetectionEngine::new();
        
        let rule = ThreatRule {
            id: "test_rule".to_string(),
            name: "Test Rule".to_string(),
            description: "Test threat rule".to_string(),
            threat_type: ThreatType::MaliciousTransaction,
            severity: ThreatSeverity::Low,
            conditions: vec![
                ThreatCondition {
                    field: "payload_size".to_string(),
                    operator: ConditionOperator::GreaterThan,
                    value: serde_json::Value::Number(serde_json::Number::from(1000)),
                    time_window_seconds: Some(60),
                }
            ],
            actions: vec![],
            enabled: true,
            cooldown_seconds: 300,
            threshold: 1,
            time_window_seconds: 60,
        };
        
        engine.add_rule(rule).await;
        
        let event = SecurityEvent {
            timestamp: Utc::now(),
            ip_address: "192.168.1.400".to_string(),
            user_agent: None,
            endpoint: "/api/upload".to_string(),
            method: "POST".to_string(),
            status_code: 200,
            request_count: 1,
            error_count: 0,
            response_time_ms: 1000,
            payload_size: 2000,
            headers: HashMap::new(),
            query_params: HashMap::new(),
        };
        
        let threats = engine.analyze_event(event).await;
        assert_eq!(threats.len(), 1);
        
        let threat_id = &threats[0].id;
        engine.mark_false_positive(threat_id).await.unwrap();
        
        let stats = engine.get_stats().await;
        assert_eq!(stats.false_positives, 1);
    }
} 