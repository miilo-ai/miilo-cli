use chrono::{DateTime, Utc};
use miilo_adapter_sdk::{ObservatorySnapshot, ProviderDescriptor};
pub type Result<T> = std::result::Result<T, String>;
pub trait Clock {
    fn now(&self) -> DateTime<Utc>;
}
pub trait ObservationSource {
    fn descriptor(&self) -> Result<ProviderDescriptor>;
    fn observe(&mut self, now: DateTime<Utc>) -> Result<ObservatorySnapshot>;
}
pub trait SnapshotSink {
    fn publish(&mut self, snapshot: &ObservatorySnapshot) -> Result<()>;
}

pub trait FolderMetadata {
    fn canonical(&self, path: &str) -> Option<std::path::PathBuf>;
    fn is_directory(&self, path: &str) -> bool;
}
