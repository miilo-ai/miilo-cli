use super::ports::{FolderMetadata, Result};
use crate::domain::group_policy::wildcard;
use serde_json::{Value, json};
use std::path::Path;

pub fn validate(value: &Value, folders: &dyn FolderMetadata) -> Result<Vec<Value>> {
    let Some(rules) = value.as_array() else {
        return if value.is_null() {
            Ok(vec![])
        } else {
            Err("Invalid group rules".into())
        };
    };
    if rules.len() > 30 {
        return Err("At most 30 groups are supported".into());
    }
    let mut names = std::collections::BTreeSet::new();
    for rule in rules {
        let name = rule["name"].as_str().unwrap_or("").trim();
        let folder = rule["folder"].as_str().unwrap_or("");
        let pattern = rule["pattern"].as_str().unwrap_or("");
        if name.is_empty()
            || name.len() > 80
            || name.chars().any(char::is_control)
            || !names.insert(name.to_owned())
        {
            return Err("Give each group a unique name (up to 80 characters)".into());
        }
        if !matches!(
            rule["kind"].as_str(),
            Some("codex" | "cursor" | "opencode" | "devin" | "all")
        ) {
            return Err("Choose Codex, Cursor, OpenCode, Devin, or all supported agents".into());
        }
        if !Path::new(folder).is_absolute()
            || folder.contains(['*', '?'])
            || !folders.is_directory(folder)
        {
            return Err("Choose an existing absolute folder for the group".into());
        }
        if pattern.len() > 120
            || pattern.contains(['/', '\\'])
            || pattern.chars().any(char::is_control)
        {
            return Err("Use a project name pattern such as project-* (no path separators)".into());
        }
        if !rule["future"].is_boolean() {
            return Err("Choose whether to include future matches".into());
        }
    }
    Ok(rules.clone())
}
pub fn matches(rule: &Value, session: &Value, folders: &dyn FolderMetadata) -> bool {
    if session["observable"] != true
        || !matches!(
            session["kind"].as_str(),
            Some("codex" | "cursor" | "opencode" | "devin")
        )
    {
        return false;
    }
    if rule["kind"] != "all" && rule["kind"] != session["kind"] {
        return false;
    }
    let Some(cwd) = session["cwd"].as_str() else {
        return false;
    };
    // Canonical paths prevent a symlink or '..' from escaping an opted-in folder.
    let Some(folder) = folders.canonical(rule["folder"].as_str().unwrap_or("")) else {
        return false;
    };
    let Some(cwd) = folders.canonical(cwd) else {
        return false;
    };
    let Ok(relative) = cwd.strip_prefix(&folder) else {
        return false;
    };
    let pattern = rule["pattern"].as_str().unwrap_or("");
    pattern.is_empty()
        || relative
            .components()
            .any(|part| wildcard(pattern, &part.as_os_str().to_string_lossy()))
        || (relative.as_os_str().is_empty()
            && folder
                .file_name()
                .is_some_and(|name| wildcard(pattern, &name.to_string_lossy())))
}
pub fn preview(sessions: &[Value], rules: &[Value], folders: &dyn FolderMetadata) -> Value {
    let mut groups: Vec<Value> = rules
        .iter()
        .map(|rule| {
            let items: Vec<_> = sessions
                .iter()
                .filter(|s| matches(rule, s, folders))
                .cloned()
                .collect();
            json!({"name":rule["name"],"rule":rule,"sessions":items})
        })
        .collect();
    let mut remaining = std::collections::BTreeMap::<String, Vec<Value>>::new();
    for session in sessions {
        if rules.iter().any(|rule| matches(rule, session, folders)) {
            continue;
        }
        let cwd = session["cwd"].as_str().unwrap_or("Unknown folder");
        let name = format!("{} / {}", session["kind"].as_str().unwrap_or("agent"), cwd);
        remaining.entry(name).or_default().push(session.clone());
    }
    groups.extend(
        remaining
            .into_iter()
            .map(|(name, sessions)| json!({"name":name,"rule":null,"sessions":sessions})),
    );
    json!({"groups":groups})
}
pub fn labels(session: &Value, rules: &[Value], folders: &dyn FolderMetadata) -> Vec<String> {
    rules
        .iter()
        .filter(|r| matches(r, session, folders))
        .filter_map(|r| r["name"].as_str().map(str::to_owned))
        .collect()
}
