//! Error types for the AI Registry module

use thiserror::Error;

/// Errors that can occur in the AI Registry module
#[derive(Error, Debug)]
pub enum RegistryError {
    /// Model not found
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    
    /// Model already exists
    #[error("Model already exists: {0}")]
    ModelAlreadyExists(String),
    
    /// Invalid model registration
    #[error("Invalid model registration: {0}")]
    InvalidRegistration(String),
    
    /// Governance violation
    #[error("Governance violation: {0}")]
    GovernanceViolation(String),
    
    /// Fee calculation error
    #[error("Fee calculation error: {0}")]
    FeeCalculationError(String),
    
    /// Storage error
    #[error("Storage error: {0}")]
    StorageError(String),
    
    /// API error
    #[error("API error: {0}")]
    ApiError(String),
    
    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    /// Database error
    #[error("Database error: {0}")]
    Database(String),
    
    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type for AI Registry operations
pub type Result<T> = std::result::Result<T, RegistryError>;