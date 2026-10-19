//! Standalone observation engine. Remote CE publication is an explicit legacy adapter.
pub mod application;
#[path = "adapters/archive/mod.rs"]
pub mod archive;
pub mod domain;
#[cfg(unix)]
#[path = "adapters/native/mod.rs"]
mod engine;
#[cfg(unix)]
pub async fn execute(args: &[String]) -> Result<(), String> {
    if args.first().is_some_and(|a| a == "archive") {
        return archive::execute(&args[1..]);
    }
    engine::execute(args).await
}
#[cfg(not(unix))]
pub async fn execute(args: &[String]) -> Result<(), String> {
    if args.first().is_some_and(|a| a == "archive") {
        return archive::execute(&args[1..]);
    }
    Err("Local transcript observation currently requires Linux or macOS; remote Miilo commands work on this platform.".into())
}
#[cfg(unix)]
#[path = "adapters/claude/mod.rs"]
mod claude;
#[cfg(unix)]
#[path = "adapters/codex/mod.rs"]
mod codex;
#[cfg(unix)]
#[path = "adapters/cursor/mod.rs"]
mod cursor;
#[cfg(unix)]
#[path = "adapters/grok/mod.rs"]
mod grok;
#[cfg(unix)]
#[path = "adapters/transcript.rs"]
mod transcript;

/// Read-only infrastructure adapters. Callers own explicit consent and path selection.
#[cfg(unix)]
pub mod adapters {
    pub use crate::claude::discovery as claude_discovery;
    pub use crate::claude::transcript::TranscriptTail as ClaudeTranscriptTail;
    pub use crate::codex::discovery as codex_discovery;
    pub use crate::codex::transcript::TranscriptTail as CodexTranscriptTail;
    pub use crate::cursor::discovery as cursor_discovery;
    pub use crate::cursor::transcript::apply as apply_cursor_activity;
    pub use crate::grok::discovery as grok_discovery;
    pub use crate::grok::transcript::TranscriptTail as GrokTranscriptTail;
    pub use crate::transcript::TranscriptTail;
}

#[cfg(unix)]
#[path = "adapters/session_database/mod.rs"]
pub mod session_database;
