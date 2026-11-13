mod support;
use miilo_adapter_sdk::telemetry::{
    CompletedOperation, Inference, InferenceKind, Operation, Outcome,
};
use miilo_telemetry::{Config, Context, Telemetry};
use std::time::Duration;
use support::Server;

#[test]
fn actual_otlp_protobuf_delivery_preserves_tree_and_excludes_content() {
    let server = Server::new(vec![200], Duration::ZERO, vec![]);
    let host = Telemetry::start(Config::new(&server.url, Some("collector-secret".into())).unwrap())
        .unwrap();
    let now = chrono::Utc::now();
    let mut record = CompletedOperation {
        operation: Operation::InvokeAgent {
            name: Some("assistant".into()),
            remote: false,
        },
        provider: Some("example".into()),
        started_at: now,
        ended_at: now,
        outcome: Outcome::Success,
    };
    let parent = host.record(&record, &Context::new()).unwrap().unwrap();
    record.operation = Operation::Inference {
        kind: InferenceKind::Chat,
        details: Inference {
            model: Some("example-model".into()),
            ..Default::default()
        },
    };
    host.record(&record, &parent).unwrap();
    host.shutdown().unwrap();
    assert_eq!(host.stats().delivered, 2);
    let received = server.received();
    let decoded = received
        .iter()
        .map(|request| request.trace())
        .collect::<Vec<_>>();
    let spans = decoded
        .iter()
        .flat_map(|r| &r.resource_spans)
        .flat_map(|r| &r.scope_spans)
        .flat_map(|s| &s.spans)
        .collect::<Vec<_>>();
    assert_eq!(spans.len(), 2);
    let agent = spans
        .iter()
        .find(|s| s.name == "invoke_agent assistant")
        .unwrap();
    let chat = spans
        .iter()
        .find(|s| s.name == "chat example-model")
        .unwrap();
    assert_eq!(chat.parent_span_id, agent.span_id);
    assert_eq!(chat.trace_id, agent.trace_id);
    assert_ne!(chat.trace_id, vec![0; 16]);
    assert!(chat.attributes.iter().all(|a| !a.key.contains("token")));
    assert_eq!(
        decoded[0].resource_spans[0]
            .resource
            .as_ref()
            .unwrap()
            .attributes
            .len(),
        1
    );
    for request in received {
        assert!(request.headers.contains("Bearer collector-secret"));
        assert!(!String::from_utf8_lossy(&request.body).contains("collector-secret"));
    }
}

#[test]
fn guard_completion_cancellation_and_revocation_are_distinct() {
    let server = Server::new(vec![200], Duration::ZERO, vec![]);
    let host = Telemetry::start(Config::new(&server.url, None).unwrap()).unwrap();
    host.explanation().finish(Outcome::Success);
    drop(host.explanation());
    let revoked = host.explanation();
    host.disable();
    drop(revoked);
    host.explanation().finish(Outcome::Success);
    host.shutdown().unwrap();
    assert_eq!(host.stats().submitted, 2);
    let spans = server
        .received()
        .into_iter()
        .flat_map(|r| r.trace().resource_spans)
        .flat_map(|r| r.scope_spans)
        .flat_map(|r| r.spans)
        .collect::<Vec<_>>();
    assert_eq!(spans.len(), 2);
    assert_eq!(
        spans
            .iter()
            .filter(|s| s
                .attributes
                .iter()
                .any(|a| a.key == "miilo.operation.cancelled"))
            .count(),
        1
    );
}

#[test]
fn invalid_metadata_never_reaches_collector() {
    let server = Server::new(vec![200], Duration::ZERO, vec![]);
    let host = Telemetry::start(Config::new(&server.url, None).unwrap()).unwrap();
    let now = chrono::Utc::now();
    let record = CompletedOperation {
        operation: Operation::ExecuteTool {
            name: "PRIVATE INPUT /home/user".into(),
        },
        provider: None,
        started_at: now,
        ended_at: now,
        outcome: Outcome::Success,
    };
    assert!(host.record(&record, &Context::new()).is_err());
    host.shutdown().unwrap();
    assert!(server.received().is_empty());
}
