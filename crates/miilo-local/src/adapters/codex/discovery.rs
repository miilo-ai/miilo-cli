use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
    time::{Duration, timeout},
};

/// Only discovery/read methods are exposed. No resume, turn/start, or control RPC.
pub async fn discover(executable: &Path, thread_id: Option<&str>) -> Result<Value, String> {
    let mut child = Command::new(executable)
        .arg("app-server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut input = child.stdin.take().ok_or("missing stdin")?;
    let mut lines = BufReader::new(child.stdout.take().ok_or("missing stdout")?);
    let operation = async {
        input.write_all(format!("{}\n", json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"miilo_readonly_observer","version":env!("CARGO_PKG_VERSION")}}})).as_bytes()).await.map_err(|e| e.to_string())?;
        response(&mut lines, 1).await?;
        input
            .write_all(b"{\"method\":\"initialized\"}\n")
            .await
            .map_err(|e| e.to_string())?;
        let (method, params) = match thread_id {
            Some(id) => ("thread/read", json!({"threadId":id})),
            None => ("thread/list", json!({"limit":100,"sortKey":"updated_at"})),
        };
        input
            .write_all(format!("{}\n", json!({"id":2,"method":method,"params":params})).as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        response(&mut lines, 2).await
    };
    let result = timeout(Duration::from_secs(20), operation)
        .await
        .map_err(|_| "Codex discovery timed out".to_owned())?;
    let _ = child.kill().await;
    result
}
async fn response<R: tokio::io::AsyncBufRead + Unpin>(
    lines: &mut R,
    id: u64,
) -> Result<Value, String> {
    for _ in 0..1000 {
        let mut line = Vec::new();
        loop {
            let available = lines
                .fill_buf()
                .await
                .map_err(|_| "Cannot read Codex discovery")?;
            if available.is_empty() {
                return Err("Codex stopped during discovery".into());
            }
            let count = available
                .iter()
                .position(|b| *b == b'\n')
                .map_or(available.len(), |n| n + 1);
            if line.len() + count > 2_000_000 {
                return Err("Codex discovery response exceeded limit".into());
            }
            let complete = available[count - 1] == b'\n';
            line.extend_from_slice(&available[..count]);
            lines.consume(count);
            if complete {
                break;
            }
        }
        let value: Value =
            serde_json::from_slice(&line).map_err(|_| "Invalid Codex discovery response")?;
        if value["id"] == id {
            return value
                .get("result")
                .cloned()
                .ok_or_else(|| "Codex rejected read-only discovery".into());
        }
    }
    Err("Codex stopped during discovery".into())
}
/// Resolve only the selected session returned by Codex, beneath the user's session root.
pub fn authorized_path(path: &Path, session_root: &Path) -> Result<PathBuf, String> {
    let root = session_root.canonicalize().map_err(|e| e.to_string())?;
    let selected = path.canonicalize().map_err(|e| e.to_string())?;
    if !selected.starts_with(root) || selected.extension().and_then(|v| v.to_str()) != Some("jsonl")
    {
        return Err("selected transcript is outside Codex sessions".into());
    }
    Ok(selected)
}
