mod support;
use miilo_telemetry::{Config, Outcome, Telemetry};
use opentelemetry_proto::tonic::collector::trace::v1::{
    ExportTracePartialSuccess, ExportTraceServiceResponse,
};
use prost::Message;
use std::time::{Duration, Instant};
use support::Server;

#[test]
fn transient_failure_has_one_bounded_retry() {
    let server = Server::new(vec![503, 200], Duration::ZERO, vec![]);
    let host = Telemetry::start(Config::new(&server.url, None).unwrap()).unwrap();
    host.explanation().finish(Outcome::Success);
    host.shutdown().unwrap();
    assert_eq!(server.received().len(), 2);
    assert_eq!(host.stats().delivered, 1);
}

#[test]
fn permanent_errors_redirects_and_partial_rejections_are_not_retried() {
    for (status, body) in [
        (400, vec![]),
        (302, vec![]),
        (
            200,
            ExportTraceServiceResponse {
                partial_success: Some(ExportTracePartialSuccess {
                    rejected_spans: 1,
                    error_message: "PRIVATE COLLECTOR ERROR".into(),
                }),
            }
            .encode_to_vec(),
        ),
    ] {
        let server = Server::new(vec![status], Duration::ZERO, body);
        let host =
            Telemetry::start(Config::new(&server.url, Some("private-token".into())).unwrap())
                .unwrap();
        host.explanation().finish(Outcome::Success);
        let error = host.shutdown().unwrap_err();
        assert!(!error.contains("PRIVATE"));
        assert!(!error.contains("private-token"));
        assert_eq!(host.stats().failed_batches, 1);
        assert_eq!(server.received().len(), 1);
    }
}

#[test]
fn saturated_queue_does_not_block_operations_or_unbound_shutdown() {
    let server = Server::new(vec![503], Duration::from_millis(600), vec![]);
    let host = Telemetry::start(Config::new(&server.url, None).unwrap()).unwrap();
    let start = Instant::now();
    for _ in 0..10_000 {
        host.explanation().finish(Outcome::Success);
    }
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "recording waited for collector"
    );
    let start = Instant::now();
    assert!(host.shutdown().is_err());
    assert!(start.elapsed() < Duration::from_secs(3));
    assert_eq!(host.stats().submitted, 10_000);
    assert!(host.stats().delivered < host.stats().submitted);
}

#[test]
fn refused_connection_is_best_effort_and_shutdown_is_bounded() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1/traces", listener.local_addr().unwrap());
    drop(listener);
    let host = Telemetry::start(Config::new(&url, None).unwrap()).unwrap();
    let start = Instant::now();
    host.explanation().finish(Outcome::Success);
    assert!(host.shutdown().is_err());
    assert!(start.elapsed() < Duration::from_secs(3));
}

#[test]
fn endpoint_and_token_validation_does_not_echo_private_values() {
    for url in [
        "file:///private",
        "http://example.com/v1/traces",
        "https://user:PRIVATE@example.com",
        "https://example.com/?key=PRIVATE",
        "https://example.com/#PRIVATE",
    ] {
        assert!(
            !Config::new(url, None)
                .err()
                .unwrap()
                .to_string()
                .contains("PRIVATE")
        );
    }
    for url in [
        "http://127.0.0.1:4318/v1/traces",
        "http://[::1]:4318/v1/traces",
        "https://collector.example/v1/traces",
    ] {
        assert!(Config::new(url, None).is_ok());
    }
    assert!(Config::new("https://collector.example", Some("bad\r\ntoken".into())).is_err());
}
