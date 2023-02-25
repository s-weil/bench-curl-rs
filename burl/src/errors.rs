use thiserror::Error;

#[derive(Error, Debug)]
pub enum BurlError {
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerDe(#[from] serde_json::Error),

    #[error("Invalid configuration: {issue}")]
    InvalidConfig { issue: String },
}

pub type BurlResult<T> = Result<T, BurlError>;
