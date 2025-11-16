use serde::{Deserialize, Serialize};
use serde_bytes;
use std::collections::BTreeMap;
use thiserror::Error;

/// Structured payload describing a handle-specific transaction operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HandleOperation {
    /// Register a new handle on-chain.
    Register(HandleRegisterOp),
}

impl HandleOperation {
    /// Returns the owner bytes associated with the operation.
    pub fn owner_bytes(&self) -> &[u8; 32] {
        match self {
            HandleOperation::Register(op) => &op.owner,
        }
    }

    /// Returns the handle string targeted by this operation.
    pub fn handle(&self) -> &str {
        match self {
            HandleOperation::Register(op) => op.handle.as_str(),
        }
    }

    /// Optional expiry timestamp (seconds since UNIX_EPOCH).
    pub fn expires_at(&self) -> Option<u64> {
        match self {
            HandleOperation::Register(op) => op.expires_at,
        }
    }

    /// Raw signature bytes backing the handle operation.
    pub fn signature(&self) -> &[u8] {
        match self {
            HandleOperation::Register(op) => &op.signature,
        }
    }

    /// Metadata map attached to the handle.
    pub fn metadata(&self) -> &BTreeMap<String, String> {
        match self {
            HandleOperation::Register(op) => &op.metadata,
        }
    }

    /// Basic semantic validation tied to the originating transaction sender.
    pub fn validate_for_sender(&self, sender: &[u8; 32]) -> Result<(), HandleOperationError> {
        match self {
            HandleOperation::Register(op) => {
                if &op.owner != sender {
                    return Err(HandleOperationError::OwnerMismatch);
                }
                op.validate_handle()?;
                op.validate_signature_length()?;
                Ok(())
            }
        }
    }
}

/// Registration payload embedded inside a transaction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HandleRegisterOp {
    pub handle: String,
    pub owner: [u8; 32],
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    #[serde(default)]
    pub expires_at: Option<u64>,
    #[serde(with = "serde_bytes")]
    pub signature: Vec<u8>,
}

impl HandleRegisterOp {
    fn validate_handle(&self) -> Result<(), HandleOperationError> {
        let handle = self.handle.trim();
        if !(handle.starts_with('@') && handle.contains('.') && handle.len() > 3) {
            return Err(HandleOperationError::InvalidHandle);
        }
        Ok(())
    }

    fn validate_signature_length(&self) -> Result<(), HandleOperationError> {
        if self.signature.len() != 64 {
            return Err(HandleOperationError::InvalidSignatureLength);
        }
        Ok(())
    }
}

/// Errors raised during embedded handle validation.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum HandleOperationError {
    #[error("handle registration owner must match transaction sender")]
    OwnerMismatch,
    #[error("handle string must start with '@' and include a suffix (e.g. '@user.ipn')")]
    InvalidHandle,
    #[error("handle registration signature must be 64 bytes")]
    InvalidSignatureLength,
}
