use chrono::{DateTime, Utc};
use miilo_adapter_sdk::{
    ObservatorySnapshot, ProviderDescriptor, runtime::observation::Observation,
};
use miilo_local::application::{
    collector::Collector,
    ports::{Clock, ObservationSource, Result, SnapshotSink},
};
use serde_json::json;
struct Fixed;
impl Clock for Fixed {
    fn now(&self) -> DateTime<Utc> {
        "2026-01-01T00:00:00Z".parse().unwrap()
    }
}
struct Source {
    state: Observation,
    fail: bool,
}
impl Source {
    fn new(provider: &str) -> Self {
        Self {
            state: Observation::at(
                provider,
                provider,
                "fixture",
                &json!({"id":"same-task"}),
                Fixed.now(),
                "epoch".into(),
            ),
            fail: false,
        }
    }
}
impl ObservationSource for Source {
    fn descriptor(&self) -> Result<ProviderDescriptor> {
        serde_json::from_value(self.state.descriptor()).map_err(|e| e.to_string())
    }
    fn observe(&mut self, now: DateTime<Utc>) -> Result<ObservatorySnapshot> {
        if self.fail {
            return Err("Read failed".into());
        }
        self.state
            .activity(&now.to_rfc3339(), "Public update".into(), "active");
        self.state.snapshot().map_err(|e| e.to_string())
    }
}
#[derive(Default)]
struct Memory(Vec<ObservatorySnapshot>);
impl SnapshotSink for Memory {
    fn publish(&mut self, snapshot: &ObservatorySnapshot) -> Result<()> {
        self.0.push(snapshot.clone());
        Ok(())
    }
}
#[test]
fn collector_runs_with_fake_ports_and_isolates_matching_task_ids() {
    let mut first = Source::new("first");
    let mut second = Source::new("second");
    let mut collector = Collector::new("process".into()).unwrap();
    let mut sink = Memory::default();
    let result = collector
        .collect(&mut [&mut first, &mut second], &Fixed, &mut sink)
        .unwrap();
    assert_eq!(result.providers.len(), 2);
    assert_eq!(result.work_items.len(), 2);
    assert_eq!(result.observed_at, Fixed.now());
    assert_eq!(sink.0.len(), 1);
    assert!(
        result
            .extensions
            .unwrap()
            .contains_key("dev.miilo.explanations")
    );
}
#[test]
fn failed_read_does_not_publish_a_partial_success_snapshot() {
    let mut first = Source::new("first");
    let mut second = Source::new("second");
    second.fail = true;
    let mut collector = Collector::new("process".into()).unwrap();
    let mut sink = Memory::default();
    assert!(
        collector
            .collect(&mut [&mut first, &mut second], &Fixed, &mut sink)
            .is_err()
    );
    assert!(sink.0.is_empty());
}
