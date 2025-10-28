//! Error types for validator resolution

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ValidatorResolutionError {
    #[error("Invalid validator ID format: {id}")]
    InvalidFormat { id: String },

    #[error("Handle not found: {handle}")]
    HandleNotFound { handle: String },

    #[error("Handle resolution timeout")]
    ResolutionTimeout,

    #[error("Invalid public key format")]
    InvalidPublicKey,

    #[error("L2 handle registry error: {0}")]
    L2RegistryError(#[from] ippan_l2_handle_registry::HandleRegistryError),

    #[error("L1 anchor error: {0}")]
    L1AnchorError(#[from] ippan_l1_handle_anchors::HandleAnchorError),

    #[error("Resolution service error: {0}")]
    ServiceError(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, ValidatorResolutionError>;
