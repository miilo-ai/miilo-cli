use super::ports::{Clock, ObservationSource, Result, SnapshotSink};
use miilo_adapter_sdk::{
    ObservatorySnapshot,
    runtime::{ObservatoryRuntime, explain},
};
use std::collections::HashSet;
pub struct Collector {
    runtime: ObservatoryRuntime,
    registered: HashSet<String>,
}
impl Collector {
    pub fn new(epoch: String) -> Result<Self> {
        Ok(Self {
            runtime: ObservatoryRuntime::new(epoch, 512).map_err(|e| e.to_string())?,
            registered: HashSet::new(),
        })
    }
    pub fn collect(
        &mut self,
        sources: &mut [&mut dyn ObservationSource],
        clock: &dyn Clock,
        sink: &mut dyn SnapshotSink,
    ) -> Result<ObservatorySnapshot> {
        let now = clock.now();
        for source in sources {
            let descriptor = source.descriptor()?;
            let id = descriptor.id.clone();
            if !self.registered.contains(&id) {
                self.runtime
                    .register(descriptor, now)
                    .map_err(|e| e.to_string())?;
                self.registered.insert(id.clone());
            }
            self.runtime
                .replace(&id, source.observe(now)?, now)
                .map_err(|e| e.to_string())?;
        }
        let mut snapshot = self.runtime.snapshot(now);
        let explanations: Vec<_> = snapshot
            .work_items
            .iter()
            .filter_map(|w| explain(&snapshot, &w.provider_id, &w.id).ok())
            .collect();
        snapshot.extensions = Some(serde_json::Map::from_iter([(
            "dev.miilo.explanations".into(),
            serde_json::to_value(explanations).map_err(|_| "Cannot encode explanations")?,
        )]));
        sink.publish(&snapshot)?;
        Ok(snapshot)
    }
}
