//! `agent-of-empires remove` command implementation

use anyhow::Result;
use chrono::Utc;
use clap::Args;

use crate::session::{ClaimOp, Instance, Storage};

#[derive(Args)]
pub struct RemoveArgs {
    /// Session ID or title to remove
    identifier: String,

    /// Delete worktree directory (default: keep worktree)
    #[arg(long = "delete-worktree")]
    delete_worktree: bool,

    /// Delete git branch after worktree removal (default: per config)
    #[arg(long = "delete-branch")]
    delete_branch: bool,

    /// Force worktree removal even with untracked/modified files
    #[arg(long)]
    force: bool,

    /// Keep container instead of deleting it (default: delete per config)
    #[arg(long = "keep-container")]
    keep_container: bool,

    /// For scratch sessions, keep the scratch directory on disk instead of
    /// removing it. The session record is still deleted; the kept path is
    /// logged so you can find the files later. No effect on non-scratch
    /// sessions.
    #[arg(long = "keep-scratch")]
    keep_scratch: bool,

    /// Permanently delete instead of moving to trash. By default `rm` moves
    /// the session to the trash (when `session.delete_to_trash` is enabled,
    /// the default) so it can be restored; `--purge` forces the irreversible
    /// teardown (worktree/branch/container cleanup per the other flags, plus
    /// transcript removal).
    #[arg(long)]
    purge: bool,
}

fn needs_worktree_cleanup(inst: &Instance, args: &RemoveArgs) -> bool {
    args.delete_worktree && inst.has_managed_worktree_or_workspace()
}

/// Whether a `--purge` should delete the session's git branch(es). #2525: gated
/// on the shape-agnostic predicate so it fires for multi-repo workspace sessions
/// (only `workspace_info`, no `worktree_info`) as well as worktree sessions;
/// `perform_deletion` keys both worktree and workspace-repo branch cleanup off
/// this flag. The old `worktree_info`-only gate skipped workspace branches.
fn should_delete_branch(
    inst: &Instance,
    args: &RemoveArgs,
    delete_worktree: bool,
    delete_branch_on_cleanup: bool,
) -> bool {
    inst.has_managed_worktree_or_workspace()
        && (args.delete_branch || (delete_worktree && delete_branch_on_cleanup))
}

