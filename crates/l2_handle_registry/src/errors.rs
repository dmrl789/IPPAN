//! Error types for L2 handle registry

use thiserror::Error;

#[derive(Error, Debug)]
pub enum HandleRegistryError {
    #[error("Handle not found: {handle}")]
    HandleNotFound { handle: String },

    #[error("Handle already exists: {handle}")]
    HandleAlreadyExists { handle: String },

    #[error("Invalid handle format: {handle}")]
    InvalidHandleFormat { handle: String },

    #[error("Unauthorized: insufficient permissions for handle {handle}")]
    Unauthorized { handle: String },

    #[error("Handle expired: {handle}")]
    HandleExpired { handle: String },

    #[error("Registry storage error: {0}")]
    StorageError(#[from] anyhow::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Invalid public key format")]
    InvalidPublicKey,

    #[error("Handle resolution timeout")]
    ResolutionTimeout,
}

pub type Result<T> = std::result::Result<T, HandleRegistryError>;
