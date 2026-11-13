#![cfg(unix)]
#[path = "../../miilo-telemetry/tests/support/mod.rs"]
mod support;
mod telemetry_fixture;
use std::{
    os::unix::fs::PermissionsExt,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
use support::Server;

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn api(delay: Duration) -> Server {
    Server::respond(delay, |_, request| {
        let body = if request.headers.starts_with("GET ") {
            telemetry_fixture::snapshot()
        } else {
            telemetry_fixture::explanation()
        };
        (200, body.to_string().into_bytes())
    })
}
fn command(api: &Server, collector: &Server) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_miilo"));
    cmd.args([
        "--endpoint",
        api.url.trim_end_matches("v1/traces"),
        "work",
        "explain",
        "private-provider/private-work",
        "--telemetry-endpoint",
        &collector.url,
    ])
    .env_remove("MIILO_TOKEN")
    .env_remove("MIILO_TOKEN_FILE");
    cmd
}
#[test]
fn ctrl_c_exports_cancelled_operation_and_exits_promptly() {
    let api = api(Duration::from_millis(500));
    let collector = Server::new(vec![200], Duration::ZERO, vec![]);
    let mut child = Process(
        command(&api, &collector)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    while !api
        .received()
        .iter()
        .any(|r| r.headers.starts_with("POST "))
    {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        Command::new("/bin/kill")
            .args(["-INT", &child.0.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let start = Instant::now();
    while child.0.try_wait().unwrap().is_none() {
        assert!(start.elapsed() < Duration::from_secs(3));
        std::thread::sleep(Duration::from_millis(5));
    }
    let requests = collector.received();
    assert_eq!(requests.len(), 1);
    let decoded = requests[0].trace();
    let spans = &decoded.resource_spans[0].scope_spans[0].spans;
    assert_eq!(spans.len(), 1);
    assert!(
        spans[0]
            .attributes
            .iter()
            .any(|a| a.key == "miilo.operation.cancelled")
    );
}

#[test]
fn collector_token_is_separate_private_and_not_in_trace_or_errors() {
    let api = api(Duration::ZERO);
    let collector = Server::new(vec![200], Duration::ZERO, vec![]);
    let scratch = tempfile::tempdir().unwrap();
    let token = scratch.path().join("token");
    std::fs::write(&token, "collector-secret\n").unwrap();
    std::fs::set_permissions(&token, std::fs::Permissions::from_mode(0o600)).unwrap();
    let output = command(&api, &collector)
        .args([
            "--token",
            "api-secret",
            "--telemetry-token-file",
            token.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let requests = collector.received();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].headers.contains("Bearer collector-secret"));
    assert!(!requests[0].headers.contains("api-secret"));
    assert!(!String::from_utf8_lossy(&requests[0].body).contains("secret"));
    assert!(
        api.received()
            .iter()
            .all(|r| !r.headers.contains("collector-secret"))
    );
    std::fs::set_permissions(&token, std::fs::Permissions::from_mode(0o644)).unwrap();
    let output = command(&api, &collector)
        .args(["--telemetry-token-file", token.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("collector-secret"));
    assert_eq!(collector.received().len(), 1);
}
