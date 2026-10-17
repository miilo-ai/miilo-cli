//! Standalone observation engine. Remote CE publication is an explicit legacy adapter.
pub mod application;
pub mod domain;
#[cfg(unix)]
#[path = "adapters/native/mod.rs"]
mod engine;
#[cfg(unix)]
pub use engine::execute;
#[cfg(not(unix))]
pub async fn execute(_: &[String]) -> Result<(), String> {
    Err("Local transcript observation currently requires Linux or macOS; remote Miilo commands work on this platform.".into())
}
#[cfg(unix)]
#[path = "adapters/codex/mod.rs"]
mod codex;
#[cfg(unix)]
#[path = "adapters/cursor/mod.rs"]
mod cursor;
#[cfg(unix)]
#[path = "adapters/transcript.rs"]
mod transcript;

/// Read-only infrastructure adapters. Callers own explicit consent and path selection.
#[cfg(unix)]
pub mod adapters {
    pub use crate::codex::discovery as codex_discovery;
    pub use crate::codex::transcript::TranscriptTail as CodexTranscriptTail;
    pub use crate::cursor::discovery as cursor_discovery;
    pub use crate::cursor::transcript::apply as apply_cursor_activity;
    pub use crate::transcript::TranscriptTail;
}
