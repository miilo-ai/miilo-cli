#![cfg(unix)]
use miilo_adapter_sdk::runtime::observation::Observation;
use miilo_local::adapters::{ClaudeTranscriptTail, claude_discovery};
use serde_json::{Value, json};
use std::{fs, io::Write};
const ID: &str = "12345678-1234-1234-1234-123456789abc";
fn row(content: Value) -> Value {
    json!({"type":"assistant","sessionId":ID,"isSidechain":false,"timestamp":"2026-10-18T00:00:00Z","message":{"role":"assistant","content":content}})
}
fn append(path: &std::path::Path, value: &Value) {
    writeln!(
        fs::OpenOptions::new().append(true).open(path).unwrap(),
        "{value}"
    )
    .unwrap();
}
#[test]
fn discovers_metadata_and_tails_selected_public_activity_without_writes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let project = root.join("project");
    fs::create_dir(&project).unwrap();
    let path = project.join(format!("{ID}.jsonl"));
    fs::write(&path,format!("{}\n",json!({"type":"user","sessionId":ID,"cwd":"/fixture/project","timestamp":"2026-10-18T00:00:00Z","message":{"content":"PRIVATE_PROMPT"}}))).unwrap();
    append(
        &path,
        &row(
            json!([{"type":"thinking","thinking":"PRIVATE_REASONING"},{"type":"text","text":"Public summary"}]),
        ),
    );
    let discovered = claude_discovery::sessions(&root).unwrap();
    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0]["kind"], "claude");
    assert!(
        !serde_json::to_string(&discovered)
            .unwrap()
            .contains("PRIVATE")
    );
    let mut state = Observation::new(
        "fixture",
        "Claude session",
        "claude_transcript",
        &json!({"id":ID}),
    );
    let mut reader = ClaudeTranscriptTail::new(path.clone());
    let bytes = fs::read(&path).unwrap();
    reader.read(&mut state).unwrap();
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(state.state, "waiting");
    assert_eq!(state.public_summary.as_deref(), Some("Public summary"));
    let revision = state.revision;
    reader.read(&mut state).unwrap();
    assert_eq!(state.revision, revision);
    for mut private in [
        row(
            json!([{"type":"thinking","thinking":"PRIVATE_REASONING"},{"type":"tool_result","content":"PRIVATE_RESULT"}]),
        ),
        row(json!([{"type":"text","text":"PRIVATE_SIDECHAIN"}])),
        row(json!([{"type":"text","text":"PRIVATE_OTHER_SESSION"}])),
        json!({"type":"api-request-blob","sessionId":ID,"data":"PRIVATE_API_PAYLOAD"}),
    ] {
        if private["message"]["content"][0]["text"] == "PRIVATE_SIDECHAIN" {
            private["isSidechain"] = json!(true);
        }
        if private["message"]["content"][0]["text"] == "PRIVATE_OTHER_SESSION" {
            private["sessionId"] = json!("other");
        }
        append(&path, &private);
    }
    reader.read(&mut state).unwrap();
    assert_eq!(state.revision, revision);
    let live = row(
        json!([{"type":"tool_use","name":"Read","input":{"secret":"PRIVATE_ARGUMENT"}},{"type":"text","text":"New public activity"}]),
    );
    let encoded = serde_json::to_string(&live).unwrap();
    fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(encoded.as_bytes())
        .unwrap();
    reader.read(&mut state).unwrap();
    assert_eq!(state.revision, revision);
    fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(b"\n")
        .unwrap();
    reader.read(&mut state).unwrap();
    assert_eq!(state.state, "active");
    assert_eq!(state.revision, revision + 2);
    let mut done = row(json!([{"type":"text","text":"Finished"}]));
    done["message"]["stop_reason"] = json!("end_turn");
    append(&path, &done);
    reader.read(&mut state).unwrap();
    assert_eq!(state.state, "waiting");
    let mut error = row(json!([{"type":"text","text":"PRIVATE_ERROR"}]));
    error["isApiErrorMessage"] = json!(true);
    append(&path, &error);
    reader.read(&mut state).unwrap();
    assert_eq!(
        state.public_summary.as_deref(),
        Some("Agent reported an error")
    );
    let snapshot = state.snapshot().unwrap();
    assert!(
        !serde_json::to_string(&snapshot)
            .unwrap()
            .contains("PRIVATE")
    );
    assert!(snapshot.work_items[0].actions.is_empty());
}
#[test]
fn rejects_foreign_nested_symlinked_and_replaced_transcripts() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let project = root.join("project");
    fs::create_dir(&project).unwrap();
    let path = project.join(format!("{ID}.jsonl"));
    fs::write(&path, "{}\n").unwrap();
    assert!(claude_discovery::authorized_path(&path, &root).is_ok());
    assert!(claude_discovery::authorized_path(&path, &project).is_err());
    let nested = project.join("subagents");
    fs::create_dir(&nested).unwrap();
    let child = nested.join(format!("{ID}.jsonl"));
    fs::write(&child, "{}\n").unwrap();
    assert!(claude_discovery::authorized_path(&child, &root).is_err());
    let link = project.join("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa.jsonl");
    std::os::unix::fs::symlink(&path, &link).unwrap();
    assert!(claude_discovery::authorized_path(&link, &root).is_err());
    let mut state = Observation::new("fixture", "Claude", "claude_transcript", &json!({"id":ID}));
    let mut reader = ClaudeTranscriptTail::new(path.clone());
    reader.read(&mut state).unwrap();
    let replaced = path.with_extension("new");
    fs::write(&replaced, "{}\n").unwrap();
    fs::rename(replaced, &path).unwrap();
    assert!(reader.read(&mut state).is_err());
}
