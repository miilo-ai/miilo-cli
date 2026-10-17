use miilo_adapter_sdk::runtime::observation::Observation;
use serde_json::Value;
/// Cursor's observed transcript shape has no event timestamp; use receipt time honestly.
pub fn apply(row: &Value, state: &mut Observation) {
    let now = chrono::Utc::now().to_rfc3339();
    if row["type"] == "turn_ended" {
        if let Some("completed" | "success" | "aborted" | "error") = row["status"].as_str() {
            state.activity(&now, "Turn ended".into(), "waiting");
        }
        return;
    }
    if row["role"] != "assistant" {
        return;
    }
    for content in row["message"]["content"].as_array().into_iter().flatten() {
        match content["type"].as_str() {
            Some("text") => {
                if let Some(text) = content["text"].as_str().filter(|v| !v.trim().is_empty()) {
                    let summary: String = text.chars().take(280).collect();
                    state.public_summary = Some(summary.clone());
                    state.activity(&now, summary, "active");
                }
            }
            Some("tool_use") => {
                let name: String = content["name"]
                    .as_str()
                    .unwrap_or("tool")
                    .chars()
                    .filter(|c| c.is_alphanumeric() || "_./-".contains(*c))
                    .take(80)
                    .collect();
                state.activity(&now, format!("Tool started: {name}"), "active");
            }
            _ => {} // Never export thinking, inputs, outputs, user messages or email.
        }
    }
}
