use thiserror::Error;

#[derive(Error, Debug)]
pub enum GhostForgeError {
    #[error("Game not found: {0}")]
    GameNotFound(String),

    #[error("Wine/Proton version not found: {0}")]
    WineVersionNotFound(String),

    #[error("Launcher not configured: {0}")]
    LauncherNotConfigured(String),

    #[error("Installation failed: {0}")]
    InstallationFailed(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Wine prefix error: {0}")]
    PrefixError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("System command failed: {0}")]
    CommandFailed(String),

    #[error("GPU not supported: {0}")]
    GpuNotSupported(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

pub type Result<T> = std::result::Result<T, GhostForgeError>;