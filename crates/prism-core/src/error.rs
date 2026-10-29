use thiserror::Error;

/// Core error types for the prisM. Suite launcher.
#[derive(Debug, Error)]
pub enum PrismError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization / Deserialization error: {0}")]
    Ron(#[from] ron::Error),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Security / Signature verification failed: {0}")]
    Security(String),

    #[error("App installation error: {0}")]
    Install(String),

    #[error("App not found in catalog: {0}")]
    AppNotFound(String),
}
