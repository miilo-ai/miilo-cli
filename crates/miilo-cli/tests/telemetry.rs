#[path = "../../miilo-telemetry/tests/support/mod.rs"]
mod support;
mod telemetry_fixture;
use std::{
    process::Command,
    time::{Duration, Instant},
};
use support::Server;

fn api(status: u16) -> Server {
    Server::respond(Duration::ZERO, move |_, request| {
        if request.headers.starts_with("GET /v1/snapshot ") {
            (200, telemetry_fixture::snapshot().to_string().into_bytes())
        } else if status == 200 {
            (
                200,
                telemetry_fixture::explanation().to_string().into_bytes(),
            )
        } else {
            (status, b"PRIVATE REMOTE ERROR".to_vec())
        }
    })
}
fn command(api: &Server) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_miilo"));
    cmd.args([
        "--endpoint",
        api.url.trim_end_matches("v1/traces"),
        "--json",
        "work",
        "explain",
        "private-provider/private-work",
        "--question",
        "PRIVATE QUESTION",
    ])
    .env_remove("MIILO_TOKEN")
    .env_remove("MIILO_TOKEN_FILE");
    cmd
}
#[test]
fn actual_cli_exports_only_opted_in_metadata_and_ignores_otel_environment() {
    let api = api(200);
    let collector = Server::new(vec![200], Duration::ZERO, vec![]);
    let unintended = Server::new(vec![200], Duration::ZERO, vec![]);
    let without = command(&api)
        .env("OTEL_EXPORTER_OTLP_ENDPOINT", &unintended.url)
        .output()
        .unwrap();
    assert!(
        without.status.success(),
        "{}",
        String::from_utf8_lossy(&without.stderr)
    );
    assert!(collector.received().is_empty());
    assert!(unintended.received().is_empty());
    let output = command(&api)
        .args(["--telemetry-endpoint", &collector.url])
        .env("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT", &unintended.url)
        .env("OTEL_EXPORTER_OTLP_HEADERS", "private=PRIVATE ENV HEADER")
        .env("OTEL_RESOURCE_ATTRIBUTES", "private=PRIVATE ENV RESOURCE")
        .env("OTEL_SERVICE_NAME", "PRIVATE ENV SERVICE")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, without.stdout);
    assert!(unintended.received().is_empty());
    let received = collector.received();
    assert_eq!(received.len(), 1);
    assert!(!received[0].headers.contains("PRIVATE"));
    assert!(!String::from_utf8_lossy(&received[0].body).contains("PRIVATE"));
    assert!(!String::from_utf8_lossy(&received[0].body).contains("private-"));
    let trace = received[0].trace();
    let resource = &trace.resource_spans[0];
    assert_eq!(resource.resource.as_ref().unwrap().attributes.len(), 1);
    let spans = &resource.scope_spans[0].spans;
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].name, "execute_tool request_explanation");
    assert!(
        spans[0]
            .attributes
            .iter()
            .all(|a| !a.key.contains("model") && !a.key.contains("token"))
    );
    let requests = api.received();
    assert!(
        requests
            .iter()
            .any(|r| String::from_utf8_lossy(&r.body).contains("PRIVATE QUESTION"))
    );
}

#[test]
fn failed_work_has_fixed_error_metadata() {
    let api = api(503);
    let collector = Server::new(vec![200], Duration::ZERO, vec![]);
    let output = command(&api)
        .args(["--telemetry-endpoint", &collector.url])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let received = collector.received();
    assert_eq!(received.len(), 1);
    let trace = received[0].trace();
    let span = &trace.resource_spans[0].scope_spans[0].spans[0];
    assert_eq!(span.status.as_ref().unwrap().code, 2);
    assert!(!String::from_utf8_lossy(&received[0].body).contains("PRIVATE"));
}

#[test]
fn unavailable_telemetry_does_not_turn_successful_work_into_failure() {
    let api = api(200);
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1/traces", listener.local_addr().unwrap());
    drop(listener);
    let start = Instant::now();
    let output = command(&api)
        .args(["--telemetry-endpoint", &url])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(start.elapsed() < Duration::from_secs(4));
    assert!(String::from_utf8_lossy(&output.stderr).contains("work result is unchanged"));
}
