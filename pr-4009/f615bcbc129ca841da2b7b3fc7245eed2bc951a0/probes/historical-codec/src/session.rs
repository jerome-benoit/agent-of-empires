use serde::{Serialize, Deserialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Running,
    Waiting,
    #[default]
    Idle,
    Unknown,
    Stopped,
    Error,
    Starting,
    Deleting,
    Creating,
}

impl Status {
    const ALL: [(Status, &'static str, &'static str); 9] = [
        (Status::Running, "running", "Running"),
        (Status::Waiting, "waiting", "Waiting"),
        (Status::Idle, "idle", "Idle"),
        (Status::Unknown, "unknown", "Unknown"),
        (Status::Stopped, "stopped", "Stopped"),
        (Status::Error, "error", "Error"),
        (Status::Starting, "starting", "Starting"),
        (Status::Deleting, "deleting", "Deleting"),
        (Status::Creating, "creating", "Creating"),
    ];

    fn spellings(self) -> (&'static str, &'static str) {
        let (_, lower, pascal) = Self::ALL[self as usize];
        (lower, pascal)
    }

    /// Lowercase CLI/hook form.
    pub fn as_str(self) -> &'static str {
        self.spellings().0
    }

    /// PascalCase HTTP API form, spelled out so a variant rename cannot change the API.
    pub fn wire_str(self) -> &'static str {
        self.spellings().1
    }

    /// Parses [`Self::wire_str`]; `None` for anything unrecognized, such as a newer daemon's status.
    pub fn from_api_str(s: &str) -> Option<Status> {
        Self::ALL
            .iter()
            .find(|(_, _, pascal)| *pascal == s)
            .map(|(status, _, _)| *status)
    }

    /// Whether the worktree may be mid-use, so an in-place worktree edit must wait.
    pub fn blocks_worktree_edit(self) -> bool {
        matches!(
            self,
            Status::Running
                | Status::Waiting
                | Status::Starting
                | Status::Creating
                | Status::Deleting
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionBucket {
    Active,
    Archived,
    Trashed,
}

enum ProfileNameError {
    Empty,
    Reserved,
    Separators,
    Component,
}

fn profile_name_error(name: &str) -> Option<ProfileNameError> {
    if name.is_empty() {
        return Some(ProfileNameError::Empty);
    }
    if name.eq_ignore_ascii_case("all") {
        return Some(ProfileNameError::Reserved);
    }
    if name.contains('\\') {
        return Some(ProfileNameError::Separators);
    }
    let mut components = Path::new(name).components();
    let first = components.next();
    if components.next().is_some() {
        return Some(ProfileNameError::Separators);
    }
    match first {
        Some(std::path::Component::Normal(component))
            if component == std::ffi::OsStr::new(name) =>
        {
            None
        }
        _ => Some(ProfileNameError::Component),
    }
}

pub(crate) fn valid_profile_name(name: &str) -> bool {
    profile_name_error(name).is_none()
}

