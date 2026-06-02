//! Regression coverage for issue #1265.
//!
//! A hung `on_launch` hook used to pin the cross-process recovery lock
//! indefinitely (the recovery worker's `spawn_blocking` thread blocked in
//! `std::process::Command::output()`, never releasing the `flock` on
//! `<app_dir>/.recovery.lock`). The fix wraps the hook subprocess in a
//! per-thread deadline installed by `HookTimeoutScope`; the tests below
//! exercise the public `execute_hooks` entry point through that scope and
//! assert that:
//!
//! 1. a hung hook is killed and the call returns within the timeout window,
//! 2. fast hooks under a scope still succeed (no happy-path regression), and
//! 3. without a scope, behavior is unchanged for non-recovery callers.

use std::time::{Duration, Instant};

use agent_of_empires::session::{execute_hooks, HookTimeoutScope};
use serial_test::serial;
use tempfile::TempDir;

#[test]
#[serial]
fn hung_on_launch_hook_times_out_and_releases_lock_within_grace() {
    let timeout = Duration::from_millis(300);
    let _scope = HookTimeoutScope::new(timeout);

    let project = TempDir::new().expect("tempdir");
    let started = Instant::now();
    let result = execute_hooks(&["sleep 60".to_string()], project.path(), &[]);
    let elapsed = started.elapsed();

    assert!(result.is_err(), "expected timeout Err, got {:?}", result);
    let err_msg = format!("{:#}", result.unwrap_err());
    assert!(
        err_msg.contains("timed out"),
        "expected timeout-shaped error, got: {}",
        err_msg
    );
    assert!(
        elapsed < Duration::from_secs(2),
        "expected timeout to fire within 2s (300ms deadline + kill grace), took {:?}",
        elapsed
    );
}

#[test]
#[serial]
fn fast_on_launch_hook_succeeds_inside_timeout_scope() {
    let _scope = HookTimeoutScope::new(Duration::from_secs(5));

    let project = TempDir::new().expect("tempdir");
    let started = Instant::now();
    let result = execute_hooks(&["true".to_string()], project.path(), &[]);
    let elapsed = started.elapsed();

    assert!(result.is_ok(), "fast hook should succeed, got {:?}", result);
    assert!(
        elapsed < Duration::from_secs(1),
        "fast hook should complete promptly, took {:?}",
        elapsed
    );
}

#[test]
#[serial]
fn no_scope_means_no_timeout_for_non_recovery_callers() {
    let project = TempDir::new().expect("tempdir");
    let result = execute_hooks(&["true".to_string()], project.path(), &[]);
    assert!(
        result.is_ok(),
        "no-scope path must remain unchanged, got {:?}",
        result
    );
}

#[test]
#[serial]
fn nested_scopes_restore_outer_timeout_on_drop() {
    let outer = Duration::from_millis(500);
    let inner = Duration::from_millis(100);
    let outer_scope = HookTimeoutScope::new(outer);
    {
        let _inner_scope = HookTimeoutScope::new(inner);
        let project = TempDir::new().expect("tempdir");
        let started = Instant::now();
        let result = execute_hooks(&["sleep 60".to_string()], project.path(), &[]);
        let elapsed = started.elapsed();
        assert!(
            result.is_err(),
            "inner scope must enforce its tighter deadline"
        );
        assert!(
            elapsed < Duration::from_millis(1500),
            "inner deadline (100ms) must fire fast, took {:?}",
            elapsed
        );
    }
    let project = TempDir::new().expect("tempdir");
    let started = Instant::now();
    let result = execute_hooks(&["sleep 60".to_string()], project.path(), &[]);
    let elapsed = started.elapsed();
    assert!(
        result.is_err(),
        "outer scope must still enforce its deadline"
    );
    assert!(
        elapsed >= Duration::from_millis(400),
        "outer deadline (500ms) must outlive the inner scope, took {:?}",
        elapsed
    );
    assert!(
        elapsed < Duration::from_secs(2),
        "outer deadline must still bound the wait, took {:?}",
        elapsed
    );
    drop(outer_scope);
}
