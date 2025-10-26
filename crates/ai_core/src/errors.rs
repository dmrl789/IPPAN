//! Error types for AI Core

use thiserror::Error;

/// Result type for AI Core operations
pub type Result<T> = std::result::Result<T, AiCoreError>;

/// AI Core error types
#[derive(Error, Debug)]
pub enum AiCoreError {
    /// Execution error
    #[error("Execution error: {0}")]
    Execution(String),
    
    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),
    
    /// Determinism error
    #[error("Determinism error: {0}")]
    Determinism(String),
    
    /// Unsupported format
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    
    /// Invalid parameters
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
    
    /// I/O error
    #[error("I/O error: {0}")]
    Io(String),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    /// Cryptographic error
    #[error("Cryptographic error: {0}")]
    Cryptographic(String),
    
    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
    
    /// Execution failed
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    
    /// Validation failed
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
}

impl From<std::io::Error> for AiCoreError {
    fn from(err: std::io::Error) -> Self {
        AiCoreError::Io(err.to_string())
    }
}

impl From<serde_json::Error> for AiCoreError {
    fn from(err: serde_json::Error) -> Self {
        AiCoreError::Serialization(err.to_string())
    }
}

impl From<toml::de::Error> for AiCoreError {
    fn from(err: toml::de::Error) -> Self {
        AiCoreError::Serialization(err.to_string())
    }
}

impl From<toml::ser::Error> for AiCoreError {
    fn from(err: toml::ser::Error) -> Self {
        AiCoreError::Serialization(err.to_string())
    }
}
