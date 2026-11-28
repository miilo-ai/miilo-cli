//! Inbound application port used by API transports and backend integrations.
use crate::{application, domain::Submission, projection, storage::Files};
use miilo_adapter_sdk::{ActionInvocation, ActionResult, ObservatorySnapshot};
use std::path::PathBuf;
pub trait Service: Send + Sync {
    fn snapshot(&self) -> Result<ObservatorySnapshot, String>;
    fn invoke(&self, request: ActionInvocation) -> Result<ActionResult, String>;
    fn import(&self, _request: Submission) -> Result<bool, String> {
        Err("This backend does not support manual imports".into())
    }
}
pub struct Manual {
    pub root: PathBuf,
}
impl Service for Manual {
    fn snapshot(&self) -> Result<ObservatorySnapshot, String> {
        Files::new(&self.root)?.transaction(|s| {
            let now = chrono::Utc::now();
            application::wake(s, now);
            Ok(projection::snapshot(s, now))
        })
    }
    fn invoke(&self, request: ActionInvocation) -> Result<ActionResult, String> {
        Files::new(&self.root)?.transaction(|s| application::decide(s, request, chrono::Utc::now()))
    }
    fn import(&self, request: Submission) -> Result<bool, String> {
        Files::new(&self.root)?.transaction(|s| application::import(s, request, chrono::Utc::now()))
    }
}
