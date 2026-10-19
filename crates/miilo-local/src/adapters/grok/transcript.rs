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
            state.state = "waiting";
            self.initialized = true;
        }
        Ok(())
    }
}
fn apply(row: &Value, state: &mut Observation) {
    if row["type"] != "assistant" {
        return;
    }
    // Format 1 stores visible assistant replies as strings. All other shapes
    // remain excluded, including reasoning, user prompts and tool payloads.
    let Some(text) = row["content"].as_str().filter(|s| !s.trim().is_empty()) else {
        return;
    };
    let text = text
        .chars()
        .filter(|c| !c.is_control() || *c == '\n')
        .take(280)
        .collect::<String>();
    let at = chrono::Utc::now().to_rfc3339();
    state.public_summary = Some(text.clone());
    state.activity(&at, text, "active");
}
