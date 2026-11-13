use miilo_adapter_sdk::telemetry::{
    CompletedOperation, Inference, InferenceKind, Operation, Outcome,
};
use miilo_telemetry::{Config, Context, Telemetry};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let endpoint = std::env::args()
        .nth(1)
        .ok_or("supply a local collector's full traces endpoint")?;
    let host = Telemetry::start(Config::new(&endpoint, None)?)?;
    let start = chrono::Utc::now();
    let mut operation = CompletedOperation {
        operation: Operation::InvokeAgent {
            name: Some("synthetic-assistant".into()),
            remote: false,
        },
        provider: Some("example".into()),
        started_at: start,
        ended_at: start,
        outcome: Outcome::Success,
    };
    let parent = host
        .record(&operation, &Context::new())?
        .ok_or("tracing disabled")?;
    operation.operation = Operation::ExecuteTool {
        name: "synthetic-transcription".into(),
    };
    host.record(&operation, &parent)?;
    operation.operation = Operation::Inference {
        kind: InferenceKind::Chat,
        details: Inference {
            model: Some("example-model".into()),
            ..Default::default()
        },
    };
    host.record(&operation, &parent)?;
    host.shutdown()?;
    println!(
        "collector accepted {} synthetic spans",
        host.stats().delivered
    );
    Ok(())
}
