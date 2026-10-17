use chrono::{DateTime, Utc};
use miilo_adapter_sdk::{ObservatorySnapshot, runtime::observation::Observation};
use serde_json::{Value, json};
type Result<T> = std::result::Result<T, String>;
pub fn project(
    state: &Observation,
    groups: &Value,
    now: DateTime<Utc>,
) -> Result<ObservatorySnapshot> {
    let recent = chrono::DateTime::parse_from_rfc3339(&state.observed_at)
        .is_ok_and(|at| (0..120).contains(&(now - at.with_timezone(&chrono::Utc)).num_seconds()));
    let mut snapshot =
        serde_json::to_value(state.snapshot().map_err(|_| "Cannot project observation")?)
            .map_err(|_| "Cannot encode observation")?;
    snapshot["workItems"][0]["revision"] = json!(state.cursor(state.revision));
    snapshot["providers"][0]["extensions"]["dev.miilo.groups"] = groups.clone();
    snapshot["actors"] = json!([{"id":"session-agent","providerId":state.provider_id,"kind":"agent","name":state.title,"health":if state.error.is_some(){"unreachable"}else{"healthy"},"status":if state.error.is_some(){"offline"}else if state.state=="active"&&recent{"working"}else{"resting"},"capabilities":[],"updatedAt":state.observed_at,"extensions":{"dev.miilo.session":{"runtimeStatusAuthoritative":false,"recentlyActive":super::session_visibility::recently_observed(&state.observed_at, now)}}}]);
    snapshot["attempts"] = json!([{"id":"observed-session","providerId":state.provider_id,"taskId":state.thread_id,"sequence":1,"lifecycle":state.state,"phase":"unknown","actorIds":["session-agent"],"startedAt":state.observed_at,"completedAt":null,"summary":"Observed existing session; execution status is not authoritative","extensions":null}]);
    serde_json::from_value(snapshot).map_err(|_| "Cannot decode observation".into())
}
