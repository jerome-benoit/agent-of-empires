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
//! [`crate::session::sync`]; the guard additionally owns the tempdir so a
//! single `AppDirGuard` binding covers both concerns.
//!
//! Scope: the guard isolates tests on Linux and macOS (the crate's
//! supported native targets; Windows is WSL2-only per the README). On
//! native Windows `dirs::home_dir()` resolves via
//! `SHGetKnownFolderPath(FOLDERID_Profile)`, not `$HOME`, so a
//! `set_var("HOME", ...)` alone would not redirect
//! `get_app_dir_path`. Isolating tests on native Windows would need a
//! different mechanism (e.g. a stub for the profile-folder lookup)
//! rather than more env vars in this snapshot set.

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
    use std::panic::AssertUnwindSafe;

    /// Locks the fix for #2306: `Drop` MUST restore `HOME` and (on
    /// Linux/macOS) `XDG_CONFIG_HOME` to their pre-guard values. A future
    /// refactor that quietly drops the `Drop` impl would reintroduce the
    /// leak this PR closes.
    #[test]
    #[serial]
    fn app_dir_guard_drop_restores_env_vars() {
        let before_home = std::env::var_os("HOME");
        let before_xdg = std::env::var_os("XDG_CONFIG_HOME");

        {
            let guard = isolate_app_dir();
            // Byte-identity holds because `TempDir::path()` is
            // un-canonicalized on macOS (`/var/folders/...`, a symlink to
            // `/private/var/folders/...`) and `set_var` writes the bytes
            // verbatim. A future refactor that canonicalizes one side
            // without the other would silently break this on macOS.
            assert_eq!(
                std::env::var_os("HOME"),
                Some(guard.path().as_os_str().to_os_string()),
                "HOME must point at the guard's tempdir during the test body"
            );
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            assert_eq!(
                std::env::var_os("XDG_CONFIG_HOME"),
                Some(guard.path().join(".config").into_os_string()),
                "XDG_CONFIG_HOME must point at <tempdir>/.config"
            );
        }

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

    /// Locks the `remove_var` branch of `restore_or_remove`: when the
    /// pre-guard env var was unset, `Drop` MUST leave it unset. Under
    /// Unix CI HOME is always set, so [`app_dir_guard_drop_restores_env_vars`]
    /// above only exercises the `set_var` restoration branch. This test
    /// forces the `None` snapshot by removing both vars before construction.
    #[test]
    #[serial]
    fn app_dir_guard_drop_removes_env_vars_when_unset() {
        let before_home = std::env::var_os("HOME");
        let before_xdg = std::env::var_os("XDG_CONFIG_HOME");
        std::env::remove_var("HOME");
        std::env::remove_var("XDG_CONFIG_HOME");

        {
            let guard = isolate_app_dir();
            // Explicit path check (not just `is_some`) catches a
            // constructor mutation that writes the wrong path when the
            // prior snapshot was `None`.
            assert_eq!(
                std::env::var_os("HOME"),
                Some(guard.path().as_os_str().to_os_string()),
                "constructor must set HOME to the guard tempdir even when the prior value was unset"
            );
        }

        assert_eq!(
            std::env::var_os("HOME"),
            None,
            "HOME must be removed on Drop when it was unset before construction"
        );
        assert_eq!(
            std::env::var_os("XDG_CONFIG_HOME"),
            None,
            "XDG_CONFIG_HOME must stay unset on Drop when it was unset before construction"
        );

        // Restore the ambient env for any downstream serial test that
        // reads it. `#[serial]` sequences tests but does not itself reset
        // the process env after each one.
        if let Some(v) = before_home {
            std::env::set_var("HOME", v);
        }
        if let Some(v) = before_xdg {
            std::env::set_var("XDG_CONFIG_HOME", v);
        }
    }

    /// `AsRef<Path>` lets call sites pass `&guard` wherever a
    /// `Path`-like is expected, matching `Path::join`-style ergonomics.
    #[test]
    #[serial]
    fn app_dir_guard_as_ref_path_matches_path() {
        let guard = isolate_app_dir();
        let via_as_ref: &Path = guard.as_ref();
        assert_eq!(
            via_as_ref,
            guard.path(),
            "AsRef<Path>::as_ref must return the same path as AppDirGuard::path"
        );
    }

    /// Locks the "Drop-runs-on-unwind" contract that motivates the entire
    /// RAII conversion. A future refactor that swaps `AppDirGuard` for a
    /// non-RAII helper (e.g., a `fn cleanup(&self)` the test must call
    /// explicitly) would let a panicking test leak `HOME`/`XDG_CONFIG_HOME`
    /// exactly the way pre-#2306 did.
    #[test]
    #[serial]
    fn app_dir_guard_drop_restores_env_vars_on_panic() {
        let before_home = std::env::var_os("HOME");
        let before_xdg = std::env::var_os("XDG_CONFIG_HOME");

        let unwound = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let _guard = isolate_app_dir();
            panic!("simulate a test-body panic while the guard is live");
        }));
        assert!(
            unwound.is_err(),
            "the inner panic must actually propagate to catch_unwind"
        );

        assert_eq!(
            std::env::var_os("HOME"),
            before_home,
            "HOME must be restored on guard Drop even when the test body panics"
        );
        assert_eq!(
            std::env::var_os("XDG_CONFIG_HOME"),
            before_xdg,
            "XDG_CONFIG_HOME must be restored on guard Drop even when the test body panics"
        );
    }

    /// Locks the "snapshot at construction" semantic: `Drop` restores to
    /// the pre-construction env values, not to whatever the test last
    /// wrote inside the guard's scope. A refactor that moves the
    /// `env::var_os` snapshot from the constructor into `Drop::drop`
    /// would pass every other test in this module but survive as a
    /// silent regression here.
    #[test]
    #[serial]
    fn app_dir_guard_drop_ignores_mid_scope_env_writes() {
        let before_home = std::env::var_os("HOME");

        {
            let _guard = isolate_app_dir();
            // Mid-scope write that must NOT survive `Drop`. Not the
            // guard's tempdir; a distinct sentinel path.
            std::env::set_var("HOME", "/tmp/aoe-mid-scope-sentinel");
            assert_eq!(
                std::env::var_os("HOME"),
                Some(OsString::from("/tmp/aoe-mid-scope-sentinel")),
                "mid-scope write must land while the guard is live"
            );
        }

        assert_eq!(
            std::env::var_os("HOME"),
            before_home,
            "Drop must restore the pre-construction snapshot, not the mid-scope write"
        );
    }
}
