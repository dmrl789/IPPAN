//! AI Service type definitions

#[cfg(feature = "analytics")]
use chrono::{DateTime, Utc};
use ippan_ai_core::Fixed;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// AI Service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIServiceConfig {
    /// Enable LLM features
    pub enable_llm: bool,
    /// Enable analytics features
    pub enable_analytics: bool,
    /// Enable smart contract AI
    pub enable_smart_contracts: bool,
    /// Enable monitoring features
    pub enable_monitoring: bool,
    /// LLM API configuration
    pub llm_config: LLMConfig,
    /// Analytics configuration
    pub analytics_config: AnalyticsConfig,
}

/// LLM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    /// API endpoint
    pub api_endpoint: String,
    /// API key
    pub api_key: String,
    /// Model name
    pub model_name: String,
    /// Max tokens
    pub max_tokens: u32,
    /// Temperature (micro-units)
    pub temperature: Fixed,
    /// Timeout in seconds
    pub timeout_seconds: u64,
}

/// Analytics configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnalyticsConfig {
    /// Enable real-time analytics
    pub enable_realtime: bool,
    /// Data retention days
    pub retention_days: u32,
    /// Analysis interval seconds
    pub analysis_interval: u64,
    /// Enable predictive analytics
    pub enable_predictive: bool,
}

/// LLM request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest {
    /// Prompt text
    pub prompt: String,
    /// Context data
    pub context: Option<HashMap<String, serde_json::Value>>,
    /// Max tokens
    pub max_tokens: Option<u32>,
    /// Temperature
    pub temperature: Option<Fixed>,
    /// Stream response
    pub stream: bool,
}

/// LLM response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    /// Generated text
    pub text: String,
    /// Usage statistics
    pub usage: LLMUsage,
    /// Model used
    pub model: String,
    /// Finish reason
    pub finish_reason: String,
}

/// LLM usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMUsage {
    /// Prompt tokens
    pub prompt_tokens: u32,
    /// Completion tokens
    pub completion_tokens: u32,
    /// Total tokens
    pub total_tokens: u32,
}

/// Analytics insight
#[cfg_attr(feature = "analytics", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct AnalyticsInsight {
    /// Insight ID
    pub id: String,
    /// Insight type
    pub insight_type: InsightType,
    /// Title
    pub title: String,
    /// Description
    pub description: String,
    /// Confidence score (0.0 to 1.0) stored as fixed-point
    pub confidence: Fixed,
    /// Severity level
    pub severity: SeverityLevel,
    /// Data points
    pub data_points: Vec<DataPoint>,
    /// Recommendations
    pub recommendations: Vec<String>,
    /// Timestamp
    #[cfg(feature = "analytics")]
    pub timestamp: DateTime<Utc>,
}

/// Insight types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InsightType {
    /// Performance insight
    Performance,
    /// Security insight
    Security,
    /// Economic insight
    Economic,
    /// Network insight
    Network,
    /// User behavior insight
    UserBehavior,
    /// Predictive insight
    Predictive,
}

/// Severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SeverityLevel {
    /// Low severity
    Low,
    /// Medium severity
    Medium,
    /// High severity
    High,
    /// Critical severity
    Critical,
}

/// Data point for analytics
#[cfg_attr(feature = "analytics", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct DataPoint {
    /// Metric name
    pub metric: String,
    /// Value
    pub value: Fixed,
    /// Unit
    pub unit: String,
    /// Timestamp
    #[cfg(feature = "analytics")]
    pub timestamp: DateTime<Utc>,
    /// Tags
    pub tags: HashMap<String, String>,
}

/// Smart contract analysis request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartContractAnalysisRequest {
    /// Contract code
    pub code: String,
    /// Language
    pub language: String,
    /// Analysis type
    pub analysis_type: ContractAnalysisType,
    /// Additional context
    pub context: Option<HashMap<String, serde_json::Value>>,
}

/// Smart contract analysis response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartContractAnalysisResponse {
    /// Analysis ID
    pub analysis_id: String,
    /// Security score (0.0 to 1.0) stored as fixed-point
    pub security_score: Fixed,
    /// Gas efficiency score (0.0 to 1.0) stored as fixed-point
    pub gas_efficiency_score: Fixed,
    /// Issues found
    pub issues: Vec<ContractIssue>,
    /// Recommendations
    pub recommendations: Vec<String>,
    /// Optimized code (if applicable)
    pub optimized_code: Option<String>,
    /// Analysis metadata
    pub metadata: ContractAnalysisMetadata,
}

