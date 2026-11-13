use crate::{Outcome, Recorder};
use chrono::Utc;
use miilo_adapter_sdk::telemetry::{CompletedOperation, Operation};
use opentelemetry::Context;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// Exactly one completion, including future cancellation via Drop.
pub struct OperationGuard {
    recorder: Arc<Recorder>,
    enabled: Arc<AtomicBool>,
    started_at: chrono::DateTime<Utc>,
    outcome: Outcome,
    counters: Arc<crate::stats::Counters>,
}
impl OperationGuard {
    pub(crate) fn new(
        recorder: Arc<Recorder>,
        enabled: Arc<AtomicBool>,
        counters: Arc<crate::stats::Counters>,
    ) -> Self {
        Self {
            recorder,
            enabled,
            started_at: Utc::now(),
            outcome: Outcome::Cancelled,
            counters,
        }
    }
    pub fn finish(mut self, outcome: Outcome) {
        self.outcome = outcome;
    }
}
impl Drop for OperationGuard {
    fn drop(&mut self) {
        if !self.enabled.load(Ordering::Acquire) {
            return;
        }
        let result = self.recorder.record(
            &CompletedOperation {
                operation: Operation::ExecuteTool {
                    name: "request_explanation".into(),
                },
                provider: None,
                started_at: self.started_at,
                ended_at: Utc::now(),
                outcome: self.outcome,
            },
            &Context::new(),
        );
        if matches!(result, Ok(Some(_))) {
            self.counters.submitted.fetch_add(1, Ordering::Relaxed);
        }
    }
}
