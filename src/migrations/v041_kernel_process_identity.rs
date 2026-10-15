//! Mark pre-existing runtime identities as unknown without sampling their PIDs.

use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;

pub fn run() -> Result<()> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(crate::process::worker_registry::workers_dir()?)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_file()
            && path.extension().is_some_and(|extension| {
                extension == "json" || extension == "orphan-stop" || extension == "purge-fence"
            })
        {
            paths.push(path);
        }
    }
    paths.sort();
    for path in paths {
        let id = path
            .file_stem()
            .and_then(|name| name.to_str())
            .context("invalid runtime identity filename")?;
        crate::session::validate_instance_id(id)?;
        crate::process::worker_registry::with_registry_lock(id, || {
            let bytes = match fs::read(&path) {
                Ok(bytes) => bytes,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                Err(error) => return Err(error.into()),
            };
            let Ok(mut value) = serde_json::from_slice::<Value>(&bytes) else {
                return Ok(());
            };
            let Some(object) = value.as_object_mut() else {
                return Ok(());
            };
            let key = if path
                .extension()
                .is_some_and(|extension| extension == "purge-fence")
            {
                "runner_process_identity"
            } else {
                "process_identity"
            };
            if object.contains_key(key) {
                return Ok(());
            }
            object.insert(key.into(), Value::Null);
            crate::session::atomic_write_verified(&path, &serde_json::to_vec(&value)?)?;
            crate::session::sync_parent_directory(&path)?;
            tracing::info!(path = %path.display(), "v041: marked legacy process identity unknown");
            Ok(())
        })?;
    }
    Ok(())
}
