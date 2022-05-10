use thiserror::Error;

/// Error type for this crate.
#[derive(Debug, Error)]
pub enum Error {
    /// Failed to generate UUID.
    #[error("Failed to generate UUID")]
    Uuid(#[from] uuid::Error),

    /// IO error.
    #[error("IO error")]
    Io(#[from] std::io::Error),

    /// Hyper error.
    #[error("Hyper error")]
    Hyper(#[from] hyper::Error),

    /// HTTP error.
    #[error("HTTP error")]
    Http(#[from] hyper::http::Error),

    /// Invalid Jailer executable path specified.
    #[error("Invalid Jailer executable path specified")]
    InvalidJailerExecPath,

    /// Invalid initrd path specified.
    #[error("Invalid initrd path specified")]
    InvalidInitrdPath,

    /// Invalid socket path specified.
    #[error("Invalid socket path specified")]
    InvalidSocketPath,

    /// Invalid drive path specified.
    #[error("Invalid drive path specified")]
    InvalidDrivePath,

    /// Invalid chroot base path specified.
    #[error("Invalid chroot base path specified")]
    InvalidChrootBasePath,
}
