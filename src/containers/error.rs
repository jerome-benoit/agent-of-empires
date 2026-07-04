use thiserror::Error;

#[derive(Debug, Error)]
pub enum DockerError {
    #[error(
        "Docker is not installed or not in PATH. Install: https://docs.docker.com/get-docker/"
    )]
    NotInstalled,

    #[error(
        "Docker daemon is not running. Start Docker Desktop or run: sudo systemctl start docker"
    )]
    DaemonNotRunning,

    #[error("Docker permission denied. On Linux: add your user to the docker group (sudo usermod -aG docker $USER) and re-login")]
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

pub type Result<T> = std::result::Result<T, DockerError>;
