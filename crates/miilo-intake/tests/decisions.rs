use chrono::{Duration, TimeZone, Utc};
use miilo_adapter_sdk::intake::{DecisionState, action};
use miilo_adapter_sdk::*;
use miilo_intake::{
    application as app,
    domain::{State, Submission},
    storage::Files,
};
fn now() -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(1800000000, 0).unwrap()
}
fn submission(revision: &str) -> Submission {
    Submission {
        id: "task-1".into(),
        title: "Prepare release".into(),
        summary: "Run registered checks".into(),
        revision: revision.into(),
        workflow_id: "release".into(),
    }
}
fn request(state: &State, action_id: &str) -> ActionInvocation {
    ActionInvocation {
        protocol_version: "0.1".into(),
        request_id: "req-1".into(),
        provider_id: app::PROVIDER.into(),
        action_id: action_id.into(),
        subject: SubjectReference {
            kind: SubjectKind::Task,
            id: "task-1".into(),
        },
        expected_revision: state.entries["task-1"].revision(),
        confirmation: Confirmation {
            method: ConfirmationMethod::Manual,
            confirmed_at: now(),
        },
        input: serde_json::Map::new(),
        idempotency_key: Some("key-1".into()),
    }
}
#[test]
fn import_never_executes_and_duplicate_observation_does_not_repeat() {
    let mut s = State::default();
    assert!(app::import(&mut s, submission("r1"), now()).unwrap());
    assert!(!app::import(&mut s, submission("r1"), now()).unwrap());
    assert_eq!(s.sequence, 1);
    assert_eq!(s.entries["task-1"].decision, DecisionState::Pending);
    let mut changed = submission("r1");
    changed.title = "Changed".into();
    assert!(app::import(&mut s, changed, now()).is_err());
}
#[test]
fn uncertain_retries_return_the_original_receipt_without_duplicate_decisions() {
    let mut s = State::default();
    app::import(&mut s, submission("r1"), now()).unwrap();
    let req = request(&s, action::ACCEPT);
    let first = app::decide(&mut s, req.clone(), now()).unwrap();
    let sequence = s.sequence;
    let replay = app::decide(&mut s, req.clone(), now() + Duration::days(1)).unwrap();
    assert_eq!(first.recorded_at, replay.recorded_at);
    assert_eq!(s.sequence, sequence);
    let mut changed = req;
    changed.action_id = action::DISMISS.into();
    assert!(app::decide(&mut s, changed, now()).is_err());
    assert!(app::import(&mut s, submission("r2"), now()).is_err());
}
#[test]
fn stale_and_unconfirmed_decisions_fail_closed() {
    let mut s = State::default();
    app::import(&mut s, submission("r1"), now()).unwrap();
    let req = request(&s, action::ACCEPT);
    app::import(&mut s, submission("r2"), now()).unwrap();
    assert!(app::decide(&mut s, req, now()).is_err());
    let mut req = request(&s, action::ACCEPT);
    req.confirmation.method = ConfirmationMethod::None;
    assert!(app::decide(&mut s, req, now()).is_err());
    assert_eq!(s.entries["task-1"].decision, DecisionState::Pending);
}
#[test]
fn deferred_work_returns_only_when_due() {
    let mut s = State::default();
    app::import(&mut s, submission("r1"), now()).unwrap();
    let mut req = request(&s, action::DEFER);
    req.input.insert(
        "until".into(),
        (now() + Duration::hours(4)).to_rfc3339().into(),
    );
    app::decide(&mut s, req, now()).unwrap();
    app::wake(&mut s, now() + Duration::hours(3));
    assert_eq!(s.entries["task-1"].decision, DecisionState::Deferred);
    app::wake(&mut s, now() + Duration::hours(4));
    assert_eq!(s.entries["task-1"].decision, DecisionState::Pending);
}
#[test]
fn storage_survives_restart_and_rolls_back_failed_transactions() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("intake");
    Files::new(&path)
        .unwrap()
        .transaction(|s| app::import(s, submission("r1"), now()))
        .unwrap();
    let result: Result<(), String> = Files::new(&path).unwrap().transaction(|s| {
        s.entries.clear();
        Err("simulated failure".into())
    });
    assert!(result.is_err());
    Files::new(&path)
        .unwrap()
        .transaction(|s| {
            assert!(s.entries.contains_key("task-1"));
            let snap = miilo_intake::projection::snapshot(s, now());
            assert_eq!(snap.work_items.len(), 1);
            assert!(snap.providers[0].capabilities.features["intake.v1"]);
            assert!(!snap.providers[0].capabilities.features["execution"]);
            Ok(())
        })
        .unwrap();
}
#[cfg(unix)]
#[test]
fn storage_rejects_insecure_and_symlink_roots() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("target");
    std::fs::create_dir(&target).unwrap();
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(Files::new(&target).is_err());
    let link = root.path().join("link");
    symlink(&target, &link).unwrap();
    assert!(Files::new(&link).is_err());
}

#[test]
fn replacing_an_inbox_cannot_reuse_the_previous_stream_cursor() {
    let first = State {
        instance_id: "instance-one".into(),
        ..State::default()
    };
    let mut second = first.clone();
    second.instance_id = "instance-two".into();
    let before = miilo_intake::projection::snapshot(&first, now());
    let after = miilo_intake::projection::snapshot(&second, now());
    assert_ne!(before.cursor, after.cursor);
    assert_ne!(before.snapshot_id, after.snapshot_id);
}
