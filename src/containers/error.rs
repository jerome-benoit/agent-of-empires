use thiserror::Error;

#[derive(Debug, Error)]
pub enum DockerError {
    #[error("Docker is not installed or not in PATH")]
    NotInstalled,

    #[error("Docker daemon is not running")]
    DaemonNotRunning,

    #[error("Docker permission denied")]
    PermissionDenied,

    #[error("Container not found: {0}")]
    ContainerNotFound(String),

    #[error("Container already exists: {0}")]
    ContainerAlreadyExists(String),

    #[error("Docker image not found: {0}")]
    ImageNotFound(String),

    #[error("Failed to create container: {0}")]
    CreateFailed(String),

    #[error("Failed to start container: {0}")]
    StartFailed(String),

    #[error("Failed to stop container: {0}")]
    StopFailed(String),

    #[error("Failed to remove container: {0}")]
    RemoveFailed(String),

    #[error("Failed to inspect container: {0}")]
    InspectFailed(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl DockerError {
    /// Multi-line remediation hint suitable for user-facing error surfaces
    /// (TUI dialogs, CLI stderr on final failure). Returns `None` for variants
    /// whose `Display` is already self-explanatory or whose context is
    /// variable (e.g. `InspectFailed` carries raw stderr; no fixed
    /// remediation applies).
    ///
    /// `Display` is deliberately single-line by default so `error = %e` at
    /// `tracing::warn!` sites produces one physical log line. Call this
    /// accessor where the surface can render multi-line prose.
    pub fn remediation(&self) -> Option<&'static str> {
        match self {
            Self::NotInstalled => Some("Install Docker: https://docs.docker.com/get-docker/"),
            Self::DaemonNotRunning => {
                Some("Start Docker Desktop or run: sudo systemctl start docker")
            }
            Self::PermissionDenied => Some(
                "On Linux, add your user to the docker group:\n\
                 sudo usermod -aG docker $USER\n\
                 Then log out and back in.",
            ),
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, DockerError>;
