#![cfg(unix)]
use miilo_adapter_sdk::runtime::observation::Observation;
use miilo_local::adapters::{GrokTranscriptTail, grok_discovery};
use serde_json::json;
use std::{fs, io::Write};
#[test]
fn grok_imports_format_one_and_tails_only_visible_assistant_strings() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let session = root.join("workspace/session-one");
    fs::create_dir_all(&session).unwrap();
    let summary = session.join("summary.json");
    fs::write(&summary,json!({"chat_format_version":1,"info":{"id":"session-one","cwd":"/fixture/project"},"session_summary":"PRIVATE_TITLE","created_at":"2026-10-18T00:00:00Z"}).to_string()).unwrap();
    let path = session.join("chat_history.jsonl");
    fs::write(
        &path,
        format!(
            "{}\n",
            json!({"type":"assistant","content":"Visible reply"})
        ),
    )
    .unwrap();
    let discovered = grok_discovery::sessions(&root).unwrap();
    assert_eq!(discovered.len(), 1);
    assert!(
        !serde_json::to_string(&discovered)
            .unwrap()
            .contains("PRIVATE")
    );
    let mut state = Observation::new(
        "fixture",
        "Grok",
        "grok_transcript",
        &json!({"id":"session-one"}),
    );
    let mut reader = GrokTranscriptTail::new(path.clone());
    let before = fs::read(&path).unwrap();
    reader.read(&mut state).unwrap();
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(state.state, "waiting");
    let revision = state.revision;
    reader.read(&mut state).unwrap();
    assert_eq!(state.revision, revision);
    let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
    for value in [
        json!({"type":"user","content":"PRIVATE_PROMPT"}),
        json!({"type":"reasoning","summary":"PRIVATE_REASONING"}),
        json!({"type":"tool_result","content":"PRIVATE_RESULT"}),
        json!({"type":"assistant","content":[{"type":"thinking","text":"PRIVATE_BLOCK"}]}),
    ] {
        writeln!(file, "{value}").unwrap();
    }
    reader.read(&mut state).unwrap();
    assert_eq!(state.revision, revision);
    let live = json!({"type":"assistant","content":"New reply"}).to_string();
    file.write_all(live.as_bytes()).unwrap();
    reader.read(&mut state).unwrap();
    assert_eq!(state.revision, revision);
    file.write_all(b"\n").unwrap();
    reader.read(&mut state).unwrap();
    assert_eq!(state.state, "active");
    assert_eq!(state.public_summary.as_deref(), Some("New reply"));
    let snapshot = state.snapshot().unwrap();
    assert!(snapshot.work_items[0].actions.is_empty());
    assert!(
        !serde_json::to_string(&snapshot)
            .unwrap()
            .contains("PRIVATE")
    );
    fs::write(&summary, json!({"chat_format_version":99}).to_string()).unwrap();
    assert!(grok_discovery::sessions(&root).unwrap().is_empty());
    let foreign = tempfile::tempdir().unwrap();
    assert!(grok_discovery::authorized_path(&path, foreign.path()).is_err());
    let replacement = path.with_extension("new");
    fs::write(&replacement, "{}\n").unwrap();
    fs::rename(replacement, &path).unwrap();
    assert!(reader.read(&mut state).is_err());
}
