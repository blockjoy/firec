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

    /// Heim process error.
    #[error("Heim process error")]
    Process(#[from] heim::process::ProcessError),

    /// HTTP error.
    #[error("HTTP error")]
    Http(#[from] hyper::http::Error),

    /// JSON error.
    #[error("JSON error")]
    Json(#[from] serde_json::Error),

    /// Integral type conversion error.
    #[error("Integral type conversion error")]
    TryFromIntError(#[from] std::num::TryFromIntError),

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

    /// Process exited early.
    #[error("Process exited early with exit status: {exit_status}")]
    ProcessExitedEarly {
        /// Result of a process after it has terminated
        exit_status: std::process::ExitStatus,
    },
}
