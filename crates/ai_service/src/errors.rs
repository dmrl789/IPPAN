//! AI Service error types

use thiserror::Error;

/// AI Service errors
#[derive(Error, Debug)]
pub enum AIServiceError {
    #[error("LLM service error: {0}")]
    LLMError(String),

    #[error("Model not found: {model_id}")]
    ModelNotFound { model_id: String },

    #[error("Invalid model configuration: {0}")]
    InvalidModelConfig(String),

    #[error("Analysis failed: {0}")]
    AnalysisFailed(String),

    #[error("Smart contract error: {0}")]
    SmartContractError(String),

    #[error("Optimization failed: {0}")]
    OptimizationFailed(String),

    #[error("Monitoring error: {0}")]
    MonitoringError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Insufficient permissions")]
    InsufficientPermissions,

    #[error("Service unavailable")]
    ServiceUnavailable,

    #[error("Timeout")]
    Timeout,

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("IO error: {0}")]
    Io(String),
}

impl From<serde_json::Error> for AIServiceError {
    fn from(err: serde_json::Error) -> Self {
        AIServiceError::SerializationError(err.to_string())
    }
}

impl From<reqwest::Error> for AIServiceError {
    fn from(err: reqwest::Error) -> Self {
        AIServiceError::NetworkError(err.to_string())
    }
}

impl From<tokio::time::error::Elapsed> for AIServiceError {
    fn from(_: tokio::time::error::Elapsed) -> Self {
        AIServiceError::Timeout
    }
}
