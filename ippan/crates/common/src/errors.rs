use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Cryptographic error: {0}")]
    Crypto(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Transaction error: {0}")]
    Transaction(String),

    #[error("State error: {0}")]
    State(String),

    #[error("Block error: {0}")]
    Block(String),

    #[error("Round error: {0}")]
    Round(String),

    #[error("Mempool error: {0}")]
    Mempool(String),

    #[error("Time error: {0}")]
    Time(String),

    #[error("Wallet error: {0}")]
    Wallet(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<ed25519_dalek::ed25519::Error> for Error {
    fn from(err: ed25519_dalek::ed25519::Error) -> Self {
        Error::Crypto(err.to_string())
    }
}

impl From<bincode::Error> for Error {
    fn from(err: bincode::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}
