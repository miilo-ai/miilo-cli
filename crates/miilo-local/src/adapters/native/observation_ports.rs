//! Native composition adapters for the observation application.
use super::{files::save_config, sessions::Session};
use crate::application::ports::{Clock, ObservationSource, Result, SnapshotSink};
use chrono::{DateTime, Utc};
use miilo_adapter_sdk::{ObservatorySnapshot, ProviderDescriptor};
use std::path::PathBuf;
pub struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}
pub struct FileSnapshot(pub PathBuf);
impl SnapshotSink for FileSnapshot {
    fn publish(&mut self, snapshot: &ObservatorySnapshot) -> Result<()> {
        save_config(
            &self.0,
            &serde_json::to_value(snapshot).map_err(|_| "Cannot serialize snapshot")?,
        )
    }
}
impl ObservationSource for Session {
    fn descriptor(&self) -> Result<ProviderDescriptor> {
        serde_json::from_value(self.observation.descriptor())
            .map_err(|_| "Invalid local descriptor".into())
    }
    fn observe(&mut self, now: DateTime<Utc>) -> Result<ObservatorySnapshot> {
        if self.tail.read(&mut self.observation).is_err() {
            self.observation.error =
                Some("Selected transcript unavailable; reconnect explicitly".into());
        }
        crate::domain::projection::project(&self.observation, &self.groups, now)
    }
}
