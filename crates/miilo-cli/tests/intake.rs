use std::process::Command;
fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_miilo"))
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn manual_import_list_accept_and_replay_without_a_server() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state");
    let path = path.to_str().unwrap();
    let file = root.path().join("submission.json");
    std::fs::write(&file,r#"{"id":"task-1","title":"Release checklist","summary":"Run configured checks","revision":"one","workflowId":"release"}"#).unwrap();
    let base = ["inbox", "--state", path];
    let mut args = base.to_vec();
    args.extend(["import", file.to_str().unwrap()]);
    assert!(run(&args).status.success());
    args = base.to_vec();
    args.push("list");
    let result = run(&args);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let revision = value["items"][0]["revision"].as_str().unwrap();
    let now = chrono::Utc::now().to_rfc3339();
    args = base.to_vec();
    args.extend([
        "decide",
        "manual-intake/task-1",
        "--action",
        "accept",
        "--revision",
        revision,
        "--request-id",
        "request-1",
        "--confirmed-at",
        &now,
    ]);
    let first = run(&args);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let second = run(&args);
    assert!(second.status.success());
    assert_eq!(first.stdout, second.stdout);
    let receipt: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(receipt["providerState"], "queued_awaiting_executor");
}

struct Server(std::process::Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
#[tokio::test]
async fn remote_client_observes_import_and_decision_through_real_loopback_api() {
    use futures_util::StreamExt;
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state");
    let token = root.path().join("token");
    std::fs::write(
        &token,
        "synthetic-test-token-not-a-real-credential-123456789",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&token, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let socket = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = socket.local_addr().unwrap();
    drop(socket);
    let _server = Server(
        Command::new(env!("CARGO_BIN_EXE_miilo"))
            .args([
                "inbox",
                "--state",
                path.to_str().unwrap(),
                "serve",
                "--bind",
                &address.to_string(),
                "--access-token-file",
                token.to_str().unwrap(),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap(),
    );
    let endpoint = format!("http://{address}");
    let client = miilo_client::MiiloClient::new(
        &endpoint,
        Some("synthetic-test-token-not-a-real-credential-123456789".into()),
    )
    .unwrap();
    let mut ready = false;
    for _ in 0..50 {
        if client.health().await.is_ok() {
            ready = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(ready);
    let unauthorized = miilo_client::MiiloClient::new(&endpoint, None)
        .unwrap()
        .snapshot()
        .await
        .unwrap_err();
    assert!(miilo_client::is_authentication_error(&unauthorized));
    let mut updates = client.observe().await.unwrap();
    let initial = updates.next().await.unwrap().unwrap();
    assert!(initial.snapshot.work_items.is_empty());
    let file = root.path().join("submission.json");
    std::fs::write(&file,r#"{"id":"task-2","title":"Review release","summary":"Synthetic API validation","revision":"one","workflowId":"release"}"#).unwrap();
    assert!(
        run(&[
            "inbox",
            "--state",
            path.to_str().unwrap(),
            "import",
            file.to_str().unwrap()
        ])
        .status
        .success()
    );
    let update = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let value = updates.next().await.unwrap().unwrap();
            if !value.snapshot.work_items.is_empty() {
                break value;
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(update.snapshot.work_items[0].title, "Review release");
    let item = &update.snapshot.work_items[0];
    let action = miilo_adapter_sdk::ActionInvocation {
        protocol_version: "0.1".into(),
        request_id: "remote-1".into(),
        provider_id: item.provider_id.clone(),
        action_id: "intake.accept".into(),
        subject: miilo_adapter_sdk::SubjectReference {
            kind: miilo_adapter_sdk::SubjectKind::Task,
            id: item.id.clone(),
        },
        expected_revision: item.revision.clone(),
        confirmation: miilo_adapter_sdk::Confirmation {
            method: miilo_adapter_sdk::ConfirmationMethod::Manual,
            confirmed_at: chrono::Utc::now(),
        },
        input: serde_json::Map::new(),
        idempotency_key: Some("remote-1".into()),
    };
    let first = client.invoke(&action).await.unwrap();
    let second = client.invoke(&action).await.unwrap();
    assert_eq!(first.recorded_at, second.recorded_at);
    assert_eq!(
        first.provider_state.as_deref(),
        Some("queued_awaiting_executor")
    );
    assert_eq!(
        client.snapshot().await.unwrap().work_items[0].provider_state,
        "queued_awaiting_executor"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn unix_socket_uses_os_ownership_without_a_bearer_credential() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("state");
    let socket = root.path().join("intake.sock");
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let _server = Server(
        Command::new(env!("CARGO_BIN_EXE_miilo"))
            .args([
                "inbox",
                "--state",
                state.to_str().unwrap(),
                "serve",
                "--listen-socket",
                socket.to_str().unwrap(),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap(),
    );
    let mut client = None;
    for _ in 0..50 {
        if let Ok(value) = miilo_client::MiiloClient::unix(&socket) {
            if value.health().await.is_ok() {
                client = Some(value);
                break;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let client = client.expect("private socket listener did not start");
    assert!(client.snapshot().await.unwrap().work_items.is_empty());
    std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o666)).unwrap();
    assert!(miilo_client::MiiloClient::unix(&socket).is_err());
}

#[cfg(unix)]
#[tokio::test]
async fn shutdown_closes_live_streams_and_releases_private_socket() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::Builder::new()
        .prefix("miilo-stop-")
        .tempdir_in("/tmp")
        .unwrap();
    let state = root.path().join("state");
    let socket = root.path().join("intake.sock");
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut server = Server(
        Command::new(env!("CARGO_BIN_EXE_miilo"))
            .args([
                "inbox",
                "--state",
                state.to_str().unwrap(),
                "serve",
                "--listen-socket",
                socket.to_str().unwrap(),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap(),
    );
    let mut ready = None;
    for _ in 0..50 {
        if let Ok(client) = miilo_client::MiiloClient::unix(&socket) {
            if client.health().await.is_ok() {
                ready = Some(client);
                break;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let client = ready.unwrap();
    let _stream = client.observe().await.unwrap();
    assert!(
        Command::new("kill")
            .args(["-TERM", &server.0.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let mut stopped = false;
    for _ in 0..100 {
        if server.0.try_wait().unwrap().is_some() {
            stopped = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(stopped, "active SSE prevented shutdown");
    assert!(!socket.exists());
}
