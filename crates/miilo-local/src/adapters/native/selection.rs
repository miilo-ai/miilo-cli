use super::*;
pub(super) async fn resolve(mut chosen: Vec<Value>) -> Result<Vec<Value>> {
    for item in &mut chosen {
        if item["kind"] == "codex" {
            let executable = env::var("MIILO_CODEX_EXECUTABLE").unwrap_or_else(|_| "codex".into());
            let result = discovery::discover(Path::new(&executable), item["id"].as_str())
                .await
                .map_err(|_| "Cannot open the selected Codex session")?;
            if result["thread"]["id"] != item["id"] {
                return Err("Codex returned another session".into());
            }
            let root = env::var_os("CODEX_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home().join(".codex"))
                .join("sessions");
            item["path"] = json!(discovery::authorized_path(
                Path::new(
                    result["thread"]["path"]
                        .as_str()
                        .ok_or("No local Codex transcript")?
                ),
                &root
            )?);
            item["root"] = json!(root);
        } else if item["kind"] == "claude" {
            let available = crate::claude::discovery::sessions(&super::inventory::claude_root())?;
            let found = available
                .iter()
                .find(|s| s["id"] == item["id"])
                .ok_or("Selected Claude session disappeared")?;
            item["path"] = found["path"].clone();
            item["root"] = found["root"].clone();
        } else if item["kind"] == "cursor" {
            let root = env::var_os("MIILO_CURSOR_PROJECTS")
                .map(PathBuf::from)
                .unwrap_or_else(|| home().join(".cursor/projects"));
            let available = crate::cursor::discovery::sessions(&root)?;
            let found = available
                .iter()
                .find(|s| s["id"] == item["id"])
                .ok_or("Selected Cursor session disappeared")?;
            item["path"] = found["path"].clone();
            item["root"] = json!(root);
        } else {
            let kind = item["kind"].as_str().ok_or("Missing session kind")?;
            let (agent, path) = super::session_databases::configured()
                .into_iter()
                .find(|(agent, _)| agent.kind() == kind)
                .ok_or("Unsupported session adapter")?;
            let found = crate::session_database::sessions(&path, agent)?
                .into_iter()
                .find(|s| s["id"] == item["id"])
                .ok_or("Selected database session disappeared")?;
            item["path"] = found["path"].clone();
            item["root"] = found["root"].clone();
        }
    }
    Ok(chosen)
}
pub(super) fn select(request: &Value, available: &Value) -> Result<Vec<Value>> {
    let selected = request["sessions"]
        .as_array()
        .ok_or("Select existing sessions to import")?;
    if selected.is_empty() || selected.len() > 100 {
        return Err("Select between one and 100 sessions".into());
    }
    let available = available["sessions"]
        .as_array()
        .ok_or("Local discovery failed")?;
    let mut chosen = vec![];
    for item in selected {
        let found = available
            .iter()
            .find(|s| s["kind"] == item["kind"] && s["id"] == item["id"])
            .ok_or("A selected session is no longer available; refresh the list")?;
        if found["observable"] != true
            || !matches!(
                found["kind"].as_str(),
                Some("codex" | "cursor" | "opencode" | "devin" | "claude")
            )
        {
            return Err("This agent does not support local import yet".into());
        }
        if !chosen
            .iter()
            .any(|s: &Value| s["id"] == found["id"] && s["kind"] == found["kind"])
        {
            chosen.push(found.clone());
        }
    }
    Ok(chosen)
}
