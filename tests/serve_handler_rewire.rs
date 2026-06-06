//! Handler-level locks for the disk-watch rewire wiring. The earlier
//! lower-layer tests in `serve_dynamic_profile_rewire.rs` exercise
//! `set_profile_disk_watch` directly; these tests prove the HTTP handlers
//! (`create_profile`, `delete_profile`, `rename_profile`) actually invoke
//! that helper. Reverting the rewire calls in `src/server/api/system.rs`
//! must fail at least one of these tests.

#![cfg(feature = "serve")]

use std::path::Path;
use std::sync::Arc;

use agent_of_empires::file_watch::FileWatchService;
use agent_of_empires::server::test_support::{
    build_test_app_state, create_profile, delete_profile, has_disk_watch_handle, rename_profile,
    CreateProfileBody, RenameProfileBody,
};
use axum::extract::{Path as AxumPath, State};
use axum::Json;
use serial_test::serial;

fn isolate_home(temp: &Path) {
    // SAFETY: env mutation; #[serial] guards cross-test races.
    unsafe { std::env::set_var("HOME", temp) };
    #[cfg(target_os = "linux")]
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", temp.join(".config"))
    };
}

fn live_state() -> Arc<agent_of_empires::server::AppState> {
    let state = build_test_app_state(Vec::new());
    let live = FileWatchService::new().expect("live svc");
    let mut state_mut = Arc::try_unwrap(state).map_err(|_| ()).expect("unique");
    state_mut.file_watch = live;
    Arc::new(state_mut)
}

#[tokio::test]
#[serial]
async fn create_profile_handler_installs_disk_watch_subscription() {
    let temp = tempfile::tempdir().unwrap();
    isolate_home(temp.path());

    let state = live_state();
    let body = Ok(Json(CreateProfileBody {
        name: "handler-add".into(),
    }));
    let _ = create_profile(State(state.clone()), body).await;

    assert!(
        has_disk_watch_handle(&state, "handler-add").await,
        "create_profile handler must invoke set_profile_disk_watch(_, true) on success"
    );
    assert_eq!(
        state.file_watch.subscriber_count_for_test(),
        1,
        "exactly one live subscription must remain after a single create"
    );
}

#[tokio::test]
#[serial]
async fn delete_profile_handler_removes_disk_watch_subscription() {
    let temp = tempfile::tempdir().unwrap();
    isolate_home(temp.path());

    let state = live_state();
    // session::delete_profile enforces "at least one profile must remain";
    // create two so the deletion target can land cleanly.
    let _ = create_profile(
        State(state.clone()),
        Ok(Json(CreateProfileBody {
            name: "handler-del-keep".into(),
        })),
    )
    .await;
    let _ = create_profile(
        State(state.clone()),
        Ok(Json(CreateProfileBody {
            name: "handler-del".into(),
        })),
    )
    .await;
    assert!(
        has_disk_watch_handle(&state, "handler-del").await,
        "precondition: subscription installed by create"
    );

    let _ = delete_profile(State(state.clone()), AxumPath("handler-del".to_string())).await;

    assert!(
        !has_disk_watch_handle(&state, "handler-del").await,
        "delete_profile handler must invoke set_profile_disk_watch(_, false) on success"
    );
    assert!(
        has_disk_watch_handle(&state, "handler-del-keep").await,
        "delete must not affect unrelated profile subscriptions"
    );
    assert_eq!(
        state.file_watch.subscriber_count_for_test(),
        1,
        "exactly the surviving profile's subscription must remain"
    );
}

#[tokio::test]
#[serial]
async fn rename_profile_handler_swaps_disk_watch_subscription() {
    let temp = tempfile::tempdir().unwrap();
    isolate_home(temp.path());

    let state = live_state();
    let _ = create_profile(
        State(state.clone()),
        Ok(Json(CreateProfileBody {
            name: "handler-rename-old".into(),
        })),
    )
    .await;
    assert!(
        has_disk_watch_handle(&state, "handler-rename-old").await,
        "precondition: old name subscribed"
    );

    let _ = rename_profile(
        State(state.clone()),
        AxumPath("handler-rename-old".to_string()),
        Ok(Json(RenameProfileBody {
            new_name: "handler-rename-new".into(),
        })),
    )
    .await;

    assert!(
        !has_disk_watch_handle(&state, "handler-rename-old").await,
        "rename_profile handler must drop the old name's subscription"
    );
    assert!(
        has_disk_watch_handle(&state, "handler-rename-new").await,
        "rename_profile handler must install the new name's subscription"
    );
    assert_eq!(
        state.file_watch.subscriber_count_for_test(),
        1,
        "exactly one live subscription must remain after the rename"
    );
}
