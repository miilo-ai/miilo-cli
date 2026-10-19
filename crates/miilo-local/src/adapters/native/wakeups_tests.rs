use super::{Wakeups, invalidates};
use notify::{
    Event, EventKind,
    event::{AccessKind, AccessMode, CreateKind, Flag, ModifyKind, RemoveKind},
};
use serde_json::json;
use std::{fs, time::Duration};
use tokio::time::timeout;

#[test]
fn mutation_and_loss_policy_preserves_authoritative_repairs() {
    let path = std::path::PathBuf::from("/selected/session.jsonl");
    let targets = [path.clone()].into_iter().collect();
    for kind in [
        EventKind::Create(CreateKind::File),
        EventKind::Modify(ModifyKind::Any),
        EventKind::Remove(RemoveKind::File),
        EventKind::Any,
        EventKind::Other,
    ] {
        assert!(invalidates(
            &Event::new(kind).add_path(path.clone()),
            &targets
        ));
    }
    for kind in [
        AccessKind::Open(AccessMode::Read),
        AccessKind::Read,
        AccessKind::Close(AccessMode::Read),
        AccessKind::Close(AccessMode::Write),
    ] {
        assert!(!invalidates(
            &Event::new(EventKind::Access(kind)).add_path(path.clone()),
            &targets
        ));
    }
    assert!(invalidates(&Event::new(EventKind::Any), &targets));
    assert!(invalidates(
        &Event::new(EventKind::Any).set_flag(Flag::Rescan),
        &targets
    ));
    assert!(invalidates(
        &Event::new(EventKind::Remove(RemoveKind::Folder)).add_path("/selected".into()),
        &targets
    ));
}

#[tokio::test]
async fn reading_selected_transcript_does_not_wake_observer() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("session.jsonl");
    fs::write(&path, "initial\n").unwrap();
    let mut wakeups = Wakeups::new(&json!({"sessions":[{"path":path}]})).unwrap();
    fs::read(&path).unwrap();
    assert!(
        timeout(Duration::from_millis(300), wakeups.wait())
            .await
            .is_err()
    );
    fs::write(&path, "changed\n").unwrap();
    timeout(Duration::from_secs(2), wakeups.wait())
        .await
        .unwrap();
    assert!(
        timeout(Duration::from_millis(200), wakeups.wait())
            .await
            .is_err()
    );
}

#[tokio::test]
// kqueue may report an existing selected DB as created on sibling creation.
#[cfg(target_os = "linux")]
async fn database_directory_only_wakes_for_selected_database_changes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("sessions.db");
    fs::write(&path, "initial").unwrap();
    let mut wakeups = Wakeups::new(&json!({"sessions":[{"path":path}]})).unwrap();
    fs::write(directory.path().join("unselected.db"), "unselected").unwrap();
    fs::write(
        directory.path().join("sessions.db-shm"),
        "reader bookkeeping",
    )
    .unwrap();
    assert!(
        timeout(Duration::from_millis(300), wakeups.wait())
            .await
            .is_err()
    );
    fs::write(
        directory.path().join("sessions.db-wal"),
        "committed activity",
    )
    .unwrap();
    timeout(Duration::from_secs(2), wakeups.wait())
        .await
        .unwrap();
}
