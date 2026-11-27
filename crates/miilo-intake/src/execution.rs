//! Application orchestration. Backends own execution/check/review details.
use crate::{domain::State, ports::Repository};
use chrono::{DateTime, Utc};
use miilo_adapter_sdk::intake::{DecisionState, ExecutionState};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub version: u32,
    pub job_id: String,
    pub namespace: String,
    pub item_id: String,
    pub source_revision: String,
    pub workflow_id: String,
    pub title: String,
    pub summary: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Report {
    pub job_id: String,
    pub execution: ExecutionState,
    pub summary: String,
    pub pr_url: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Job {
    pub request: Request,
    pub report: Option<Report>,
    pub last_error: Option<String>,
}
/// submit must be idempotent on job_id and return the current state for replays.
/// A backend must reject job_id reuse with a changed payload. It must run its own
/// configured verification/review before reporting ready_for_review.
pub trait Executor {
    fn submit(&self, request: &Request) -> Result<Report, String>;
}
/// Persist the exact request before external effects; an uncertain POST retains
/// the same job identity and payload. Never synthesize a successful worker start.
pub fn tick(
    store: &impl Repository,
    executor: &impl Executor,
    workflows: &[String],
    now: DateTime<Utc>,
) -> Result<usize, String> {
    let pending = store.transaction(|s| prepare(s, workflows, now))?;
    let mut changed = 0;
    for (id, request) in pending {
        let result = executor.submit(&request);
        store.transaction(|s| {
            let entry = s.entries.get_mut(&id).ok_or("Intake item disappeared")?;
            let job = entry
                .job
                .as_mut()
                .ok_or("Dispatch reservation disappeared")?;
            if serde_json::to_value(&job.request).ok() != serde_json::to_value(&request).ok() {
                return Err("Dispatch binding changed".into());
            }
            let old = serde_json::to_value(&job).map_err(|_| "Cannot inspect job")?;
            match result {
                Ok(report)
                    if report.job_id == request.job_id
                        && report.summary.len() <= 4000
                        && !matches!(report.execution, ExecutionState::NotStarted) =>
                {
                    entry.execution = report.execution;
                    job.report = Some(report);
                    job.last_error = None;
                }
                _ => {
                    entry.execution = ExecutionState::Blocked;
                    job.last_error = Some(
                        "Executor outcome unavailable or invalid; retry uses the same job identity"
                            .into(),
                    );
                }
            }
            if serde_json::to_value(&job).map_err(|_| "Cannot inspect job")? != old {
                entry.updated_at = now;
                entry.generation += 1;
                s.sequence += 1;
                changed += 1;
            }
            Ok(())
        })?;
    }
    Ok(changed)
}
fn prepare(
    s: &mut State,
    workflows: &[String],
    now: DateTime<Utc>,
) -> Result<Vec<(String, Request)>, String> {
    if uuid::Uuid::parse_str(&s.instance_id).is_err() {
        return Err("Repository requires a durable instance identity".into());
    }
    let mut requests = Vec::new();
    for (id, entry) in &mut s.entries {
        if entry.decision != DecisionState::Accepted
            || !workflows.contains(&entry.submission.workflow_id)
            || matches!(
                entry.execution,
                ExecutionState::Completed | ExecutionState::Cancelled
            )
        {
            continue;
        }
        if entry.job.is_none() {
            let identity = serde_json::to_vec(&(
                &s.instance_id,
                crate::application::PROVIDER,
                id,
                &entry.submission.revision,
                &entry.submission.workflow_id,
            ))
            .map_err(|_| "Cannot bind job")?;
            let digest = Sha256::digest(identity);
            let job_id = format!("intake-{:x}", digest);
            let request = Request {
                version: 1,
                job_id,
                namespace: s.instance_id.clone(),
                item_id: id.clone(),
                source_revision: entry.submission.revision.clone(),
                workflow_id: entry.submission.workflow_id.clone(),
                title: entry.submission.title.clone(),
                summary: entry.submission.summary.clone(),
            };
            entry.job = Some(Job {
                request,
                report: None,
                last_error: None,
            });
            entry.updated_at = now;
            entry.generation += 1;
            s.sequence += 1;
        }
        requests.push((id.clone(), entry.job.as_ref().unwrap().request.clone()));
    }
    Ok(requests)
}
