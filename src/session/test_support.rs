//! Shared test-support helpers.
//!
//! Tests across the crate need to point `HOME` (and, on Linux/macOS,
//! `XDG_CONFIG_HOME`) at an isolated temporary directory so `get_app_dir_path`
//! resolves to a scratch location instead of the developer's real config.
//! Returning a bare `TempDir` from the old duplicated `isolate_app_dir`
//! helpers meant the tempdir was cleaned up on drop, but the env vars leaked
//! into the process for the rest of the test run, poisoning any later test
//! that read them. See issue #2306.
//!
//! `AppDirGuard` owns both the tempdir and a snapshot of the previous env
//! values, so its `Drop` closes the leak: env vars are restored to their
//! prior state (or removed if they were previously unset) even if the test
//! panics. The `Drop` shape mirrors `StorageHomeGuard` at
//! [`crate::session::sync`] (see `src/session/sync.rs:346-377`); the guard
//! additionally owns the tempdir so a single `AppDirGuard` binding covers
//! both concerns.

use std::ffi::OsString;
use std::path::Path;
use tempfile::TempDir;

/// RAII guard: isolates `HOME` and `XDG_CONFIG_HOME` for one test; restores
/// them on `Drop`. See the module doc for the process-global env-leak
/// motivation (issue #2306).
pub(crate) struct AppDirGuard {
    temp: TempDir,
    prev_home: Option<OsString>,
    prev_xdg: Option<OsString>,
}

impl AppDirGuard {
    /// Returns the tempdir root.
    pub(crate) fn path(&self) -> &Path {
        self.temp.path()
    }
}

/// Blanket-friendly path access: `app_dir(&guard)` works wherever
/// `impl AsRef<Path>` is accepted. Also enables `&AppDirGuard: AsRef<Path>`
/// via the standard-library blanket `impl<T: AsRef<U>, U> AsRef<U> for &T`.
impl AsRef<Path> for AppDirGuard {
    fn as_ref(&self) -> &Path {
        self.path()
    }
}

impl Drop for AppDirGuard {
    fn drop(&mut self) {
        restore_or_remove("HOME", self.prev_home.take());
        restore_or_remove("XDG_CONFIG_HOME", self.prev_xdg.take());
    }
}

fn restore_or_remove(key: &str, prev: Option<OsString>) {
    match prev {
        Some(v) => std::env::set_var(key, v),
        None => std::env::remove_var(key),
    }
}

/// Isolate the app dir for one test.
///
/// Points `HOME` at a fresh tempdir root, and on Linux/macOS points
/// `XDG_CONFIG_HOME` at `<tempdir>/.config` (the shape `get_app_dir_path`
/// resolves against). The two vars land at different paths on purpose: the
/// crate's config resolution uses `HOME` for user-scoped paths and
/// `XDG_CONFIG_HOME` for the crate's own dot-dir subtree.
///
/// The prior values are snapshotted via [`std::env::var_os`] (`OsString`,
/// not `String`) so a non-UTF-8 prior value survives round-tripping through
/// the guard's `Drop`.
pub(crate) fn isolate_app_dir() -> AppDirGuard {
    let temp_home = TempDir::new().expect("create tempdir for AppDirGuard");
    let prev_home = std::env::var_os("HOME");
    let prev_xdg = std::env::var_os("XDG_CONFIG_HOME");
    std::env::set_var("HOME", temp_home.path());
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    std::env::set_var("XDG_CONFIG_HOME", temp_home.path().join(".config"));
    AppDirGuard {
        temp: temp_home,
        prev_home,
        prev_xdg,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    /// Locks the fix for #2306: `Drop` MUST restore `HOME` and (on
    /// Linux/macOS) `XDG_CONFIG_HOME` to their pre-guard values, and MUST
    /// remove them when the pre-guard value was unset. A future refactor
    /// that quietly drops the `Drop` impl would reintroduce the leak this
    /// PR closes.
    #[test]
    #[serial]
    fn app_dir_guard_drop_restores_env_vars() {
        // Snapshot the ambient env before any guard construction.
        let before_home = std::env::var_os("HOME");
        let before_xdg = std::env::var_os("XDG_CONFIG_HOME");

        {
            let guard = isolate_app_dir();
            // Construction path: HOME is set to the tempdir root, and on
            // Linux/macOS XDG_CONFIG_HOME is set to `<tempdir>/.config`.
            assert_eq!(
                std::env::var_os("HOME").as_deref(),
                Some(guard.path().as_os_str()),
                "HOME must point at the guard's tempdir during the test body"
            );
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            {
                let expected = guard.path().join(".config");
                assert_eq!(
                    std::env::var_os("XDG_CONFIG_HOME"),
                    Some(expected.into_os_string()),
                    "XDG_CONFIG_HOME must point at <tempdir>/.config"
                );
            }
        } // guard drops here; env restored.

        assert_eq!(
            std::env::var_os("HOME"),
            before_home,
            "HOME must be restored on guard Drop"
        );
        assert_eq!(
            std::env::var_os("XDG_CONFIG_HOME"),
            before_xdg,
            "XDG_CONFIG_HOME must be restored on guard Drop"
        );
    }

    /// `AsRef<Path>` lets call sites pass `&guard` wherever a
    /// `Path`-like is expected, matching `Path::join`-style ergonomics.
    #[test]
    #[serial]
    fn app_dir_guard_as_ref_path_matches_path() {
        let guard = isolate_app_dir();
        let via_as_ref: &Path = guard.as_ref();
        assert_eq!(via_as_ref, guard.path());
    }
}
