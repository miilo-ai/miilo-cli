//! Filesystem and command adapters for the shared grouping application.
use super::*;
use crate::application::{groups as core, ports::FolderMetadata};
use crate::domain::group_policy;
#[cfg(test)]
use crate::domain::group_policy::wildcard;
struct Folders;
impl FolderMetadata for Folders {
    fn canonical(&self, path: &str) -> Option<PathBuf> {
        fs::canonicalize(path).ok()
    }
    fn is_directory(&self, path: &str) -> bool {
        Path::new(path).is_dir()
    }
}
pub fn validate(value: &Value) -> Result<Vec<Value>> {
    core::validate(value, &Folders)
}
pub fn matches(rule: &Value, session: &Value) -> bool {
    core::matches(rule, session, &Folders)
}
pub fn preview(sessions: &[Value], rules: &[Value]) -> Value {
    core::preview(sessions, rules, &Folders)
}
pub fn labels(session: &Value, rules: &[Value]) -> Vec<String> {
    core::labels(session, rules, &Folders)
}
pub(super) fn future_matches(rule: &Value, session: &Value) -> bool {
    group_policy::future_matches(rule, session, matches(rule, session), chrono::Utc::now())
}
pub(super) fn bind_consent(rules: &mut [Value], previous: &Value) {
    group_policy::bind_consent(rules, previous, chrono::Utc::now())
}
pub fn cursor_workspace(scope: &Path, encoded: &str) -> Option<PathBuf> {
    // Cursor's folder name is a lossy path encoding. Match existing directory
    // metadata rather than guessing that every '-' was a path separator.
    let scope = fs::canonicalize(scope).ok()?;
    let encode = |p: &Path| {
        p.to_string_lossy()
            .trim_start_matches('/')
            .replace('/', "-")
    };
    let prefix = encode(&scope);
    if encoded != prefix && !encoded.starts_with(&(prefix + "-")) {
        return None;
    }
    let mut pending = vec![(scope, 0)];
    let mut found = None;
    let mut visited = 0;
    while let Some((path, depth)) = pending.pop() {
        visited += 1;
        if visited > 3000 {
            return None;
        }
        let slug = encode(&path);
        if slug == encoded {
            if found.is_some() {
                return None;
            }
            found = Some(path.clone());
        }
        if depth >= 8 || !encoded.starts_with(&(slug + "-")) {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&path) {
            for entry in entries.flatten() {
                if entry.file_name().to_string_lossy().starts_with('.') {
                    continue;
                }
                if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                    pending.push((entry.path(), depth + 1));
                }
            }
        }
    }
    found
}
pub async fn execute() -> Result<()> {
    use std::io::BufRead;
    let mut input = String::new();
    io::stdin()
        .lock()
        .take(2_000_001)
        .read_line(&mut input)
        .map_err(|_| "Cannot read group preview")?;
    if input.len() > 2_000_000 {
        return Err("Group preview too large".into());
    }
    let request: Value = serde_json::from_str(&input).map_err(|_| "Invalid group preview")?;
    let rules = validate(&request["groups"])?;
    let sessions = request["sessions"]
        .as_array()
        .ok_or("No sessions to group")?;
    println!("{}", preview(sessions, &rules));
    Ok(())
}
#[cfg(test)]
#[path = "groups_tests.rs"]
mod tests;
