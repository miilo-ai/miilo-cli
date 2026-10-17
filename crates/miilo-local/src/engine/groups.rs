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
mod tests {
    use super::*;
    #[test]
    fn patterns_are_literal_except_star_and_question_mark() {
        assert!(wildcard("vave-*", "vave-ai"));
        assert!(wildcard("miilo-?e", "miilo-ce"));
        assert!(!wildcard("vave-*", "miilo-ce"));
        assert!(!wildcard("[ab]", "a"));
        assert!(wildcard("*", ""));
        assert!(!wildcard("?", ""));
    }
    #[test]
    fn folders_are_scoped_and_nested_projects_match() {
        let temporary = tempfile::tempdir().unwrap();
        let base = fs::canonicalize(temporary.path()).unwrap();
        let work = base.join("Work");
        let project = work.join("vave-ai/src");
        let other = base.join("Work-other/vave-ai");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&other).unwrap();
        let rule =
            json!({"name":"Vave","folder":work,"pattern":"vave-*","kind":"codex","future":false});
        assert!(validate(&json!([rule.clone()])).is_ok());
        let session = json!({"kind":"codex","observable":true,"cwd":project});
        assert!(matches(&rule, &session));
        assert!(!matches(
            &rule,
            &json!({"cwd":other,"kind":"codex","observable":true})
        ));
        let link = work.join("vave-link");
        std::os::unix::fs::symlink(&other, &link).unwrap();
        assert!(!matches(
            &rule,
            &json!({"cwd":link,"kind":"codex","observable":true})
        ));
        let encoded = project
            .to_string_lossy()
            .trim_start_matches('/')
            .replace('/', "-");
        assert_eq!(cursor_workspace(&work, &encoded), Some(project.clone()));
        assert_eq!(cursor_workspace(&other, &encoded), None);
        assert!(!matches(
            &rule,
            &json!({"cwd":project,"kind":"cursor","observable":true})
        ));
        assert!(!matches(
            &rule,
            &json!({"cwd":project,"kind":"codex","observable":false})
        ));
        assert!(validate(&json!([rule.clone(), rule.clone()])).is_err());
        let preview = preview(&[session], &[rule]);
        assert_eq!(
            preview["groups"][0]["sessions"].as_array().unwrap().len(),
            1
        );
    }
}
