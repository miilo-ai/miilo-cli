use crate::transcript::TranscriptTail as Reader;
use miilo_adapter_sdk::runtime::observation::Observation;
use serde_json::Value;
use std::path::PathBuf;
pub struct TranscriptTail(Reader);
impl TranscriptTail {
    pub fn new(path: PathBuf) -> Self {
        Self(Reader::new(path, apply))
    }
    pub fn consume(&mut self, bytes: &[u8], state: &mut Observation) {
        self.0.consume(bytes, state);
    }
    pub fn read(&mut self, state: &mut Observation) -> Result<(), String> {
        self.0.read(state)
    }
}

fn apply(row: &Value, state: &mut Observation) {
    let Some(timestamp) = row["timestamp"].as_str() else {
        return;
    };
    if chrono::DateTime::parse_from_rfc3339(timestamp).is_err() {
        return;
    }
    let payload = &row["payload"];
    let kind = payload["type"].as_str().unwrap_or("");
    let channel = payload["channel"]
        .as_str()
        .or_else(|| payload["phase"].as_str());
    match (row["type"].as_str().unwrap_or(""), kind) {
        ("response_item", "message")
            if payload["role"] == "assistant"
                && matches!(channel, Some("commentary" | "final")) =>
        {
            let text = payload["content"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|c| c["text"].as_str())
                .collect::<Vec<_>>()
                .join(" ");
            if !text.trim().is_empty() {
                state.public_summary = Some(text.chars().take(280).collect());
                state.activity(
                    timestamp,
                    text.chars().take(280).collect(),
                    if channel == Some("final") {
                        "waiting"
                    } else {
                        "active"
                    },
                );
            }
        }
        ("response_item", "function_call" | "custom_tool_call") => {
            // Never forward arguments, tool outputs, reasoning, or encrypted content.
            let name = payload["name"].as_str().unwrap_or("tool");
            let safe: String = name
                .chars()
                .filter(|c| c.is_alphanumeric() || "_./-".contains(*c))
                .take(80)
                .collect();
            state.activity(timestamp, format!("Tool started: {safe}"), "active");
        }
        ("response_item", "function_call_output" | "custom_tool_call_output") => {
            state.activity(timestamp, "Tool completed".into(), "active")
        }
        ("event_msg", "task_started") => state.activity(timestamp, "Turn started".into(), "active"),
        ("event_msg", "task_complete" | "turn_aborted") => {
            state.activity(timestamp, "Turn ended".into(), "waiting")
        }
        _ => {}
    }
}
