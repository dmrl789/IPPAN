//! Error types for the AI Core module

use thiserror::Error;

/// Errors that can occur in the AI Core module
#[derive(Error, Debug)]
pub enum AiCoreError {
    /// Model execution failed
    #[error("Model execution failed: {0}")]
    ExecutionFailed(String),
    
    /// Model validation failed
    #[error("Model validation failed: {0}")]
    ValidationFailed(String),
    
    /// Deterministic execution violation
    #[error("Deterministic execution violation: {0}")]
    DeterminismViolation(String),
    
    /// Model format not supported
    #[error("Unsupported model format: {0}")]
    UnsupportedFormat(String),
    
    /// Invalid model parameters
    #[error("Invalid model parameters: {0}")]
    InvalidParameters(String),
    
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    /// Cryptographic error
    #[error("Cryptographic error: {0}")]
    Crypto(String),
    
    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type for AI Core operations
pub type Result<T> = std::result::Result<T, AiCoreError>;