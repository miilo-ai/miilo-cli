mod config;
mod exporter;
mod operation;
mod stats;
pub use stats::DeliveryStats;

pub use config::{Config, ConfigError};
pub use miilo_adapter_sdk::telemetry::{CompletedOperation, ErrorKind, Outcome, TelemetryError};
use miilo_adapter_sdk::telemetry::{GenAiTelemetry, OpenTelemetrySink};
pub use opentelemetry::Context;
use opentelemetry::{KeyValue, trace::TracerProvider};
use opentelemetry_sdk::{
    Resource,
    trace::{BatchConfigBuilder, BatchSpanProcessor, Sampler, SdkTracerProvider},
};
pub use operation::OperationGuard;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

type Recorder = GenAiTelemetry<OpenTelemetrySink<opentelemetry_sdk::trace::Tracer>>;

/// Own this on the blocking host thread, never in the UI or async executor.
pub struct Telemetry {
    provider: SdkTracerProvider,
    recorder: Arc<Recorder>,
    enabled: Arc<AtomicBool>,
    counters: Arc<stats::Counters>,
}
impl Telemetry {
    pub fn start(config: Config) -> Result<Self, &'static str> {
        let counters = Arc::new(stats::Counters::default());
        let exporter = exporter::Exporter::new(config, counters.clone())?;
        let batch = BatchConfigBuilder::default()
            .with_max_queue_size(256)
            .with_max_export_batch_size(64)
            .with_scheduled_delay(Duration::from_millis(200))
            .build();
        let processor = BatchSpanProcessor::builder(exporter)
            .with_batch_config(batch)
            .build();
        let provider = SdkTracerProvider::builder()
            .with_sampler(Sampler::AlwaysOn)
            .with_resource(
                Resource::builder_empty()
                    .with_attributes([KeyValue::new("service.name", "miilo")])
                    .build(),
            )
            .with_span_processor(processor)
            .build();
        let mut recorder =
            GenAiTelemetry::new(OpenTelemetrySink::new(provider.tracer("miilo.operations")));
        recorder.set_enabled(true);
        Ok(Self {
            provider,
            recorder: Arc::new(recorder),
            enabled: Arc::new(AtomicBool::new(true)),
            counters,
        })
    }
    pub fn explanation(&self) -> OperationGuard {
        OperationGuard::new(
            self.recorder.clone(),
            self.enabled.clone(),
            self.counters.clone(),
        )
    }
    pub fn record(
        &self,
        operation: &CompletedOperation,
        parent: &Context,
    ) -> Result<Option<Context>, TelemetryError> {
        if !self.enabled.load(Ordering::Acquire) {
            return Ok(None);
        }
        let result = self.recorder.record(operation, parent)?;
        if result.is_some() {
            self.counters.submitted.fetch_add(1, Ordering::Relaxed);
        }
        Ok(result)
    }
    pub fn stats(&self) -> DeliveryStats {
        self.counters.snapshot()
    }
    /// Stops future records. Previously queued spans cannot be recalled.
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Release);
    }
    /// Best effort; reports delivery errors without changing the work result.
    pub fn shutdown(&self) -> Result<(), &'static str> {
        self.disable();
        self.provider
            .shutdown_with_timeout(Duration::from_secs(2))
            .map_err(|_| "telemetry shutdown did not complete successfully")?;
        let stats = self.stats();
        if stats.submitted != stats.delivered || stats.failed_batches > 0 {
            return Err("telemetry delivery was incomplete");
        }
        Ok(())
    }
}
