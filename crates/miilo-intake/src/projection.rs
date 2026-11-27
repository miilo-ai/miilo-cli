use crate::{application::PROVIDER, domain::State};
use chrono::{DateTime, Utc};
use miilo_adapter_sdk::{intake::*, *};
use std::collections::BTreeMap;
pub fn snapshot(state: &State, now: DateTime<Utc>) -> ObservatorySnapshot {
    let work_items = state.entries.values().map(project_item).collect();
    ObservatorySnapshot {
        protocol_version: "0.1".into(),
        snapshot_id: format!("intake-{}-{}", state.instance_id, state.sequence),
        observed_at: now,
        cursor: format!("{}:{}", state.instance_id, state.sequence),
        providers: vec![ProviderDescriptor {
            id: PROVIDER.into(),
            name: "Manual inbox".into(),
            kind: ProviderKind::Custom,
            health: Health::Healthy,
            native_url: None,
            extensions: None,
            capabilities: ProviderCapabilities {
                observe: vec!["intake".into()],
                control: vec![
                    action::ACCEPT.into(),
                    action::DEFER.into(),
                    action::DISMISS.into(),
                ],
                features: BTreeMap::from([
                    ("intake.v1".into(), true),
                    (
                        "execution".into(),
                        state.entries.values().any(|e| e.job.is_some()),
                    ),
                ]),
                speech: None,
                model_routes: vec![],
            },
        }],
        work_items,
        actors: vec![],
        attempts: vec![],
        artifacts: vec![],
        evidence: vec![],
        attention: vec![],
        extensions: None,
    }
}

fn project_item(entry: &crate::domain::Entry) -> WorkItem {
    let details = IntakeDetails {
        version: VERSION,
        source: SourceReference {
            connection_id: PROVIDER.into(),
            external_id: entry.submission.id.clone(),
            revision: entry.submission.revision.clone(),
        },
        decision: entry.decision,
        execution: entry.execution,
        synchronization: SynchronizationState::NotRequired,
        workflow_id: entry.submission.workflow_id.clone(),
        job_id: entry.job.as_ref().map(|job| job.request.job_id.clone()),
        deferred_until: entry.deferred_until.map(|time| time.to_rfc3339()),
    };
    let lifecycle = match entry.execution {
        ExecutionState::Queued => Lifecycle::Queued,
        ExecutionState::Running | ExecutionState::Checking => Lifecycle::Active,
        ExecutionState::Completed => Lifecycle::Complete,
        ExecutionState::Cancelled => Lifecycle::Cancelled,
        _ => Lifecycle::Waiting,
    };
    let provider_state = if entry.job.is_some() {
        serde_json::to_value(entry.execution)
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned()
    } else if entry.decision == DecisionState::Accepted {
        "queued_awaiting_executor".into()
    } else {
        serde_json::to_value(entry.decision)
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned()
    };
    let phase = match entry.execution {
        ExecutionState::Running => Phase::Implementing,
        ExecutionState::Checking => Phase::Verifying,
        ExecutionState::ReadyForReview => Phase::Reviewing,
        _ => Phase::Planning,
    };
    WorkItem {
        id: entry.submission.id.clone(),
        provider_id: PROVIDER.into(),
        workspace_id: None,
        parent_id: None,
        kind: WorkItemKind::Task,
        title: entry.submission.title.clone(),
        summary: entry.submission.summary.clone(),
        lifecycle,
        phase,
        attention: if entry.decision == DecisionState::Pending {
            AttentionLevel::Decision
        } else if entry.execution == ExecutionState::Blocked {
            AttentionLevel::Intervention
        } else {
            AttentionLevel::None
        },
        health: if entry.execution == ExecutionState::Blocked {
            Health::Degraded
        } else {
            Health::Healthy
        },
        provider_state,
        revision: entry.revision(),
        dependency_ids: vec![],
        actions: project_actions(entry),
        native_url: entry
            .job
            .as_ref()
            .and_then(|j| j.report.as_ref())
            .and_then(|r| r.pr_url.clone()),
        updated_at: entry.updated_at,
        extensions: Some(serde_json::Map::from_iter([(
            EXTENSION.into(),
            serde_json::to_value(details).unwrap(),
        )])),
    }
}
fn project_actions(entry: &crate::domain::Entry) -> Vec<AvailableAction> {
    [(action::ACCEPT, "Accept"), (action::DEFER, "Later"), (action::DISMISS, "Dismiss")].into_iter().map(|(id,label)| {
        AvailableAction {
            id: id.into(), label: label.into(), risk: ActionRisk::Bounded, confirmation: ConfirmationRequirement::Explicit,
            required_scopes: vec!["intake:decide".into()], available: entry.decision != DecisionState::Accepted,
            unavailable_reason: (entry.decision == DecisionState::Accepted).then(|| "Already accepted; cancellation is separate".into()),
            expires_at: None, idempotent: true,
            input_schema: if id == action::DEFER { Some(serde_json::from_value(serde_json::json!({"type":"object","required":["until"],"additionalProperties":false,"properties":{"until":{"type":"string","format":"date-time"}}})).unwrap()) } else { None },
            native_url: None,
        }
    }).collect()
}
