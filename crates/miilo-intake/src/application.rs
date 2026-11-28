use crate::domain::{Entry, Receipt, State, Submission};
use chrono::{DateTime, Utc};
use miilo_adapter_sdk::intake::{DecisionState, ExecutionState, action};
use miilo_adapter_sdk::{
    ActionInvocation, ActionResult, ActionStatus, ConfirmationMethod, SubjectKind,
};

pub const PROVIDER: &str = "manual-intake";
/// All mutations must execute inside one repository transaction.
pub fn import(state: &mut State, input: Submission, now: DateTime<Utc>) -> Result<bool, String> {
    input.validate()?;
    if let Some(old) = state.entries.get(&input.id) {
        if old.submission.revision == input.revision {
            if serde_json::to_value(&old.submission).ok() != serde_json::to_value(&input).ok() {
                return Err("Source revision reused with different content".into());
            }
            return Ok(false);
        }
        if old.decision == DecisionState::Accepted {
            return Err("Accepted work needs explicit reconciliation before replacement".into());
        }
    }
    if !state.entries.contains_key(&input.id) && state.entries.len() >= 64 {
        return Err("Inbox capacity reached".into());
    }
    let generation = state.entries.get(&input.id).map_or(1, |e| e.generation + 1);
    state.entries.insert(
        input.id.clone(),
        Entry {
            submission: input,
            generation,
            decision: DecisionState::Pending,
            execution: ExecutionState::NotStarted,
            deferred_until: None,
            job: None,
            updated_at: now,
        },
    );
    state.sequence += 1;
    Ok(true)
}
pub fn decide(
    state: &mut State,
    request: ActionInvocation,
    now: DateTime<Utc>,
) -> Result<ActionResult, String> {
    if request.protocol_version != "0.1"
        || request.provider_id != PROVIDER
        || !matches!(request.subject.kind, SubjectKind::Task)
        || request.request_id.trim().is_empty()
        || request.request_id.len() > 200
        || request
            .idempotency_key
            .as_deref()
            .is_none_or(|v| v.trim().is_empty() || v.len() > 200)
    {
        return Err("Invalid action envelope".into());
    }
    let key = request.idempotency_key.as_ref().unwrap();
    if let Some(previous) = state.receipts.get(key) {
        if serde_json::to_value(&previous.invocation).ok() != serde_json::to_value(&request).ok() {
            return Err("Idempotency key already bound to another command".into());
        }
        return Ok(previous.result.clone());
    }
    if state
        .receipts
        .values()
        .any(|r| r.invocation.request_id == request.request_id)
    {
        return Err("Request ID already used".into());
    }
    if state.receipts.len() >= 10000 {
        return Err("Receipt capacity reached; archive before more decisions".into());
    }
    if !matches!(
        request.confirmation.method,
        ConfirmationMethod::Manual | ConfirmationMethod::Voice
    ) || (now - request.confirmation.confirmed_at).num_seconds() < -30
        || (now - request.confirmation.confirmed_at).num_seconds() > 120
    {
        return Err("Explicit current user confirmation required".into());
    }
    let entry = state
        .entries
        .get_mut(&request.subject.id)
        .ok_or("Unknown intake item")?;
    if request.expected_revision != entry.revision() {
        return Err("Intake revision changed; refresh before deciding".into());
    }
    if entry.decision == DecisionState::Accepted {
        return Err("Work already accepted; dismissal is not cancellation".into());
    }
    match request.action_id.as_str() {
        action::ACCEPT | action::DISMISS => {
            if !request.input.is_empty() {
                return Err("Unexpected action input".into());
            }
            entry.decision = if request.action_id == action::ACCEPT {
                DecisionState::Accepted
            } else {
                DecisionState::Dismissed
            };
            entry.execution = if entry.decision == DecisionState::Accepted {
                ExecutionState::Queued
            } else {
                ExecutionState::NotStarted
            };
            entry.deferred_until = None;
        }
        action::DEFER => {
            if request.input.len() != 1 {
                return Err("Defer requires only until".into());
            }
            let until = DateTime::parse_from_rfc3339(
                request
                    .input
                    .get("until")
                    .and_then(|v| v.as_str())
                    .ok_or("Defer requires until")?,
            )
            .map_err(|_| "Invalid defer timestamp")?
            .with_timezone(&Utc);
            if until <= now {
                return Err("Defer time must be in the future".into());
            }
            entry.decision = DecisionState::Deferred;
            entry.deferred_until = Some(until);
        }
        _ => return Err("Unsupported intake action".into()),
    }
    entry.generation += 1;
    entry.updated_at = now;
    state.sequence += 1;
    let result = ActionResult {
        protocol_version: "0.1".into(),
        request_id: request.request_id.clone(),
        provider_id: PROVIDER.into(),
        action_id: request.action_id.clone(),
        status: ActionStatus::Completed,
        recorded_at: now,
        provider_state: Some(
            if entry.decision == DecisionState::Accepted {
                "queued_awaiting_executor"
            } else {
                "decision_recorded"
            }
            .into(),
        ),
        message: Some(
            if entry.decision == DecisionState::Accepted {
                "Acceptance saved. Execution has not started; monitor dispatch status."
            } else {
                "Decision saved."
            }
            .into(),
        ),
        native_url: None,
        result: None,
    };
    state.receipts.insert(
        key.clone(),
        Receipt {
            invocation: request,
            result: result.clone(),
        },
    );
    Ok(result)
}
pub fn wake(state: &mut State, now: DateTime<Utc>) {
    for entry in state.entries.values_mut() {
        if entry.decision == DecisionState::Deferred
            && entry.deferred_until.is_some_and(|t| t <= now)
        {
            entry.decision = DecisionState::Pending;
            entry.deferred_until = None;
            entry.generation += 1;
            entry.updated_at = now;
            state.sequence += 1;
        }
    }
}
