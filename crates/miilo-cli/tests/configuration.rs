#![cfg(unix)]
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::Path,
    process::{Command, Output},
};

fn command(home: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_miilo"));
    cmd.env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("XDG_STATE_HOME", home.join(".local/state"))
        .env_remove("MIILO_HOME")
        .env_remove("MIILO_LOCAL_CONFIG")
        .env_remove("MIILO_OWL_CONFIG_DIR")
        .env("MIILO_URL", "http://invalid.invalid")
        .env("MIILO_TOKEN_FILE", home.join("missing-token"));
    cmd
}
fn run(home: &Path, action: &str) -> Output {
    command(home).args(["config", action]).output().unwrap()
}
fn private(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}
fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn migration_preserves_connections_credentials_preferences_and_collector_path() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let endpoints = r#"{"version":1,"endpoints":[{"id":"lab","url":"https://example.invalid","tokenFile":"/fixture/token","providers":["factory"],"allowActions":false}]}"#;
    let preferences =
        r#"{"observatoryView":"orbit","robotMotion":"off","robotColors":[{"key":"a","color":3}]}"#;
    let connection_path = home.join(".config/miilo-owl/config.json");
    let preferences_path = home.join(".local/state/miilo/widgets/70-miilo-fleet.owl.json");
    let local_path = home.join(".config/miilo/local.json");
    private(&connection_path, endpoints);
    private(&preferences_path, preferences);
    private(
        &local_path,
        r#"{"version":2,"sessions":[],"futureEnrollment":false}"#,
    );
    let result = success(run(home, "migrate"));
    assert_eq!(result["copiedFiles"], 2);
    let root = home.join(".miilo");
    assert_eq!(
        fs::read_to_string(root.join("connections/config.json")).unwrap(),
        endpoints
    );
    assert_eq!(
        fs::read_to_string(root.join("preferences/70-miilo-fleet.owl.json")).unwrap(),
        preferences
    );
    assert_eq!(fs::read_to_string(&connection_path).unwrap(), endpoints);
    assert_eq!(fs::read_to_string(&preferences_path).unwrap(), preferences);
    assert!(!root.join("local.json").exists());
    assert_eq!(
        success(run(home, "paths"))["localConfiguration"],
        json!(local_path)
    );
    assert_eq!(
        fs::metadata(root.join("config.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(root).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(success(run(home, "migrate"))["changed"], false);
    assert_eq!(success(run(home, "check"))["valid"], true);
}

#[test]
fn conflicting_destination_never_activates_or_overwrites() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    private(
        &home.join(".config/miilo-owl/config.json"),
        r#"{"version":1,"endpoints":[]}"#,
    );
    let target = home.join(".miilo/connections/config.json");
    private(&target, "{}");
    assert!(!run(home, "migrate").status.success());
    assert_eq!(fs::read_to_string(target).unwrap(), "{}");
    assert!(!home.join(".miilo/config.json").exists());
}

#[test]
fn symlink_source_or_target_is_rejected() {
    for source_link in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path();
        let source = home.join(".config/miilo-owl/config.json");
        let target = home.join(".miilo/connections/config.json");
        let outside = home.join("outside.json");
        private(&outside, "{}");
        if source_link {
            fs::create_dir_all(source.parent().unwrap()).unwrap();
            symlink(&outside, &source).unwrap();
        } else {
            private(&source, "{}");
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            symlink(&outside, &target).unwrap();
        }
        assert!(!run(home, "migrate").status.success());
        assert_eq!(fs::read_to_string(outside).unwrap(), "{}");
        assert!(!home.join(".miilo/config.json").exists());
    }
}

#[test]
fn invalid_or_future_manifest_fails_without_reinitializing() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let path = home.join(".miilo/config.json");
    private(&path, r#"{"schemaVersion":99}"#);
    assert!(!run(home, "check").status.success());
    assert!(!run(home, "init").status.success());
    assert_eq!(fs::read_to_string(path).unwrap(), r#"{"schemaVersion":99}"#);
}

#[test]
fn explicit_overrides_win_and_project_configuration_is_not_loaded() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let project = home.join("project");
    private(
        &project.join(".miilo/config.json"),
        "bad project configuration",
    );
    let root = home.join("custom");
    let local = home.join("explicit-local.json");
    let output = command(home)
        .current_dir(project)
        .env("MIILO_HOME", &root)
        .env("MIILO_LOCAL_CONFIG", &local)
        .args(["config", "init"])
        .output()
        .unwrap();
    success(output);
    let output = command(home)
        .env("MIILO_HOME", &root)
        .env("MIILO_LOCAL_CONFIG", &local)
        .args(["config", "paths"])
        .output()
        .unwrap();
    assert_eq!(success(output)["localConfiguration"], json!(local));
    assert!(root.join("config.json").exists());
    assert!(!home.join(".miilo").exists());
}

#[test]
fn invalid_legacy_json_and_public_credentials_fail_before_activation() {
    for (text, mode) in [("invalid", 0o600), ("{}", 0o644)] {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path();
        let source = home.join(".config/miilo-owl/config.json");
        private(&source, text);
        fs::set_permissions(&source, fs::Permissions::from_mode(mode)).unwrap();
        assert!(!run(home, "migrate").status.success());
        assert!(!home.join(".miilo/config.json").exists());
    }
}

#[test]
fn fresh_initialization_uses_shared_local_path_without_enrolling() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    assert_eq!(success(run(home, "paths"))["active"], false);
    success(run(home, "init"));
    let result = success(run(home, "paths"));
    assert_eq!(
        result["localConfiguration"],
        json!(home.join(".miilo/local.json"))
    );
    assert_eq!(
        result["connections"],
        json!(home.join(".miilo/connections/config.json"))
    );
    assert!(!home.join(".miilo/local.json").exists());
}
