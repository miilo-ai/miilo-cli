use crate::{
    Result,
    adapters::{client, server::Shared},
    domain::Invitation,
};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
};

pub async fn serve(path: PathBuf, shared: Arc<Shared>) -> Result<tokio::task::JoinHandle<()>> {
    if path.exists() {
        if UnixStream::connect(&path).await.is_ok() {
            return Err("Host control socket already active".into());
        }
        std::fs::remove_file(&path).map_err(|_| "Cannot replace stale control socket")?;
    }
    let listener =
        UnixListener::bind(path.clone()).map_err(|_| "Cannot open host control socket")?;
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
        .map_err(|_| "Cannot protect control socket")?;
    Ok(tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            // One bounded command at a time. The private directory/socket is owner-only.
            let _ = tokio::time::timeout(std::time::Duration::from_secs(3), async {
                let mut line = String::new();
                use tokio::io::AsyncReadExt;
                let read = BufReader::new((&mut socket).take(8193))
                    .read_line(&mut line)
                    .await;
                let response = if read.is_ok() && line.len() <= 8192 {
                    serde_json::from_str::<Value>(&line)
                        .map_err(|_| "Invalid command".into())
                        .and_then(|v| command(&shared, &v))
                } else {
                    Err("Control command too large".into())
                };
                let value = match response {
                    Ok(value) => json!({"ok":value}),
                    Err(error) => json!({"error":error}),
                };
                let _ = socket.write_all(format!("{value}\n").as_bytes()).await;
            })
            .await;
        }
    }))
}
fn command(s: &Shared, value: &Value) -> Result<Value> {
    let mut host = s.host.lock().map_err(|_| "Host state unavailable")?;
    match value["command"].as_str().unwrap_or("") {
        "invite" => {
            let (secret, expires_at) = host.invite();
            let i = Invitation {
                version: 1,
                host_id: s.identity.id.clone(),
                name: s.identity.name.clone(),
                url: s.identity.url.clone(),
                certificate_pem: s.identity.certificate.clone(),
                fingerprint: s.identity.fingerprint(),
                secret,
                expires_at,
            };
            Ok(json!({"invitation":client::encode(&i)?,"expiresAt":expires_at,"readOnly":true}))
        }
        "requests" => {
            use crate::application::Clock;
            Ok(json!(
                host.pending
                    .iter()
                    .filter(|p| p.expires_at > host.clock.now() && p.token.is_none() && !p.denied)
                    .map(
                        |p| json!({"id":p.id,"name":p.name,"code":p.code,"expiresAt":p.expires_at})
                    )
                    .collect::<Vec<_>>()
            ))
        }
        "approve" | "deny" => {
            host.decide(
                value["id"].as_str().unwrap_or(""),
                value["code"].as_str().unwrap_or(""),
                value["command"] == "approve",
            )?;
            Ok(json!({"status":"saved"}))
        }
        "devices" => Ok(json!(
            host.ledger
                .devices
                .iter()
                .map(|d| json!({"id":d.id,"name":d.name,"expiresAt":d.expires_at,"readOnly":true}))
                .collect::<Vec<_>>()
        )),
        "revoke" => {
            host.revoke(value["id"].as_str().unwrap_or(""))?;
            Ok(json!({"status":"revoked"}))
        }
        _ => Err("Unknown local pairing command".into()),
    }
}
pub async fn request(directory: &Path, command: Value) -> Result<Value> {
    let mut socket = UnixStream::connect(directory.join("control.sock"))
        .await
        .map_err(|_| "Start `miilo local share` first")?;
    socket
        .write_all(format!("{command}\n").as_bytes())
        .await
        .map_err(|_| "Cannot send host command")?;
    use tokio::io::AsyncReadExt;
    let mut line = String::new();
    tokio::time::timeout(
        std::time::Duration::from_secs(4),
        BufReader::new(socket.take(32769)).read_line(&mut line),
    )
    .await
    .map_err(|_| "Host command timed out")?
    .map_err(|_| "Cannot read host response")?;
    if line.len() > 32768 {
        return Err("Host response too large".into());
    }
    let value: Value = serde_json::from_str(&line).map_err(|_| "Invalid host response")?;
    if value.get("error").is_some() {
        return Err(value["error"]
            .as_str()
            .unwrap_or("Host command failed")
            .into());
    }
    Ok(value["ok"].clone())
}
