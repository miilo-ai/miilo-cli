#![cfg(unix)]
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
const ID: &str = "11111111-1111-1111-1111-111111111111";
fn command(home: &std::path::Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_miilo-local"));
    cmd.env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("XDG_STATE_HOME", home.join(".local/state"))
        .env("MIILO_CURSOR_PROJECTS", home.join("cursor"))
        .env("MIILO_CODEX_EXECUTABLE", home.join("no-codex-installed"))
        .env("MIILO_CE_EXECUTABLE", home.join("no-ce-installed"))
        .env("MIILO_URL", "http://invalid.invalid")
        .env("MIILO_TOKEN_FILE", home.join("missing-token"))
        .env_remove("MIILO_LOCAL_CONFIG");
    cmd
}
#[test]
fn import_observe_update_explain_and_stale_without_ce_or_account() {
    let scratch = tempfile::tempdir().unwrap();
    let home = scratch.path().canonicalize().unwrap();
    let project = home.join("work/my-project");
    fs::create_dir_all(&project).unwrap();
    let file = home.join(format!("cursor/project/agent-transcripts/{ID}/{ID}.jsonl"));
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file,"{\"role\":\"user\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"PRIVATE_USER_INPUT\"}]}}\n").unwrap();
    let mut child = command(&home)
        .arg("import")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    writeln!(child.stdin.take().unwrap(),"{}",json!({"project":project,"sessions":[{"kind":"cursor","id":ID,"path":"/forged"}],"startService":false,"connectWidget":false})).unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let receipt: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(receipt["mode"], "local");
    let config = home.join(".config/miilo/local.json");
    let saved: Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
    assert_eq!(saved["version"], 2);
    assert!(saved.get("endpoint").is_none());
    assert!(saved.get("tokenFile").is_none());
    assert_ne!(saved["sessions"][0]["path"], "/forged");
    assert_eq!(
        fs::metadata(&config).unwrap().permissions().mode() & 0o077,
        0
    );
    let snapshot_file = std::path::Path::new(saved["snapshotFile"].as_str().unwrap());
    let mut observer = command(&home)
        .arg("run")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(150));
    let mut transcript = fs::OpenOptions::new().append(true).open(&file).unwrap();
    writeln!(transcript,"{}",json!({"role":"assistant","message":{"content":[{"type":"text","text":"Checking the deployment"},{"type":"tool_use","name":"test","input":{"secret":"PRIVATE_TOOL_INPUT"}}]}})).unwrap();
    transcript.flush().unwrap();
    let deadline = Instant::now() + Duration::from_secs(8);
    let snapshot = loop {
        let snapshot: Value = serde_json::from_slice(&fs::read(snapshot_file).unwrap()).unwrap();
        if snapshot["workItems"][0]["summary"] == "Checking the deployment" {
            break snapshot;
        }
        assert!(Instant::now() < deadline, "observer did not ingest update");
        std::thread::sleep(Duration::from_millis(100));
    };
    let encoded = snapshot.to_string();
    assert!(!encoded.contains("PRIVATE_USER_INPUT"));
    assert!(!encoded.contains("PRIVATE_TOOL_INPUT"));
    assert!(
        snapshot["workItems"][0]["actions"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let target = format!(
        "{}/{}",
        snapshot["workItems"][0]["providerId"].as_str().unwrap(),
        ID
    );
    let result = command(&home).args(["explain", &target]).output().unwrap();
    assert!(result.status.success());
    let explanation: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert!(explanation["evidenceIds"].as_array().unwrap().is_empty());
    assert!(
        explanation["limitations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s.as_str().unwrap().contains("No verification"))
    );
    observer.kill().unwrap();
    observer.wait().unwrap();
    let mut stale = snapshot;
    stale["observedAt"] = json!("2020-01-01T00:00:00Z");
    fs::write(snapshot_file, stale.to_string()).unwrap();
    assert!(
        !command(&home)
            .arg("snapshot")
            .output()
            .unwrap()
            .status
            .success()
    );
    fs::set_permissions(&config, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(
        !command(&home)
            .args(["run", "--once"])
            .output()
            .unwrap()
            .status
            .success()
    );
}
