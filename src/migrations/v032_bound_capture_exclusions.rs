//! Preserve legacy SID exclusions without inventing their namespace.

use anyhow::{Context, Result};
use serde_json::Value;
use std::{fs, path::Path};

pub fn run() -> Result<()> {
    run_in(&crate::session::get_app_dir()?)
}

fn run_in(app_dir: &Path) -> Result<()> {
    let profiles = app_dir.join("profiles");
    if profiles.exists() {
        for entry in fs::read_dir(profiles)? {
            let entry = entry?;
            if entry.path().is_dir() {
                migrate_file(&entry.path().join("sessions.json"))?;
            }
        }
    }
    migrate_file(&app_dir.join("sessions.json"))
}

fn migrate_file(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let directory = path.parent().context("sessions path has no parent")?;
    let _lock =
        crate::session::acquire_storage_flock(directory, crate::session::STORAGE_LOCK_FILENAME)?;
    let mut value: Value = match serde_json::from_str(&fs::read_to_string(path)?) {
        Ok(value) => value,
        Err(error) => {
            tracing::debug!("v032: cannot parse {}: {error}; skipping", path.display());
            return Ok(());
        }
    };
    let mut changed = false;
    if let Some(instances) = value.as_array_mut() {
        for instance in instances {
            if let Some(exclusions) = instance
                .get_mut("retroactive_capture_excludes")
                .and_then(Value::as_array_mut)
            {
                for exclusion in exclusions {
                    if let Some(sid) = exclusion.as_str() {
                        *exclusion = serde_json::to_value(
                            crate::session::ConversationBinding::unknown(sid),
                        )?;
                        changed = true;
                    }
                }
            }
        }
    }
    if changed {
        crate::session::backup_before_repair(path)?;
        crate::session::atomic_write(path, serde_json::to_string_pretty(&value)?.as_bytes())?;
        tracing::info!(
            "v032: bound legacy capture exclusions in {}",
            path.display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_exclusions_remain_conservative_after_upgrade_and_reentry() {
        let temp = tempfile::tempdir().unwrap();
        let profile = temp.path().join("profiles/default");
        fs::create_dir_all(&profile).unwrap();
        let path = profile.join("sessions.json");
        fs::write(
            &path,
            r#"[{"retroactive_capture_excludes":["legacy-sid"]}]"#,
        )
        .unwrap();
        run_in(temp.path()).unwrap();
        run_in(temp.path()).unwrap();
        let rows: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        let exclusions: Vec<crate::session::ConversationBinding> =
            serde_json::from_value(rows[0]["retroactive_capture_excludes"].clone()).unwrap();
        let source = crate::session::ExecutionBinding {
            agent: "claude".into(),
            stores: vec![temp.path().join("new-store")],
            configuration: Vec::new(),
            cwd: temp.path().into(),
            cwd_filesystem: "host".into(),
            filesystem: "host".into(),
        };
        assert!(exclusions
            .iter()
            .any(|binding| binding.excludes_capture("legacy-sid", Some(&source))));
        assert!(!exclusions
            .iter()
            .any(|binding| binding.excludes_capture("new-sid", Some(&source))));
    }

    fn restore_points_under(dir: &Path) -> Vec<std::path::PathBuf> {
        let mut found: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("sessions.json.pre-recovery-"))
            })
            .collect();
        found.sort();
        found
    }

    /// The previous release rejects the object shape and drops the row, so the
    /// only way back is the restore point taken before the rewrite.
    #[test]
    fn retyping_leaves_a_restore_point_the_previous_release_can_read() {
        // v1.16.1 typed this field as `HashSet<String>`.
        #[derive(serde::Deserialize)]
        struct Pre116 {
            #[serde(default)]
            retroactive_capture_excludes: std::collections::HashSet<String>,
        }

        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("profiles/default/sessions.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            r#"[{"retroactive_capture_excludes":["legacy-sid"]}]"#,
        )
        .unwrap();

        run_in(temp.path()).unwrap();
        run_in(temp.path()).unwrap();

        let restore_points = restore_points_under(path.parent().unwrap());
        assert_eq!(
            restore_points.len(),
            1,
            "re-entry must not add another restore point: {restore_points:?}"
        );
        let before: Vec<Pre116> =
            serde_json::from_slice(&fs::read(&restore_points[0]).unwrap()).unwrap();
        assert!(before[0]
            .retroactive_capture_excludes
            .contains("legacy-sid"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&restore_points[0])
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600, "restore point must stay owner-only");
        }
    }
}
