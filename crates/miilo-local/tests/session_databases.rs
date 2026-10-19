#![cfg(unix)]
use miilo_adapter_sdk::runtime::observation::Observation;
use miilo_local::session_database::{Agent, DatabaseReader, sessions};
use rusqlite::{Connection, params};
use serde_json::json;
fn fixture(agent: Agent) -> (tempfile::TempDir, std::path::PathBuf, Connection) {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().canonicalize().unwrap().join(agent.filename());
    let db = Connection::open(&path).unwrap();
    match agent {
        Agent::OpenCode=>db.execute_batch("CREATE TABLE session(id TEXT PRIMARY KEY,title TEXT,directory TEXT,time_created INTEGER,time_updated INTEGER,time_archived INTEGER);CREATE TABLE message(id TEXT PRIMARY KEY,session_id TEXT,time_updated INTEGER,data TEXT);CREATE TABLE part(id TEXT PRIMARY KEY,message_id TEXT,session_id TEXT,time_updated INTEGER,data TEXT);").unwrap(),
        Agent::Devin=>db.execute_batch("CREATE TABLE sessions(id TEXT PRIMARY KEY,title TEXT,working_directory TEXT,created_at INTEGER,last_activity_at INTEGER,hidden INTEGER,main_chain_id INTEGER);CREATE TABLE message_nodes(node_id INTEGER,session_id TEXT,parent_node_id INTEGER,created_at INTEGER,chat_message TEXT);").unwrap(),
        Agent::Hermes=>db.execute_batch("CREATE TABLE sessions(id TEXT PRIMARY KEY,title TEXT,cwd TEXT,started_at REAL,source TEXT,archived INTEGER);CREATE TABLE messages(id INTEGER,session_id TEXT,role TEXT,content TEXT,timestamp REAL,finish_reason TEXT,active INTEGER);").unwrap(),
    }
    let sql = match agent {
        Agent::OpenCode => "INSERT INTO session VALUES(?1,'Existing session',?2,1000,1000,NULL)",
        Agent::Devin => "INSERT INTO sessions VALUES(?1,'Existing session',?2,1,1,0,1)",
        Agent::Hermes => "INSERT INTO sessions VALUES(?1,'Existing session',?2,1,'cli',0)",
    };
    db.execute(sql, params!["selected", temp.path().to_str()])
        .unwrap();
    (temp, path, db)
}
fn observation() -> Observation {
    Observation::new(
        "selected",
        "Existing session",
        "database_session",
        &json!({"id":"selected","name":"Existing session"}),
    )
}
#[test]
fn opencode_filters_private_payloads_and_observes_wal_updates_without_writes() {
    let (temp, path, db) = fixture(Agent::OpenCode);
    db.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
    for (id, role) in [("assistant", "assistant"), ("user", "user")] {
        db.execute(
            "INSERT INTO message VALUES(?1,'selected',1000,?2)",
            params![id, json!({"role":role}).to_string()],
        )
        .unwrap();
    }
    for (id, msg, part) in [
        (
            "public",
            "assistant",
            json!({"type":"text","text":"Public progress"}),
        ),
        (
            "reasoning",
            "assistant",
            json!({"type":"reasoning","text":"PRIVATE_REASONING"}),
        ),
        ("user", "user", json!({"type":"text","text":"PRIVATE_USER"})),
        (
            "tool",
            "assistant",
            json!({"type":"tool","tool":"check","state":{"input":"PRIVATE_INPUT","output":"PRIVATE_OUTPUT"}}),
        ),
    ] {
        db.execute(
            "INSERT INTO part VALUES(?1,?2,'selected',1000,?3)",
            params![id, msg, part.to_string()],
        )
        .unwrap();
    }
    let discovered = sessions(&path, Agent::OpenCode).unwrap();
    assert_eq!(discovered.len(), 1);
    let mut reader = DatabaseReader::new(
        path.clone(),
        temp.path().to_owned(),
        Agent::OpenCode,
        "selected".into(),
    );
    let mut state = observation();
    reader.read(&mut state).unwrap();
    assert_eq!(state.state, "waiting");
    let revision = state.revision;
    reader.read(&mut state).unwrap();
    assert_eq!(state.revision, revision, "Polling must not invent activity");
    let before = std::fs::read(&path).unwrap();
    db.execute(
        "UPDATE part SET time_updated=2000,data=?1 WHERE id='public'",
        [json!({"type":"text","text":"Updated progress"}).to_string()],
    )
    .unwrap();
    reader.read(&mut state).unwrap();
    assert_eq!(state.public_summary.as_deref(), Some("Updated progress"));
    assert_eq!(state.state, "active");
    assert_eq!(
        std::fs::read(&path).unwrap(),
        before,
        "Observer and WAL writer do not checkpoint the main database"
    );
    let snapshot = serde_json::to_string(&state.snapshot().unwrap()).unwrap();
    assert!(!snapshot.contains("PRIVATE_"));
    assert!(state.snapshot().unwrap().work_items[0].actions.is_empty());
    db.execute("UPDATE message SET data=?1,time_updated=3000 WHERE id='assistant'",[json!({"role":"assistant","error":{"name":"APIError","data":{"message":"PRIVATE_ERROR"}}}).to_string()]).unwrap();
    reader.read(&mut state).unwrap();
    assert_eq!(state.state, "waiting");
    assert_eq!(
        state.public_summary.as_deref(),
        Some("Agent reported an error")
    );
}
#[test]
fn devin_uses_only_current_branch_and_public_assistant_content() {
    let (temp, path, db) = fixture(Agent::Devin);
    for (id, parent, role, text) in [
        (1, None, "user", "PRIVATE_USER"),
        (2, Some(1), "assistant", "Public answer"),
        (3, Some(1), "assistant", "PRIVATE_ABANDONED"),
    ] {
        let row = json!({"role":role,"content":text,"thinking":{"thinking":"PRIVATE_REASONING"},"metadata":{"finish_reason":"stop","authorization":"PRIVATE_TOKEN"}});
        db.execute(
            "INSERT INTO message_nodes VALUES(?1,'selected',?2,?1,?3)",
            params![id, parent, row.to_string()],
        )
        .unwrap();
    }
    db.execute(
        "UPDATE sessions SET main_chain_id=2 WHERE id='selected'",
        [],
    )
    .unwrap();
    let mut reader = DatabaseReader::new(
        path.clone(),
        temp.path().to_owned(),
        Agent::Devin,
        "selected".into(),
    );
    let mut state = observation();
    reader.read(&mut state).unwrap();
    assert_eq!(state.public_summary.as_deref(), Some("Public answer"));
    assert_eq!(state.state, "waiting");
    assert!(
        !serde_json::to_string(&state.snapshot().unwrap())
            .unwrap()
            .contains("PRIVATE_")
    );
    let revision = state.revision;
    reader.read(&mut state).unwrap();
    assert_eq!(state.revision, revision);
    db.execute(
        "INSERT INTO message_nodes VALUES(4,'selected',2,4,?1)",
        [json!({"role":"assistant","content":"New progress"}).to_string()],
    )
    .unwrap();
    db.execute(
        "UPDATE sessions SET main_chain_id=4 WHERE id='selected'",
        [],
    )
    .unwrap();
    reader.read(&mut state).unwrap();
    assert_eq!(state.state, "active");
    assert_eq!(state.public_summary.as_deref(), Some("New progress"));
    db.execute("UPDATE sessions SET hidden=1", []).unwrap();
    assert!(reader.read(&mut state).is_err());
    assert!(sessions(&path, Agent::Devin).unwrap().is_empty());
}
#[test]
fn paths_missing_schema_replacement_and_selected_session_fail_closed() {
    for agent in [Agent::OpenCode, Agent::Devin, Agent::Hermes] {
        let (temp, path, db) = fixture(agent);
        let mut reader = DatabaseReader::new(
            path.clone(),
            temp.path().to_owned(),
            agent,
            "selected".into(),
        );
        let mut state = observation();
        reader.read(&mut state).unwrap();
        let mut absent = DatabaseReader::new(
            path.clone(),
            temp.path().to_owned(),
            agent,
            "not-selected".into(),
        );
        assert!(absent.read(&mut state).is_err());
        let other = tempfile::tempdir().unwrap();
        assert!(
            miilo_local::session_database::authorized_path(&path, other.path(), agent).is_err()
        );
        let original = path.with_extension("old");
        drop(db);
        std::fs::rename(&path, &original).unwrap();
        std::fs::copy(&original, &path).unwrap();
        assert!(reader.read(&mut state).is_err());
        std::fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink(&original, &path).unwrap();
        assert!(sessions(&path, agent).is_err());
    }
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("opencode.db");
    Connection::open(&path).unwrap();
    assert!(sessions(&path, Agent::OpenCode).is_err());
}
