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
//! `src/session/sync.rs`; the guard additionally owns the tempdir so a single
//! `AppDirGuard` binding covers both concerns.

use std::path::Path;
use tempfile::TempDir;

/// RAII guard that isolates `HOME` and `XDG_CONFIG_HOME` for the lifetime of a
/// single test. On `Drop`, the previous env values are restored (or removed
/// if they were unset before the guard was created), which closes the
/// process-global env leak described in issue #2306.
pub(crate) struct AppDirGuard {
    _temp: TempDir,
    prev_home: Option<String>,
    prev_xdg: Option<String>,
}

impl AppDirGuard {
    /// Returns the tempdir root. Test helpers that build paths off the
    /// isolated home should call this rather than reaching for the inner
    /// `TempDir`.
    pub(crate) fn path(&self) -> &Path {
        self._temp.path()
    }
}

impl Drop for AppDirGuard {
    fn drop(&mut self) {
        restore_or_remove("HOME", self.prev_home.take());
        restore_or_remove("XDG_CONFIG_HOME", self.prev_xdg.take());
    }
}

fn restore_or_remove(key: &str, prev: Option<String>) {
    match prev {
        Some(v) => std::env::set_var(key, v),
        None => std::env::remove_var(key),
    }
}

/// Point `HOME` (and, on Linux/macOS, `XDG_CONFIG_HOME`) at a fresh tempdir
/// and return an `AppDirGuard` that restores the previous env values on drop.
pub(crate) fn isolate_app_dir() -> AppDirGuard {
    let temp_home = TempDir::new().expect("create temp home for AppDirGuard");
    let prev_home = std::env::var("HOME").ok();
    let prev_xdg = std::env::var("XDG_CONFIG_HOME").ok();
    std::env::set_var("HOME", temp_home.path());
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    std::env::set_var("XDG_CONFIG_HOME", temp_home.path().join(".config"));
    AppDirGuard {
        _temp: temp_home,
        prev_home,
        prev_xdg,
    }
}
