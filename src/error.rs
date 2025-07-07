use thiserror::Error;

/// Main error type for the IPPAN project
#[derive(Error, Debug)]
pub enum IppanError {
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Consensus error: {0}")]
    Consensus(String),
    
    #[error("Storage error: {0}")]
    Storage(String),
    
    #[error("DHT error: {0}")]
    Dht(String),
    
    #[error("Wallet error: {0}")]
    Wallet(String),
    
    #[error("Staking error: {0}")]
    Staking(String),
    
    #[error("Domain error: {0}")]
    Domain(String),
    
    #[error("API error: {0}")]
    Api(String),
    
    #[error("Cryptography error: {0}")]
    Crypto(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Timeout error: {0}")]
    Timeout(String),
    
    #[error("Invalid stake amount: {0}")]
    InvalidStakeAmount(String),
    
    #[error("Stake not found: {0}")]
    StakeNotFound(String),
    
    #[error("Stake already exists: {0}")]
    StakeAlreadyExists(String),
    
    #[error("Insufficient funds: required {required}, available {available}")]
    InsufficientFunds { required: u64, available: u64 },
    
    #[error("Invalid signature")]
    InvalidSignature,
    
    #[error("Block not found: {hash:?}")]
    BlockNotFound { hash: [u8; 32] },
    
    #[error("Transaction not found: {hash:?}")]
    TransactionNotFound { hash: [u8; 32] },
    
    #[error("Node not found: {node_id:?}")]
    NodeNotFound { node_id: [u8; 32] },
    
    #[error("Domain not found: {domain}")]
    DomainNotFound { domain: String },
    
    #[error("Storage proof failed")]
    StorageProofFailed,
    
    #[error("Invalid HashTimer: {reason}")]
    InvalidHashTimer { reason: String },
    
    #[error("Invalid IPPAN Time: expected {expected}, got {actual}")]
    InvalidIppanTime { expected: u64, actual: u64 },
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    
    #[error("Bincode error: {0}")]
    Bincode(#[from] bincode::Error),
    
    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),
}

impl From<ed25519_dalek::ed25519::Error> for IppanError {
    fn from(err: ed25519_dalek::ed25519::Error) -> Self {
        IppanError::Crypto(format!("Ed25519 error: {}", err))
    }
}

// impl From<sha2::digest::InvalidLength> for IppanError {
//     fn from(err: sha2::digest::InvalidLength) -> Self {
//         IppanError::Crypto(format!("Invalid length: {}", err))
//     }
// } 