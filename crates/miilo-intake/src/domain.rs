use chrono::{DateTime, Utc};
use miilo_adapter_sdk::intake::{DecisionState, ExecutionState};
use miilo_adapter_sdk::{ActionInvocation, ActionResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Submission {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub revision: String,
    pub workflow_id: String,
}
impl Submission {
    pub fn validate(&self) -> Result<(), String> {
        if [&self.id, &self.revision, &self.workflow_id]
            .iter()
            .any(|s| s.trim().is_empty() || s.len() > 200)
            || self.title.trim().is_empty()
            || self.title.len() > 1000
            || self.summary.len() > 16000
        {
            return Err("Invalid or oversized submission".into());
        }
        Ok(())
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Entry {
    pub submission: Submission,
    pub generation: u64,
    pub decision: DecisionState,
    pub execution: ExecutionState,
    pub deferred_until: Option<DateTime<Utc>>,
    #[serde(default)]
    pub job: Option<crate::execution::Job>,
    pub updated_at: DateTime<Utc>,
}
impl Entry {
    pub fn revision(&self) -> String {
        format!("{}:{}", self.submission.revision, self.generation)
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    pub invocation: ActionInvocation,
    pub result: ActionResult,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct State {
    pub version: u32,
    pub instance_id: String,
    pub sequence: u64,
    pub entries: BTreeMap<String, Entry>,
    pub receipts: BTreeMap<String, Receipt>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            version: 1,
            instance_id: String::new(),
            sequence: 0,
            entries: BTreeMap::new(),
            receipts: BTreeMap::new(),
        }
    }
}
