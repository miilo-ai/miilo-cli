//! Version-qualified local Claude Code metadata; never execute or resume Claude.
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Read},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    time::SystemTime,
};

pub fn authorized_path(path: &Path, root: &Path) -> Result<PathBuf, String> {
    if fs::symlink_metadata(path)
        .map_err(|_| "Transcript unavailable")?
        .file_type()
        .is_symlink()
    {
        return Err("Claude transcript must not be a symlink".into());
    }
    let selected = path.canonicalize().map_err(|_| "Transcript unavailable")?;
    let root = root
        .canonicalize()
        .map_err(|_| "Claude projects unavailable")?;
    let relative = selected
        .strip_prefix(root)
        .map_err(|_| "Transcript outside Claude projects")?;
    let id = selected.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    if relative.components().count() != 2
        || selected.extension().and_then(|s| s.to_str()) != Some("jsonl")
        || !valid_id(id)
        || !selected.is_file()
    {
        return Err("Select a main Claude Code session transcript".into());
    }
    Ok(selected)
}
fn valid_id(id: &str) -> bool {
    id.len() == 36
        && id.chars().enumerate().all(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                c == '-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}
pub fn sessions(root: &Path) -> Result<Vec<Value>, String> {
    let root = root
        .canonicalize()
        .map_err(|_| "Claude projects unavailable")?;
    let mut files = vec![];
    let mut visited = 0;
    for project in fs::read_dir(&root)
        .map_err(|_| "Cannot list Claude projects")?
        .flatten()
        .take(2000)
    {
        if !project.file_type().is_ok_and(|t| t.is_dir()) {
            continue;
        }
        let Ok(entries) = fs::read_dir(project.path()) else {
            continue;
        };
        for entry in entries.flatten().take(2000) {
            visited += 1;
            if visited > 10_000 {
                break;
            }
            let Ok(path) = authorized_path(&entry.path(), &root) else {
                continue;
            };
            let modified = path
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            files.push((modified, path));
            files.sort_by_key(|(at, _)| std::cmp::Reverse(*at));
            files.truncate(100);
        }
        if visited > 10_000 {
            break;
        }
    }
    let mut sessions = vec![];
    for (modified, path) in files {
        let id = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let mut cwd = None;
        let mut created = None;
        // Only inspect a bounded prefix for session identity/project metadata.
        let file = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&path)
            .map_err(|_| "Cannot read Claude metadata")?;
        if !file.metadata().is_ok_and(|m| m.is_file()) {
            continue;
        }
        for line in BufReader::new(file.take(512 * 1024))
            .lines()
            .take(256)
            .flatten()
        {
            let Ok(row) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            if row["sessionId"] != id || row["isSidechain"] == true {
                continue;
            }
            if cwd.is_none() {
                cwd = row["cwd"]
                    .as_str()
                    .filter(|s| Path::new(s).is_absolute() && s.len() <= 4096)
                    .map(str::to_owned);
            }
            if created.is_none() {
                created = row["timestamp"]
                    .as_str()
                    .filter(|s| chrono::DateTime::parse_from_rfc3339(s).is_ok())
                    .map(str::to_owned);
            }
            if cwd.is_some() && created.is_some() {
                break;
            }
        }
        let Some(cwd) = cwd else {
            continue;
        };
        let name = Path::new(&cwd)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Local session")
            .chars()
            .take(120)
            .collect::<String>();
        sessions.push(json!({"kind":"claude","id":id,"name":name,"cwd":cwd,"createdAt":created,"updatedAt":chrono::DateTime::<chrono::Utc>::from(modified).to_rfc3339(),"observable":true,"path":path,"root":root}));
    }
    Ok(sessions)
}