#[tracing::instrument(target = "cli.session", skip_all, fields(profile = %profile))]
pub async fn run(profile: &str, args: RemoveArgs) -> Result<()> {
    let storage = Storage::new_unwatched(profile)?;

    // Phase 1 (unlocked): identify the target and run the slow deletion
    // side effects (worktree removal, branch deletion, container teardown,
    // detach hooks). The flock would otherwise be held for the entire
    // deletion sequence, blocking peer mutators on the same profile.
    let (instances, _groups) = storage.load_with_groups()?;

    let inst = super::resolve_session(&args.identifier, &instances)
        .map_err(|e| anyhow::anyhow!("{} in profile '{}'", e, storage.profile()))?
        .clone();
    let removed_id = inst.id.clone();
    let removed_title = inst.title.clone();

    let config = crate::session::repo_config::resolve_config_with_repo_or_warn(
        profile,
        std::path::Path::new(&inst.project_path),
    );

    // Trash-first: unless --purge is given (or delete_to_trash is disabled),
    // stop the live session and mark it trashed, keeping every durable
    // artifact so it can be restored. Mirrors the archive CLI's tmux
    // teardown. See #2489.
    if config.session.delete_to_trash && !args.purge {
        if let Err(e) = inst.kill() {
            eprintln!("Warning: failed to kill agent tmux session: {}", e);
        }
        inst.kill_ancillary_tmux_sessions();

        let landed = storage.update(|all_instances, _groups| {
            if let Some(stored) = all_instances.iter_mut().find(|i| i.id == removed_id) {
                stored.trash();
                Ok(true)
            } else {
                Ok(false)
            }
        })?;
        if !landed {
            anyhow::bail!(
                "Session {} was removed by another process before it could be trashed",
                removed_title
            );
        }

        // The session is durably trashed; now move its worktree out of the
        // active dir into the holding area and persist the repointed
        // project_path. A failure here never blocks the trash; the worktree
        // just stays in place and a later reconcile pass can relocate it.
        let mut inst = inst;
        inst.trash();
        match crate::session::trash::relocate_worktree_to_trash(&mut inst) {
            crate::session::trash::RelocateOutcome::Relocated { .. } => {
                let new_path = inst.project_path.clone();
                let pre = inst.pre_trash_project_path.clone();
                let _ = storage.update(|all_instances, _groups| {
                    if let Some(stored) = all_instances.iter_mut().find(|i| i.id == removed_id) {
                        stored.project_path = new_path.clone();
                        stored.pre_trash_project_path = pre.clone();
                    }
                    Ok(())
                });
            }
            crate::session::trash::RelocateOutcome::Failed { reason } => {
                eprintln!("  Note: left worktree in place ({reason}).");
            }
            crate::session::trash::RelocateOutcome::Skipped => {}
        }

        println!(
            "  Moved session to trash: {} (from profile '{}')",
            removed_title,
            storage.profile()
        );
        println!(
            "  Restore with `aoe session restore {removed_id}`, or delete permanently with `aoe rm --purge {removed_id}`."
        );
        return Ok(());
    }

    let delete_worktree = needs_worktree_cleanup(&inst, &args);
    let delete_branch = should_delete_branch(
        &inst,
        &args,
        delete_worktree,
        config.worktree.delete_branch_on_cleanup,
    );
    let delete_sandbox = inst.sandbox_info.as_ref().is_some_and(|s| s.enabled)
        && !args.keep_container
        && config.sandbox.auto_cleanup;

    // Phase 1 (locked, quick): claim the purge before the unlocked teardown so
    // a concurrent restore from another process (CLI / serve daemon / TUI)
    // cannot bring the session back after its artifacts are already gone. The
    // durable on-disk claim is the only cross-process serialization point; the
    // server's in-memory instance lock is invisible here. See #2541.
    let was_trashed = inst.is_trashed();
    let claim = storage.update(|all_instances, _groups| {
        Ok(decide_purge_claim(
            all_instances,
            &removed_id,
            was_trashed,
            Utc::now(),
        ))
    })?;
    match claim {
        PurgeClaim::AlreadyGone => {
            anyhow::bail!(
                "Session {} was already removed by another process",
                removed_title
            );
        }
        PurgeClaim::Restored => {
            anyhow::bail!(
                "Session {} was restored before its purge could start, so it was not purged",
                removed_title
            );
        }
        PurgeClaim::RestoreInProgress => {
            anyhow::bail!(
                "Session {} is being restored by another process, so it was not purged",
                removed_title
            );
        }
        PurgeClaim::Claimed => {}
    }

    let result =
        crate::session::deletion::perform_deletion(&crate::session::deletion::DeletionRequest {
            session_id: inst.id.clone(),
            instance: inst.clone(),
            delete_worktree,
            delete_branch,
            delete_sandbox,
            force_delete: args.force,
            detach_hooks: false,
            keep_scratch: args.keep_scratch,
        });

    for msg in &result.messages {
        println!("  {}", msg);
    }
    for err in &result.errors {
        eprintln!("Warning: {}", err);
    }

    // A failed teardown (worktree/branch/container cleanup) must keep the
    // session row so the leftover artifacts can be retried, not abandoned by
    // dropping the record below. Mirrors `empty-trash`, which only purges rows
    // whose teardown succeeded. See #2489.
    if !result.success {
        release_purge_claim(&storage, &removed_id);
        anyhow::bail!(
            "Session teardown failed, so the session record was kept (retry, or fix the \
             underlying cause and remove it again)"
        );
    }

    // Permanent purge of a structured-view session must also drop its durable
    // transcript so it does not orphan in the event store; the CLI opens the
    // store directly since it has no live worker. Only after a successful
    // teardown so a failed purge stays restorable. If the transcript can't be
    // dropped, keep the session row (skip the removal below) rather than
    // orphan the transcript. See #2489.
    if let Err(e) = super::purge_acp_transcript(&inst) {
        release_purge_claim(&storage, &removed_id);
        anyhow::bail!(
            "Session teardown succeeded but its transcript could not be purged, so the session \
             record was kept (retry, or remove it once the event store is reachable): {e}"
        );
    }

    if !delete_worktree {
        if inst
            .worktree_info
            .as_ref()
            .is_some_and(|wt| wt.managed_by_aoe)
        {
            println!(
                "Worktree preserved at: {} (use --delete-worktree to remove)",
                inst.project_path
            );
        } else if let Some(ws_info) = &inst.workspace_info {
            if ws_info.cleanup_on_delete {
                println!(
                    "Workspace preserved at: {} (use --delete-worktree to remove)",
                    ws_info.workspace_dir
                );
            }
        }
    }
    if let Some(sandbox) = &inst.sandbox_info {
        if sandbox.enabled {
            if args.keep_container {
                println!("Container preserved: {}", sandbox.container_name);
            } else if !config.sandbox.auto_cleanup {
                println!(
                    "Container preserved: {} (auto_cleanup disabled in config)",
                    sandbox.container_name
                );
            }
        }
    }

    // Phase 2 (locked): drop the entry by id from the latest disk state.
    // #2534: revalidate under the lock. The destructive teardown above ran on
    // an unlocked snapshot; if this purge targeted a trashed session and a
    // concurrent restore untrashed it in the meantime, the restore must win, so
    // keep the row instead of deleting a session the user just brought back.
    // A no-op when a peer already removed it; that is the correct semantics.
    let outcome = storage.update(|all_instances, _groups| {
        Ok(finalize_purge_removal(
            all_instances,
            &removed_id,
            was_trashed,
        ))
    })?;

    if matches!(outcome, RowRemoval::KeptRestored) {
        eprintln!(
            "Warning: session {} was restored while its purge was running; kept the \
             restored record, but its worktree, branch, container, or transcript may \
             already have been removed by the purge. Inspect and repair it.",
            removed_title
        );
        return Ok(());
    }

    // Keep the project in the new-session wizard's Recent tab after its last
    // session is gone (#2141). Best-effort; a failure must not fail the remove.
    if matches!(outcome, RowRemoval::Removed) {
        if let Some(entry) = crate::session::recent_project_entry_for(&inst) {
            if let Err(e) = crate::session::record_recent_project(entry) {
                tracing::warn!(target: "session.delete",
                    "recording recent project after remove failed: {e}");
            }
        }
    }

    println!(
        "  Removed session: {} (from profile '{}')",
        removed_title,
        storage.profile()
    );

    Ok(())
}

