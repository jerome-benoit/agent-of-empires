use thiserror::Error;

#[derive(Debug, Error)]
pub enum DockerError {
    #[error(
        "Docker is not installed or not in PATH.\n\
         Install Docker: https://docs.docker.com/get-docker/"
    )]
    NotInstalled,

    #[error(
        "Docker daemon is not running.\n\
         Start Docker Desktop or run: sudo systemctl start docker"
    )]
    DaemonNotRunning,

    #[error(
        "Docker permission denied.\n\
         On Linux, add your user to the docker group:\n\
         sudo usermod -aG docker $USER\n\
         Then log out and back in."
    )]
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
    /// Single-line Display for use in structured log fields.
    ///
    /// `DaemonNotRunning` and `PermissionDenied` carry multi-line
    /// remediation prose in their `Display` (e.g. `"...\nOn Linux, add your
    /// user to the docker group:\nsudo usermod -aG docker $USER..."`) which
    /// is useful in terminal / TUI surfaces but splits a single `tracing`
    /// event across multiple physical log lines when rendered by the
    /// text-mode subscriber this project uses — `grep containers.runtime`
    /// only correlates the first line and orphans the continuation lines.
    /// Prefer this at gate-site warns (`error = %e.summary()`); keep the
    /// full `Display` for user-facing surfaces.
    pub fn summary(&self) -> String {
        self.to_string()
            .lines()
            .next()
            .unwrap_or_default()
            .to_string()
    }
}

pub type Result<T> = std::result::Result<T, DockerError>;
