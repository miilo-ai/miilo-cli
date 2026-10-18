use crate::transcript::TranscriptTail as Reader;
use miilo_adapter_sdk::runtime::observation::Observation;
use serde_json::Value;
use std::path::PathBuf;

pub struct TranscriptTail {
    reader: Reader,
    initialized: bool,
}
impl TranscriptTail {
    pub fn new(path: PathBuf) -> Self {
        Self {
            reader: Reader::new(path, apply),
            initialized: false,
        }
    }
    pub fn read(&mut self, state: &mut Observation) -> Result<(), String> {
        self.reader.read(state)?;
        if !self.initialized {
            // Existing history is not proof that a process is still running.
            state.state = "waiting";
            self.initialized = true;
        }
        Ok(())
    }
}
fn apply(row: &Value, state: &mut Observation) {
    if row["type"] != "assistant"
        || row["message"]["role"] != "assistant"
        || row["isSidechain"] == true
        || row["sessionId"].as_str() != Some(&state.thread_id)
    {
        return;
    }
    let Some(at) = row["timestamp"]
        .as_str()
        .filter(|s| chrono::DateTime::parse_from_rfc3339(s).is_ok())
    else {
        return;
    };
    if row["isApiErrorMessage"] == true {
        state.public_summary = Some("Agent reported an error".into());
        state.activity(at, "Agent reported an error".into(), "waiting");
        return;
    }
    let lifecycle = if matches!(
        row["message"]["stop_reason"].as_str(),
        Some("end_turn" | "stop_sequence" | "max_tokens")
    ) {
        "waiting"
    } else {
        "active"
    };
    for content in row["message"]["content"].as_array().into_iter().flatten() {
        match content["type"].as_str() {
            Some("text") => {
                let Some(text) = content["text"].as_str().filter(|s| !s.trim().is_empty()) else {
                    continue;
                };
                let text: String = text
                    .chars()
                    .filter(|c| !c.is_control() || *c == '\n')
                    .take(280)
                    .collect();
                state.public_summary = Some(text.clone());
                state.activity(at, text, lifecycle);
            }
            Some("tool_use") => {
                let name: String = content["name"]
                    .as_str()
                    .unwrap_or("tool")
                    .chars()
                    .filter(|c| c.is_alphanumeric() || "_./-".contains(*c))
                    .take(80)
                    .collect();
                state.activity(at, format!("Tool started: {name}"), "active");
            }
            _ => {} // Exclude thinking, signatures, tool payloads and unknown blocks.
        }
    }
}