/// Outcome of the final locked row-removal step in a `--purge`. See #2534.
#[derive(Debug, PartialEq)]
enum RowRemoval {
    /// The row was dropped from storage.
    Removed,
    /// A concurrent restore won; the (now untrashed) row was kept.
    KeptRestored,
    /// A peer already removed the row before this purge reached the lock.
    AlreadyGone,
}

/// Outcome of acquiring the purge claim before the unlocked teardown. See #2541.
#[derive(Debug, PartialEq)]
enum PurgeClaim {
    /// The claim was won (free, expired, or already ours); teardown may proceed.
    Claimed,
    /// A concurrent restore untrashed the targeted row before the claim landed.
    Restored,
    /// A peer holds a fresh Restore claim on the row.
    RestoreInProgress,
    /// A peer already removed the row before this purge reached the lock.
    AlreadyGone,
}

/// Decide whether a `--purge` may claim and tear down a row, run under the flock
/// in `run`. Splits into: gone (peer removed it), restored (H1: the targeted
/// trashed row was un-trashed between the snapshot and this claim, so a live
/// session is never torn down), refused (a peer holds a fresh Restore claim),
/// or claimed. On `Claimed` the row's Purge claim is set as a side effect. See
/// #2541.
fn decide_purge_claim(
    all: &mut [Instance],
    removed_id: &str,
    was_trashed: bool,
    now: chrono::DateTime<chrono::Utc>,
) -> PurgeClaim {
    match all.iter_mut().find(|i| i.id == removed_id) {
        None => PurgeClaim::AlreadyGone,
        Some(stored)
            if super::purge_restored_row_must_be_kept(was_trashed, stored.is_trashed()) =>
        {
            PurgeClaim::Restored
        }
        Some(stored) => match stored.try_claim(ClaimOp::Purge, Instance::op_claim_ttl(), now) {
            Err(ClaimOp::Restore) => PurgeClaim::RestoreInProgress,
            _ => PurgeClaim::Claimed,
        },
    }
}

