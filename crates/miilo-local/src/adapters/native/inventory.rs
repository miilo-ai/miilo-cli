use super::*;
pub(super) async fn inventory(directory: &Path) -> Value {
    let mut sessions = vec![];
    let mut errors = vec![];
    let executable = env::var("MIILO_CODEX_EXECUTABLE").unwrap_or_else(|_| "codex".into());
    match discovery::discover(Path::new(&executable), None).await {
        Ok(result) => {
            for thread in result["data"].as_array().into_iter().flatten() {
                sessions.push(json!({"kind":"codex","id":thread["id"],"name":thread["name"],"cwd":thread["cwd"],"updatedAt":thread["updatedAt"],"createdAt":thread["createdAt"],"observable":true}));
            }
        }
        Err(_) => errors.push("Codex discovery unavailable"),
    }
    let cursor = env::var_os("MIILO_CURSOR_PROJECTS")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".cursor/projects"));
    if cursor.exists() {
        match crate::cursor::discovery::sessions(&cursor) {
            Ok(items) => {
                for item in items {
                    let cwd = groups::cursor_workspace(
                        directory,
                        item["workspace"].as_str().unwrap_or(""),
                    );
                    sessions.push(json!({"kind":"cursor","id":item["id"],"name":item["workspace"],"cwd":cwd.map(|p|json!(p)).unwrap_or(item["workspace"].clone()),"updatedAt":item["updatedAt"],"createdAt":item["createdAt"],"observable":true}));
                }
            }
            Err(_) => errors.push("Cursor discovery unavailable"),
        }
    }
    for (agent, path) in super::session_databases::configured() {
        if path.exists() {
            match crate::session_database::sessions(&path, agent) {
                Ok(items) => sessions.extend(items),
                Err(_) => errors.push(if agent.kind() == "devin" {
                    "Devin local discovery unavailable"
                } else {
                    "OpenCode local discovery unavailable"
                }),
            }
        }
    }
    let devin = json!({"available":super::session_databases::configured().iter().any(|(agent,path)|agent.kind()=="devin"&&path.is_file()),"observationSupported":true,"scope":"local sessions only"});
    json!({"sessions":sessions,"errors":errors,"devin":devin,"scope":directory,"readOnly":true})
}
