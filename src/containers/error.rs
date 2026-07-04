use thiserror::Error;

/// Errors from Docker / Podman / Apple Container operations.
///
/// `Display` strings are single-line by convention: the three unit variants
/// (`NotInstalled`, `DaemonNotRunning`, `PermissionDenied`) inline their
/// actionable remediation on one line, and the string-carrying variants
/// (`InspectFailed`, `RemoveFailed`, etc.) must be constructed via
/// [`sanitize_stderr`] to normalize any embedded newlines from raw runtime
/// stderr. This lets `tracing::warn!(error = %e)` at gate sites emit one
/// physical log line, keeping `grep target` correlation intact under the
/// text-mode subscriber this project uses.
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

/// Collapse embedded newlines and trim so a raw runtime stderr string,
/// when wrapped in a [`DockerError`] string-carrying variant, still
/// renders as a single-line `Display`.
///
/// Docker / Podman / Apple stderr routinely contains `\n` between error
/// summary lines. Without this, `tracing::warn!(error = %e)` on a gate
/// site would split one logical event across physical log lines,
/// re-introducing the exact grep-hostility Round 10's single-line
/// convention fixed for the unit variants. Call at every construction
/// site that wraps stderr into a `DockerError` variant.
pub(crate) fn sanitize_stderr(stderr: &str) -> String {
    stderr.trim().replace('\n', " | ")
}

pub type Result<T> = std::result::Result<T, DockerError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_strings_are_single_line_and_actionable() {
        for e in [
            DockerError::NotInstalled,
            DockerError::DaemonNotRunning,
            DockerError::PermissionDenied,
        ] {
            let s = e.to_string();
            assert!(!s.contains('\n'), "Display must be single-line: {s:?}");
            assert!(s.len() < 160, "Display should fit terminal-wrap: {s:?}");
        }
        assert!(DockerError::NotInstalled
            .to_string()
            .contains("docs.docker.com"));
        assert!(DockerError::DaemonNotRunning
            .to_string()
            .contains("systemctl"));
        assert!(DockerError::PermissionDenied
            .to_string()
            .contains("usermod"));
    }

    #[test]
    fn parameterized_variants_stay_single_line_when_stderr_multiline() {
        let raw = "Error response from daemon: something failed\nAdditional context: line two\n";
        let sanitized = sanitize_stderr(raw);
        let e = DockerError::InspectFailed(sanitized);
        let rendered = e.to_string();
        assert!(
            !rendered.contains('\n'),
            "sanitize_stderr must strip newlines from parameterized variant Display: {rendered:?}"
        );
        assert!(
            rendered.contains(" | "),
            "sanitize_stderr should join lines with ` | ` for readability: {rendered:?}"
        );
    }

    #[test]
    fn sanitize_stderr_trims_trailing_whitespace() {
        assert_eq!(sanitize_stderr("some error\n"), "some error");
        assert_eq!(sanitize_stderr("  padded  \n"), "padded");
        assert_eq!(sanitize_stderr("a\nb\nc"), "a | b | c");
    }
}
