use hyper::StatusCode;
use thiserror::Error;

/// Error type for this crate.
#[derive(Debug, Error)]
pub enum Error {
    /// Failed to generate UUID.
    #[error("Failed to generate UUID: {0}")]
    Uuid(#[from] uuid::Error),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Hyper error.
    #[error("Hyper error: {0}")]
    Hyper(#[from] hyper::Error),

    /// HTTP error.
    #[error("HTTP error: {0}")]
    Http(#[from] hyper::http::Error),

    /// JSON error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Integral type conversion error.
    #[error("Integral type conversion error: {0}")]
    TryFromIntError(#[from] std::num::TryFromIntError),

    /// Task join error.
    #[error("Task join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),

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

    /// Firecracker REST API error
    #[error("Firecracker API call failed with status={status}, body={body:?}")]
    FirecrackerAPIError {
        /// Error HTTP status code
        status: StatusCode,
        /// Optional error message body
        body: Option<String>,
    },

    /// Process not started
    #[error("Process not started")]
    ProcessNotStarted,

    /// Process not running
    #[error("Process not running for pid: {0}")]
    ProcessNotRunning(i32),

    /// Process not killed
    #[error("Process not killed for pid: {0}")]
    ProcessNotKilled(i32),

    /// Process exited immediatelly after start.
    #[error("Process exited immediatelly with status: {exit_status}")]
    ProcessExitedImmediatelly {
        /// Result of a process after it has terminated
        exit_status: std::process::ExitStatus,
    },
}
