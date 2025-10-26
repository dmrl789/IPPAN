//! Error types for L1 handle anchors

use thiserror::Error;

#[derive(Error, Debug)]
pub enum HandleAnchorError {
    #[error("Invalid ownership proof")]
    InvalidOwnershipProof,
    
    #[error("Handle anchor not found: {handle_hash}")]
    AnchorNotFound { handle_hash: String },
    
    #[error("Invalid signature")]
    InvalidSignature,
    
    #[error("Anchor expired")]
    AnchorExpired,
    
    #[error("Storage error: {0}")]
    StorageError(#[from] anyhow::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, HandleAnchorError>;