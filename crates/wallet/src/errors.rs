use thiserror::Error;

#[derive(Error, Debug)]
pub enum WalletError {
    #[error("Address not found in wallet: {0}")]
    AddressNotFound(String),

    #[error("Invalid address format: {0}")]
    InvalidAddress(String),

    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: u64, available: u64 },

    #[error("Invalid private key: {0}")]
    InvalidPrivateKey(String),

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Decryption error: {0}")]
    DecryptionError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Transaction error: {0}")]
    TransactionError(String),

    #[error("Wallet locked: {0}")]
    WalletLocked(String),

    #[error("Invalid password")]
    InvalidPassword,

    #[error("Wallet not initialized")]
    WalletNotInitialized,

    #[error("Address generation failed: {0}")]
    AddressGenerationFailed(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Crypto error: {0}")]
    CryptoError(String),
}

pub type Result<T> = std::result::Result<T, WalletError>;
