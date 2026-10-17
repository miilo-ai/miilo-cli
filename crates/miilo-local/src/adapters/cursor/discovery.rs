use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};
/// Enumerate metadata only. No database scraping, credential access, or task execution.
pub fn sessions(root: &Path) -> Result<Vec<Value>, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let mut candidates: Vec<(SystemTime, Value)> = Vec::new();
    let mut visited = 0;
    for project in fs::read_dir(&root)
        .map_err(|e| e.to_string())?
        .flatten()
        .take(2000)
    {
        let directory = project.path().join("agent-transcripts");
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for session in entries.flatten().take(2000) {
            visited += 1;
            if visited > 10_000 {
                break;
            }
            let id = session.file_name().to_string_lossy().into_owned();
            if !valid_id(&id) {
                continue;
            }
            let file = session.path().join(format!("{id}.jsonl"));
            let Ok(file) = authorized_path(&file, &root) else {
                continue;
            };
            let Ok(metadata) = file.metadata() else {
                continue;
            };
            let updated = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            let workspace = project.file_name().to_string_lossy().into_owned();
            candidates.push((updated,json!({"id":id,"workspace":workspace,"path":file,"updatedAt":chrono::DateTime::<chrono::Utc>::from(updated).to_rfc3339(),"createdAt":metadata.created().ok().map(|value|chrono::DateTime::<chrono::Utc>::from(value).to_rfc3339())})));
            candidates.sort_by_key(|(time, _)| std::cmp::Reverse(*time));
            candidates.truncate(100);
        }
        if visited > 10_000 {
            break;
        }
    }
    candidates.sort_by_key(|(time, _)| std::cmp::Reverse(*time));
    Ok(candidates.into_iter().take(100).map(|(_, v)| v).collect())
}
pub fn authorized_path(path: &Path, root: &Path) -> Result<PathBuf, String> {
    let path = path.canonicalize().map_err(|e| e.to_string())?;
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    if !path.starts_with(root) || path.extension().and_then(|v| v.to_str()) != Some("jsonl") {
        return Err("transcript outside Cursor projects".into());
    }
    Ok(path)
}
fn valid_id(id: &str) -> bool {
    id.len() == 36 && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}
