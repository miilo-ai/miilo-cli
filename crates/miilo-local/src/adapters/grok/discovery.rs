use serde_json::{Value, json};
use std::{
    fs,
    io::Read,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    time::SystemTime,
};
pub fn authorized_path(path: &Path, root: &Path) -> Result<PathBuf, String> {
    if fs::symlink_metadata(path)
        .map_err(|_| "Grok history unavailable")?
        .file_type()
        .is_symlink()
    {
        return Err("Grok history must not be a symlink".into());
    }
    let path = path
        .canonicalize()
        .map_err(|_| "Grok history unavailable")?;
    let root = root
        .canonicalize()
        .map_err(|_| "Grok sessions unavailable")?;
    if !path.starts_with(&root)
        || path
            .strip_prefix(root)
            .map_err(|_| "Foreign Grok history")?
            .components()
            .count()
            != 3
        || path.file_name().and_then(|s| s.to_str()) != Some("chat_history.jsonl")
        || !path.is_file()
    {
        return Err("Select a Grok main session history".into());
    }
    Ok(path)
}
pub fn sessions(root: &Path) -> Result<Vec<Value>, String> {
    let root = root
        .canonicalize()
        .map_err(|_| "Grok sessions unavailable")?;
    let mut candidates = vec![];
    let mut visited = 0;
    for workspace in fs::read_dir(&root)
        .map_err(|_| "Cannot list Grok sessions")?
        .flatten()
        .take(2000)
    {
        if !workspace.file_type().is_ok_and(|t| t.is_dir()) {
            continue;
        }
        let Ok(entries) = fs::read_dir(workspace.path()) else {
            continue;
        };
        for session in entries.flatten().take(2000) {
            visited += 1;
            if visited > 10000 {
                break;
            }
            if !session.file_type().is_ok_and(|t| t.is_dir()) {
                continue;
            }
            let Ok(path) = authorized_path(&session.path().join("chat_history.jsonl"), &root)
            else {
                continue;
            };
            let modified = path
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            candidates.push((modified, path));
            candidates.sort_by_key(|(at, _)| std::cmp::Reverse(*at));
            candidates.truncate(100);
        }
        if visited > 10000 {
            break;
        }
    }
    let mut result = vec![];
    for (modified, path) in candidates {
        let parent = path.parent().ok_or("Missing Grok session directory")?;
        let Ok(file) = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(parent.join("summary.json"))
        else {
            continue;
        };
        if !file
            .metadata()
            .is_ok_and(|m| m.is_file() && m.len() <= 65536)
        {
            continue;
        }
        let Ok(summary) = serde_json::from_reader::<_, Value>(file.take(65537)) else {
            continue;
        };
        if summary["chat_format_version"] != 1 {
            continue;
        }
        let Some(id) = summary["info"]["id"]
            .as_str()
            .filter(|s| s.len() <= 128 && parent.file_name().and_then(|n| n.to_str()) == Some(s))
        else {
            continue;
        };
        let Some(cwd) = summary["info"]["cwd"]
            .as_str()
            .filter(|s| s.len() <= 4096 && Path::new(s).is_absolute())
        else {
            continue;
        };
        let name = Path::new(cwd)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Local session")
            .chars()
            .take(120)
            .collect::<String>();
        result.push(json!({"kind":"grok","id":id,"cwd":cwd,"name":name,"path":path,"root":root,"observable":true,"createdAt":summary["created_at"],"updatedAt":chrono::DateTime::<chrono::Utc>::from(modified).to_rfc3339()}));
    }
    Ok(result)
}