/// Contract analysis types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContractAnalysisType {
    /// Security analysis
    Security,
    /// Gas optimization
    GasOptimization,
    /// Code quality
    CodeQuality,
    /// Comprehensive analysis
    Comprehensive,
}

/// Contract issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractIssue {
    /// Issue type
    pub issue_type: String,
    /// Severity
    pub severity: SeverityLevel,
    /// Description
    pub description: String,
    /// Line number
    pub line_number: Option<u32>,
    /// Suggested fix
    pub suggested_fix: Option<String>,
}

/// Contract analysis metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractAnalysisMetadata {
    /// Lines of code
    pub lines_of_code: u32,
    /// Complexity score
    pub complexity_score: Fixed,
    /// Analysis duration
    pub analysis_duration_ms: u64,
    /// Tools used
    pub tools_used: Vec<String>,
}

/// Transaction optimization request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionOptimizationRequest {
    /// Transaction data
    pub transaction: TransactionData,
    /// Optimization goals
    pub goals: Vec<OptimizationGoal>,
    /// Constraints
    pub constraints: Option<OptimizationConstraints>,
}

/// Transaction optimization response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionOptimizationResponse {
    /// Optimized transaction
    pub optimized_transaction: TransactionData,
    /// Optimization suggestions
    pub suggestions: Vec<OptimizationSuggestion>,
    /// Expected improvements
    pub expected_improvements: HashMap<String, Fixed>,
    /// Confidence score
    pub confidence: Fixed,
}

/// Transaction data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionData {
    /// Transaction type
    pub tx_type: String,
    /// From address
    pub from: String,
    /// To address
    pub to: String,
    /// Amount
    pub amount: u64,
    /// Gas limit
    pub gas_limit: u64,
    /// Gas price
    pub gas_price: u64,
    /// Data
    pub data: Vec<u8>,
    /// Nonce
    pub nonce: u64,
}

/// Optimization goals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationGoal {
    /// Minimize gas usage
    MinimizeGas,
    /// Maximize throughput
    MaximizeThroughput,
    /// Minimize latency
    MinimizeLatency,
    /// Maximize security
    MaximizeSecurity,
    /// Minimize cost
    MinimizeCost,
}

/// Optimization constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConstraints {
    /// Maximum gas limit
    pub max_gas_limit: Option<u64>,
    /// Maximum gas price
    pub max_gas_price: Option<u64>,
    /// Maximum latency
    pub max_latency_ms: Option<u64>,
    /// Security requirements
    pub security_requirements: Vec<String>,
}

/// Optimization suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSuggestion {
    /// Suggestion type
    pub suggestion_type: String,
    /// Description
    pub description: String,
    /// Expected improvement
    pub expected_improvement: Fixed,
    /// Implementation difficulty
    pub difficulty: DifficultyLevel,
}

/// Difficulty levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DifficultyLevel {
    /// Easy
    Easy,
    /// Medium
    Medium,
    /// Hard
    Hard,
}

/// Monitoring alert
#[cfg_attr(feature = "analytics", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct MonitoringAlert {
    /// Alert ID
    pub alert_id: String,
    /// Alert type
    pub alert_type: String,
    /// Severity
    pub severity: SeverityLevel,
    /// Title
    pub title: String,
    /// Description
    pub description: String,
    /// Metrics
    pub metrics: HashMap<String, Fixed>,
    /// Timestamp
    #[cfg(feature = "analytics")]
    pub timestamp: DateTime<Utc>,
    /// Status
    pub status: AlertStatus,
    /// Actions taken
    pub actions_taken: Vec<String>,
}

/// Alert status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertStatus {
    /// Active
    Active,
    /// Acknowledged
    Acknowledged,
    /// Resolved
    Resolved,
    /// Suppressed
    Suppressed,
}

impl Default for AIServiceConfig {
    fn default() -> Self {
        Self {
            enable_llm: true,
            enable_analytics: true,
            enable_smart_contracts: true,
            enable_monitoring: true,
            llm_config: LLMConfig {
                api_endpoint: "https://api.openai.com/v1".to_string(),
                api_key: "".to_string(),
                model_name: "gpt-4".to_string(),
                max_tokens: 4000,
                temperature: Fixed::from_ratio(7, 10),
                timeout_seconds: 30,
            },
            analytics_config: AnalyticsConfig {
                enable_realtime: true,
                retention_days: 30,
                analysis_interval: 60,
                enable_predictive: true,
            },
        }
    }
}
