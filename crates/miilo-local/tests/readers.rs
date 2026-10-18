#![cfg(unix)]
use miilo_adapter_sdk::runtime::observation::Observation;
use miilo_local::adapters::{CodexTranscriptTail, TranscriptTail, apply_cursor_activity};
use serde_json::json;
fn state() -> Observation {
    Observation::new(
        "one",
        "Existing session",
        "codex_transcript",
        &json!({"id":"one","name":"Existing session"}),
    )
}
#[test]
fn codex_partial_appends_exclude_private_channels_and_preserve_public_summary() {
    let mut state = state();
    let mut tail = CodexTranscriptTail::new(Default::default());
    let row = |payload| {
        format!(
            "{}\n",
            json!({"timestamp":"2026-10-16T00:00:00Z","type":"response_item","payload":payload})
        )
    };
    let public = row(
        json!({"type":"message","role":"assistant","phase":"commentary","content":[{"text":"Checking existing work"}]}),
    );
    tail.consume(&public.as_bytes()[..public.len() - 1], &mut state);
    assert_eq!(state.revision, 0);
    tail.consume(b"\n", &mut state);
    assert_eq!(state.revision, 1);
    for payload in [
        json!({"type":"reasoning","summary":"PRIVATE"}),
        json!({"type":"message","role":"user","content":[{"text":"PRIVATE"}]}),
        json!({"type":"message","role":"assistant","phase":"analysis","content":[{"text":"PRIVATE"}]}),
        json!({"type":"custom_tool_call","name":"test","input":"PRIVATE"}),
    ] {
        tail.consume(row(payload).as_bytes(), &mut state);
    }
    assert_eq!(state.revision, 2);
    let snapshot = state.snapshot().unwrap();
    assert_eq!(snapshot.work_items[0].summary, "Checking existing work");
    assert!(
        !serde_json::to_string(&snapshot)
            .unwrap()
            .contains("PRIVATE")
    );
    assert!(snapshot.work_items[0].actions.is_empty());
}
#[test]
fn replacement_truncation_and_symlink_targets_require_reconnection() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().canonicalize().unwrap().join("selected.jsonl");
    std::fs::write(&path, "{}\n").unwrap();
    let mut tail = TranscriptTail::new(path.clone(), apply_cursor_activity);
    let mut state = state();
    tail.read(&mut state).unwrap();
    let replacement = path.with_extension("new");
    std::fs::write(&replacement, "{}\n").unwrap();
    std::fs::rename(replacement, &path).unwrap();
    assert!(tail.read(&mut state).is_err());
    let mut tail = TranscriptTail::new(path.clone(), apply_cursor_activity);
    tail.read(&mut state).unwrap();
    std::fs::write(&path, "").unwrap();
    assert!(tail.read(&mut state).is_err());
    let link = path.with_extension("link");
    std::os::unix::fs::symlink(&path, &link).unwrap();
    let mut tail = TranscriptTail::new(link, apply_cursor_activity);
    assert!(tail.read(&mut state).is_err());
}

#[test]
fn bootstrap_preserves_codex_time_and_dates_cursor_history_by_file() {
    use std::{
        fs::{self, File, FileTimes},
        time::{Duration, UNIX_EPOCH},
    };
    let dir = tempfile::tempdir().unwrap();
    let at = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    for source in ["codex_transcript", "cursor_transcript"] {
        let path = dir.path().join(source);
        let row = if source == "codex_transcript" {
            json!({"timestamp":"2020-01-01T00:00:00Z","type":"event_msg","payload":{"type":"task_complete"}})
        } else {
            json!({"type":"turn_ended","status":"completed"})
        };
        fs::write(&path, format!("{row}\n")).unwrap();
        File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_times(FileTimes::new().set_modified(at))
            .unwrap();
        let mut observation = state();
        observation.source = source.into();
        let path = path.canonicalize().unwrap();
        if source == "codex_transcript" {
            CodexTranscriptTail::new(path)
                .read(&mut observation)
                .unwrap();
            assert!(observation.observed_at.starts_with("2020-01-01"));
        } else {
            TranscriptTail::new(path, apply_cursor_activity)
                .read(&mut observation)
                .unwrap();
            let expected = chrono::DateTime::<chrono::Utc>::from(at);
            assert_eq!(observation.observed_at, expected.to_rfc3339());
        }
    }
}
