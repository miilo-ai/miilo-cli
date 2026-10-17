//! Folder boundaries and project-name patterns shared by discovery and observation.
use super::*;

pub fn validate(value: &Value) -> Result<Vec<Value>> {
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
        if !matches!(rule["kind"].as_str(), Some("codex" | "cursor" | "all")) {
            return Err("Choose Codex, Cursor, or all supported agents".into());
        }
        if !Path::new(folder).is_absolute()
            || folder.contains(['*', '?'])
            || !Path::new(folder).is_dir()
        {
            return Err("Choose an existing absolute folder for the group".into());
        }
        if pattern.len() > 120
            || pattern.contains(['/', '\\'])
            || pattern.chars().any(char::is_control)
        {
            return Err("Use a project name pattern such as vave-* (no path separators)".into());
        }
        if !rule["future"].is_boolean() {
            return Err("Choose whether to include future matches".into());
        }
    }
    Ok(rules.clone())
}
fn wildcard(pattern: &str, name: &str) -> bool {
    // Bounded dynamic programming: '*' and '?' only, no regex or shell evaluation.
    let chars: Vec<_> = name.chars().collect();
    let mut row = vec![false; chars.len() + 1];
    row[0] = true;
    for p in pattern.chars() {
        let mut next = vec![false; chars.len() + 1];
        if p == '*' {
            next[0] = row[0];
        }
        for j in 1..=chars.len() {
            next[j] = if p == '*' {
                row[j] || next[j - 1]
            } else {
                row[j - 1] && (p == '?' || p == chars[j - 1])
            };
        }
        row = next;
    }
    row[chars.len()]
}
pub fn matches(rule: &Value, session: &Value) -> bool {
    if session["observable"] != true
        || !matches!(session["kind"].as_str(), Some("codex" | "cursor"))
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
    let Ok(folder) = fs::canonicalize(rule["folder"].as_str().unwrap_or("")) else {
        return false;
    };
    let Ok(cwd) = fs::canonicalize(cwd) else {
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
/// A discovery window is not a complete historical inventory. Unknown creation
/// times can never authorize future enrollment of an unseen existing session.
pub(super) fn future_matches(rule: &Value, session: &Value) -> bool {
    if rule["future"] != true || !matches(rule, session) {
        return false;
    }
    let Some(cutoff) = rule["enrolledAfter"]
        .as_str()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
    else {
        return false;
    };
    let created = session["createdAt"]
        .as_i64()
        .and_then(|n| chrono::DateTime::from_timestamp(n, 0))
        .or_else(|| {
            session["createdAt"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|v| v.with_timezone(&chrono::Utc))
        });
    created.is_some_and(|at| {
        at > cutoff.with_timezone(&chrono::Utc)
            && at <= chrono::Utc::now() + chrono::Duration::seconds(5)
    })
}
pub(super) fn bind_consent(rules: &mut [Value], previous: &Value) {
    for rule in rules {
        let existing = previous.as_array().into_iter().flatten().find(|old| {
            old["name"] == rule["name"]
                && old["folder"] == rule["folder"]
                && old["pattern"] == rule["pattern"]
                && old["kind"] == rule["kind"]
                && old["future"] == true
        });
        rule["enrolledAfter"] = if rule["future"] == true {
            existing
                .and_then(|old| old["enrolledAfter"].as_str())
                .map(|s| json!(s))
                .unwrap_or_else(|| json!(chrono::Utc::now().to_rfc3339()))
        } else {
            Value::Null
        };
    }
}
pub fn preview(sessions: &[Value], rules: &[Value]) -> Value {
    let mut groups: Vec<Value> = rules
        .iter()
        .map(|rule| {
            let items: Vec<_> = sessions
                .iter()
                .filter(|s| matches(rule, s))
                .cloned()
                .collect();
            json!({"name":rule["name"],"rule":rule,"sessions":items})
        })
        .collect();
    let mut remaining = std::collections::BTreeMap::<String, Vec<Value>>::new();
    for session in sessions {
        if rules.iter().any(|rule| matches(rule, session)) {
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
pub fn labels(session: &Value, rules: &[Value]) -> Vec<String> {
    rules
        .iter()
        .filter(|r| matches(r, session))
        .filter_map(|r| r["name"].as_str().map(str::to_owned))
        .collect()
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
