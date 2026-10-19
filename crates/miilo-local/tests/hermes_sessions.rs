#![cfg(unix)]
use miilo_adapter_sdk::runtime::observation::Observation;
use miilo_local::session_database::{Agent, DatabaseReader, sessions};
use rusqlite::Connection;
use serde_json::json;
#[test]
fn hermes_reads_only_active_cli_assistant_content_from_selected_profile() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let path = root.join("state.db");
    let db = Connection::open(&path).unwrap();
    db.execute_batch("PRAGMA journal_mode=WAL;
    CREATE TABLE sessions(id TEXT,title TEXT,cwd TEXT,started_at REAL,source TEXT,archived INTEGER);
    CREATE TABLE messages(id INTEGER PRIMARY KEY,session_id TEXT,role TEXT,content TEXT,timestamp REAL,finish_reason TEXT,active INTEGER,reasoning TEXT,tool_calls TEXT);
    INSERT INTO sessions VALUES('selected','Public title','/fixture/project',1000.25,'cli',0),('other','Other','/fixture/project',1000,'cli',0),('private','Private','/fixture/project',1000,'telegram',0);
    INSERT INTO messages VALUES(1,'selected','user','PRIVATE_PROMPT',1000,NULL,1,NULL,NULL),(2,'selected','assistant','Visible reply',1001.5,'stop',1,'PRIVATE_REASONING','PRIVATE_ARGUMENT'),(3,'selected','assistant','PRIVATE_ARCHIVE',1002,NULL,0,NULL,NULL),(4,'other','assistant','PRIVATE_OTHER',1003,NULL,1,NULL,NULL);").unwrap();
    let found = sessions(&path, Agent::Hermes).unwrap();
    assert_eq!(found.len(), 2);
    assert!(found.iter().all(|s| s["id"] != "private"));
    let mut state = Observation::new(
        "fixture",
        "Hermes",
        "hermes_session",
        &json!({"id":"selected"}),
    );
    let mut reader =
        DatabaseReader::new(path.clone(), root.clone(), Agent::Hermes, "selected".into());
    let before = std::fs::read(&path).unwrap();
    reader.read(&mut state).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(state.public_summary.as_deref(), Some("Visible reply"));
    assert_eq!(state.state, "waiting");
    assert!(state.observed_at.contains(".500"));
    let revision = state.revision;
    reader.read(&mut state).unwrap();
    assert_eq!(state.revision, revision);
    db.execute("INSERT INTO messages VALUES(5,'selected','assistant','Live update',1004.75,NULL,1,'PRIVATE_NEW',NULL)",[]).unwrap();
    reader.read(&mut state).unwrap();
    assert_eq!(state.state, "active");
    assert_eq!(state.public_summary.as_deref(), Some("Live update"));
    assert!(
        !serde_json::to_string(&state.snapshot().unwrap())
            .unwrap()
            .contains("PRIVATE")
    );
    assert!(state.snapshot().unwrap().work_items[0].actions.is_empty());
    db.execute("UPDATE sessions SET archived=1 WHERE id='selected'", [])
        .unwrap();
    assert!(reader.read(&mut state).is_err());
    let empty = root.join("empty");
    std::fs::create_dir(&empty).unwrap();
    let empty = empty.join("state.db");
    Connection::open(&empty).unwrap();
    assert!(sessions(&empty, Agent::Hermes).is_err());
}
