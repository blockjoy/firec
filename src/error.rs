use thiserror::Error;

/// Error type for this crate.
#[derive(Debug, Error)]
pub enum Error {
    /// Failed to generate UUID.
    #[error("Failed to generate UUID")]
    Uuid(#[from] uuid::Error),
}