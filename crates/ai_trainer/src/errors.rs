use thiserror::Error;

/// Errors returned by the deterministic trainer.
#[derive(Debug, Error)]
pub enum TrainerError {
    #[error("dataset error: {0}")]
    Dataset(String),

    #[error("training error: {0}")]
    Training(String),
}