/// The final locked row removal for a `--purge`, run under the flock in `run`.
/// Applies the #2534 restore-race recheck: a row a peer restored mid-purge is
/// kept and its Purge claim released (ownership-guarded so a peer's fresh
/// Restore claim is never cleared); otherwise the row is dropped. See #2541.
fn finalize_purge_removal(
    all: &mut Vec<Instance>,
    removed_id: &str,
    was_trashed: bool,
) -> RowRemoval {
    match all.iter().position(|i| i.id == removed_id) {
        None => RowRemoval::AlreadyGone,
        Some(idx) if super::purge_restored_row_must_be_kept(was_trashed, all[idx].is_trashed()) => {
            all[idx].clear_op_claim_if_owned(ClaimOp::Purge);
            RowRemoval::KeptRestored
        }
        Some(idx) => {
            all.remove(idx);
            RowRemoval::Removed
        }
    }
}

/// Release a purge claim on a kept row (teardown or transcript failed), owned
/// guarded so a peer's fresh Restore claim is never cleared. Best-effort: a
/// stranded claim self-heals via the TTL. See #2541.
fn release_purge_claim(storage: &Storage, removed_id: &str) {
    let _ = storage.update(|all_instances, _groups| {
        if let Some(stored) = all_instances.iter_mut().find(|i| i.id == removed_id) {
            stored.clear_op_claim_if_owned(ClaimOp::Purge);
        }
        Ok(())
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{WorkspaceInfo, WorkspaceRepo};

    fn args(delete_worktree: bool) -> RemoveArgs {
        RemoveArgs {
            identifier: "x".to_string(),
            delete_worktree,
            delete_branch: false,
            force: false,
            keep_container: false,
            keep_scratch: false,
            purge: false,
        }
    }

    // Regression for #2363: a multi-repo workspace session has no
    // `worktree_info`, so the old worktree_info-only check returned false and
    // `--delete-worktree` silently left the workspace dir on disk.
    #[test]
    fn needs_worktree_cleanup_true_for_workspace_session() {
        let mut inst = Instance::new("WS", "/tmp/ws/repo-a");
        inst.workspace_info = Some(WorkspaceInfo {
            branch: "feature/abc".to_string(),
            workspace_dir: "/tmp/ws".to_string(),
            repos: vec![WorkspaceRepo {
                name: "repo-a".to_string(),
                source_path: "/tmp/src/repo-a".to_string(),
                branch: "feature/abc".to_string(),
                worktree_path: "/tmp/ws/repo-a".to_string(),
                main_repo_path: "/tmp/src/repo-a".to_string(),
                managed_by_aoe: true,
            }],
            created_at: Utc::now(),
            cleanup_on_delete: true,
        });

        assert!(needs_worktree_cleanup(&inst, &args(true)));
        assert!(!needs_worktree_cleanup(&inst, &args(false)));
    }

    // Regression for #2525: a multi-repo workspace session has no
    // `worktree_info`, so the old `worktree_info`-only gate returned false and
    // `--purge --delete-worktree --delete-branch` left the AoE-created branches
    // behind. The shape-agnostic gate must enable branch deletion for it.
    #[test]
    fn should_delete_branch_true_for_workspace_session() {
        let mut inst = Instance::new("WS", "/tmp/ws/repo-a");
        inst.workspace_info = Some(WorkspaceInfo {
            branch: "feature/abc".to_string(),
            workspace_dir: "/tmp/ws".to_string(),
            repos: vec![WorkspaceRepo {
                name: "repo-a".to_string(),
                source_path: "/tmp/src/repo-a".to_string(),
                branch: "feature/abc".to_string(),
                worktree_path: "/tmp/ws/repo-a".to_string(),
                main_repo_path: "/tmp/src/repo-a".to_string(),
                managed_by_aoe: true,
            }],
            created_at: Utc::now(),
            cleanup_on_delete: true,
        });

        let mut with_flag = args(true);
        with_flag.delete_branch = true;
        // Explicit --delete-branch fires regardless of the config default.
        assert!(should_delete_branch(&inst, &with_flag, true, false));
        // And via the config default when deleting the worktree.
        assert!(should_delete_branch(&inst, &args(true), true, true));
        // Not without any managed worktree/workspace.
        let plain = Instance::new("plain", "/tmp/plain");
        assert!(!should_delete_branch(&plain, &with_flag, true, true));
    }

    fn trashed(title: &str) -> Instance {
        let mut inst = Instance::new(title, "/tmp/x");
        inst.trash();
        inst
    }

    // H1: the targeted-trashed row was un-trashed between the purge snapshot and
    // the claim, so the purge must not tear down a session that came back to
    // life. See #2541.
    #[test]
    fn purge_bails_when_target_untrashed_between_snapshot_and_claim() {
        let mut live = trashed("s");
        live.untrash(); // restored between snapshot and claim
        let id = live.id.clone();
        let mut all = vec![live];
        assert_eq!(
            decide_purge_claim(&mut all, &id, true, Utc::now()),
            PurgeClaim::Restored
        );
        assert_eq!(all[0].op_claim, None, "no claim is set on a restored row");
    }

    // H1: a direct `rm --purge` of a genuinely live session (was_trashed=false)
    // has no restore to lose to, so it proceeds and claims. See #2541.
    #[test]
    fn rm_purge_of_live_session_still_proceeds() {
        let live = Instance::new("s", "/tmp/x");
        let id = live.id.clone();
        let mut all = vec![live];
        assert_eq!(
            decide_purge_claim(&mut all, &id, false, Utc::now()),
            PurgeClaim::Claimed
        );
        assert_eq!(
            all[0].op_claim.as_ref().map(|c| c.op.clone()),
            Some(ClaimOp::Purge)
        );
    }

    // A fresh peer Restore claim refuses the purge (symmetry).
    #[test]
    fn purge_claim_refused_when_restore_holds() {
        let mut row = trashed("s");
        row.try_claim(ClaimOp::Restore, Instance::op_claim_ttl(), Utc::now())
            .unwrap();
        let id = row.id.clone();
        let mut all = vec![row];
        assert_eq!(
            decide_purge_claim(&mut all, &id, true, Utc::now()),
            PurgeClaim::RestoreInProgress
        );
    }

    // The final removal keeps a row a peer restored mid-purge and releases the
    // owned Purge claim (anti-wedge regression). See #2534/#2541.
    #[test]
    fn purge_clears_claim_on_kept_restored_row() {
        let mut row = trashed("s");
        row.try_claim(ClaimOp::Purge, Instance::op_claim_ttl(), Utc::now())
            .unwrap();
        row.untrash(); // a peer restored it mid-purge
        let id = row.id.clone();
        let mut all = vec![row];
        assert_eq!(
            finalize_purge_removal(&mut all, &id, true),
            RowRemoval::KeptRestored
        );
        assert_eq!(all.len(), 1, "the restored row is kept");
        assert_eq!(all[0].op_claim, None, "our purge claim is released");
    }

    // A still-trashed row is removed by the final commit; a peer's fresh Restore
    // claim (stale-override) is never cleared by the ownership guard.
    #[test]
    fn purge_removes_still_trashed_row() {
        let mut row = trashed("s");
        row.try_claim(ClaimOp::Purge, Instance::op_claim_ttl(), Utc::now())
            .unwrap();
        let id = row.id.clone();
        let mut all = vec![row];
        assert_eq!(
            finalize_purge_removal(&mut all, &id, true),
            RowRemoval::Removed
        );
        assert!(all.is_empty());
    }
}
