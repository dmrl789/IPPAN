use thiserror::Error;

/// Errors that can occur when communicating with an IPPAN node or gateway.
#[derive(Debug, Error)]
pub enum SdkError {
    #[error("invalid base URL: {0}")]
    InvalidBaseUrl(String),
    #[error("url error: {0}")]
    Url(#[from] url::ParseError),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("server error (status {status}, code {code}): {message}")]
    ServerError {
        status: u16,
        code: String,
        message: String,
    },
    #[error("parse error: {0}")]
    Parse(String),
}

impl SdkError {
    pub(crate) fn parse_error(msg: impl Into<String>) -> Self {
        SdkError::Parse(msg.into())
    }

    pub(crate) fn server_error(
        status: u16,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        SdkError::ServerError {
            status,
            code: code.into(),
            message: message.into(),
        }
    }
}
