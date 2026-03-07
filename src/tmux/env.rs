//! Hidden environment variable helpers for tmux sessions
//!
//! This module provides utilities to get and set hidden environment variables
//! in tmux sessions using the `-h` flag. Hidden variables are not inherited by
//! child processes, making them ideal for storing session metadata.

use anyhow::bail;
use std::process::Command;

pub const AOE_INSTANCE_ID_KEY: &str = "AOE_INSTANCE_ID";
pub const AOE_CAPTURED_SESSION_ID_KEY: &str = "AOE_CAPTURED_SESSION_ID";

/// Set a hidden environment variable in a tmux session
///
/// Hidden variables (set with `-h`) are not inherited by child processes.
pub fn set_hidden_env(session_name: &str, key: &str, value: &str) -> anyhow::Result<()> {
    let output = Command::new("tmux")
        .args(["set-environment", "-h", "-t", session_name, key, value])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to set hidden env var: {}", stderr);
    }

    Ok(())
}

/// Get a hidden environment variable from a tmux session
///
/// Returns `None` if the variable is unset or if the command fails.
pub fn get_hidden_env(session_name: &str, key: &str) -> Option<String> {
    let output = Command::new("tmux")
        .args(["show-environment", "-h", "-t", session_name, key])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.trim();

    // tmux outputs "-KEY" when the variable is unset
    if line.starts_with('-') {
        return None;
    }

    // Parse "KEY=VALUE" format
    if let Some((_, value)) = line.split_once('=') {
        Some(value.to_string())
    } else {
        None
    }
}

/// Remove a hidden environment variable from a tmux session
pub fn remove_hidden_env(session_name: &str, key: &str) -> anyhow::Result<()> {
    let output = Command::new("tmux")
        .args(["set-environment", "-h", "-u", "-t", session_name, key])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to remove hidden env var: {}", stderr);
    }

    Ok(())
}

/// Clear all AoE hidden environment variables from a tmux session.
///
/// Best-effort: logs warnings on failure rather than propagating errors,
/// since stale env vars are harmless if the session is about to be recreated.
pub fn clear_all_hidden_env(session_name: &str) {
    for key in [AOE_INSTANCE_ID_KEY, AOE_CAPTURED_SESSION_ID_KEY] {
        if let Err(e) = remove_hidden_env(session_name, key) {
            tracing::warn!("Failed to clear stale {key} env var: {e}");
        }
    }
}

/// Get hidden environment variables from multiple sessions in a single tmux command
///
/// Attempts to batch-read from all sessions with a single command. Falls back to
/// sequential reads if the batch command fails.
///
/// Returns a vector of (session_name, value) tuples in the same order as input.
pub fn get_hidden_env_batch(session_names: &[&str], key: &str) -> Vec<(String, Option<String>)> {
    if session_names.is_empty() {
        return Vec::new();
    }

    // Try batch command first
    let mut args = vec!["show-environment".to_string(), "-h".to_string()];
    for session_name in session_names {
        args.push("-t".to_string());
        args.push(session_name.to_string());
        args.push(key.to_string());
        args.push(";".to_string());
    }

    // Remove trailing semicolon if present
    if args.last().is_some_and(|s| s == ";") {
        args.pop();
    }

    let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let output = Command::new("tmux").args(&str_args).output();

    let fallback = || {
        session_names
            .iter()
            .map(|name| (name.to_string(), get_hidden_env(name, key)))
            .collect()
    };

    match output {
        Ok(out) if out.status.success() => {
            parse_batch_output(&String::from_utf8_lossy(&out.stdout), session_names)
                .unwrap_or_else(fallback)
        }
        _ => fallback(),
    }
}

/// Parse output from batch show-environment command.
///
/// Each session's output is on a separate line in the format "KEY=VALUE" or "-KEY".
/// If the number of output lines does not match the number of sessions (e.g. due to
/// tmux error lines), returns `None` so the caller can fall back to sequential reads.
fn parse_batch_output(
    output: &str,
    session_names: &[&str],
) -> Option<Vec<(String, Option<String>)>> {
    let lines: Vec<&str> = output.lines().collect();
    if lines.len() != session_names.len() {
        return None;
    }
    let mut results = Vec::new();

    for (i, session_name) in session_names.iter().enumerate() {
        if i < lines.len() {
            let line = lines[i].trim();
            let value = if line.starts_with('-') {
                None
            } else if let Some((_, val)) = line.split_once('=') {
                Some(val.to_string())
            } else {
                None
            };
            results.push((session_name.to_string(), value));
        } else {
            results.push((session_name.to_string(), None));
        }
    }

    Some(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_key_value() {
        let output = "AOE_INSTANCE_ID=abc123";
        let result = parse_batch_output(output, &["test_session"]).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "test_session");
        assert_eq!(result[0].1, Some("abc123".to_string()));
    }

    #[test]
    fn test_parse_unset_key() {
        let output = "-AOE_INSTANCE_ID";
        let result = parse_batch_output(output, &["test_session"]).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "test_session");
        assert_eq!(result[0].1, None);
    }

    #[test]
    fn test_parse_multiple_sessions() {
        let output = "AOE_INSTANCE_ID=abc123\n-AOE_INSTANCE_ID\nAOE_INSTANCE_ID=xyz789";
        let result = parse_batch_output(output, &["session1", "session2", "session3"]).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].1, Some("abc123".to_string()));
        assert_eq!(result[1].1, None);
        assert_eq!(result[2].1, Some("xyz789".to_string()));
    }

    #[test]
    fn test_parse_value_with_equals() {
        let output = "KEY=value=with=equals";
        let result = parse_batch_output(output, &["test_session"]).unwrap();
        assert_eq!(result[0].1, Some("value=with=equals".to_string()));
    }

    #[test]
    fn test_parse_line_count_mismatch_returns_none() {
        let output = "";
        assert!(parse_batch_output(output, &["session1", "session2"]).is_none());

        let output = "VAL1\nVAL2\nVAL3";
        assert!(parse_batch_output(output, &["session1"]).is_none());
    }

    #[test]
    fn test_parse_whitespace_handling() {
        let output = "  AOE_INSTANCE_ID=value123  \n  -AOE_INSTANCE_ID  ";
        let result = parse_batch_output(output, &["session1", "session2"]).unwrap();
        assert_eq!(result[0].1, Some("value123".to_string()));
        assert_eq!(result[1].1, None);
    }

    #[test]
    fn test_get_hidden_env_batch_empty_input() {
        let result = get_hidden_env_batch(&[], "KEY");
        assert_eq!(result.len(), 0);
    }
}
