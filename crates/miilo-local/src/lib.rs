//! Standalone observation engine. Remote CE publication is an explicit legacy adapter.
#[cfg(unix)]
mod engine;
#[cfg(unix)]
pub use engine::execute;
#[cfg(not(unix))]
pub async fn execute(_: &[String]) -> Result<(), String> {
    Err("Local transcript observation currently requires Linux or macOS; remote Miilo commands work on this platform.".into())
}
#[cfg(unix)]
mod codex {
    pub mod discovery;
    pub mod transcript;
}
#[cfg(unix)]
mod cursor {
    pub mod discovery;
    pub mod transcript;
}
#[cfg(unix)]
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
