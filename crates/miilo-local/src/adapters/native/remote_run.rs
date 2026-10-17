use super::*;
fn validate_endpoint(endpoint: &str) -> Result<()> {
    let url = url::Url::parse(endpoint).map_err(|_| "Invalid CE endpoint")?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("Use an HTTP(S) CE URL without embedded credentials".into());
    }
    // Desktop session metadata stays local unless a remote encrypted target is explicit.
    if url.scheme() == "http"
        && !matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))
    {
        return Err("Remote CE endpoints require HTTPS".into());
    }
    Ok(())
}
pub(super) async fn run(once: bool) -> Result<()> {
    let mut config: Value = serde_json::from_str(&private_read(&config_path())?)
        .map_err(|_| "Invalid local configuration")?;
    if config["version"] != 1 {
        return Err("Unsupported local configuration version".into());
    }
    let endpoint_owned = config["endpoint"]
        .as_str()
        .ok_or("Missing CE endpoint")?
        .trim_end_matches('/')
        .to_owned();
    let endpoint = endpoint_owned.as_str();
    validate_endpoint(endpoint)?;
    let token_path_owned = PathBuf::from(
        config["tokenFile"]
            .as_str()
            .ok_or("Missing CE credential file")?,
    );
    let token_path = token_path_owned.as_path();
    let group_rules = groups::validate(&config["groups"])?;
    if group_rules
        .iter()
        .any(|rule| rule["future"] == true && rule["enrolledAfter"].is_null())
    {
        eprintln!(
            "Future enrollment is paused for legacy rules without creation-time consent; selected sessions remain observed."
        );
    }
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|_| "Cannot initialize HTTP")?;
    let mut owned_ce = if config["managedCe"] == true {
        let local = url::Url::parse(endpoint).map_err(|_| "Invalid managed local endpoint")?;
        if local.scheme() != "http" || local.host_str() != Some("127.0.0.1") || local.path() != "/"
        {
            return Err("Managed CE must use a loopback endpoint".into());
        }
        let bind = format!(
            "127.0.0.1:{}",
            local.port_or_known_default().ok_or("Missing local port")?
        );
        Some(
            Command::new(env::var("MIILO_CE_EXECUTABLE").unwrap_or_else(|_| "miilo-ce".into()))
                .env("MIILO_CE_TOKEN_FILE", token_path)
                .env_remove("MIILO_CE_TOKEN")
                .args(["--bind", &bind])
                .kill_on_drop(true)
                .spawn()
                .map_err(|_| "Install miilo-ce before starting local observation")?,
        )
    } else {
        None
    };
    if let Some(child) = &mut owned_ce {
        let mut ready = false;
        for _ in 0..30 {
            if child.try_wait().map_err(|_| "Cannot check CE")?.is_some() {
                return Err("Local CE failed to start; check whether port 3700 is in use".into());
            }
            if client
                .get(format!("{endpoint}/health"))
                .send()
                .await
                .is_ok_and(|r| r.status().is_success())
            {
                ready = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        if !ready {
            return Err("Local CE did not become ready".into());
        }
    }
    let mut selected = sessions(&config)?;
    // Bootstrap is history. Wait for a subsequent natural update before showing activity.
    for session in &mut selected {
        if session.tail.read(&mut session.observation).is_err() {
            session.observation.error = Some("Selected transcript unavailable".into());
        }
        session.observation.state = "waiting";
    }
    let mut revisions = std::collections::BTreeMap::new();
    let mut next_discovery = tokio::time::Instant::now();
    let mut reconnect_widgets = group_rules.iter().any(|r| r["future"] == true);
    loop {
        if group_rules.iter().any(|r| r["future"] == true)
            && tokio::time::Instant::now() >= next_discovery
        {
            next_discovery = tokio::time::Instant::now() + Duration::from_secs(60);
            let directory = config["project"]
                .as_str()
                .map(PathBuf::from)
                .unwrap_or_else(home);
            let found = inventory(&directory).await;
            let candidates: Vec<Value> = found["sessions"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|s| group_rules.iter().any(|r| groups::future_matches(r, s)))
                .filter(|s| {
                    let key = format!(
                        "{}:{}",
                        s["kind"].as_str().unwrap_or(""),
                        s["id"].as_str().unwrap_or("")
                    );
                    !config["knownSessions"]
                        .as_array()
                        .is_some_and(|known| known.iter().any(|v| v == &key))
                })
                .filter(|s| {
                    !config["sessions"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|old| old["kind"] == s["kind"] && old["id"] == s["id"])
                })
                .take(100usize.saturating_sub(selected.len()))
                .cloned()
                .collect();
            for candidate in candidates {
                if let Ok(mut resolved) = selection::resolve(vec![candidate]).await {
                    let mut item = resolved.remove(0);
                    item["groups"] = json!(groups::labels(&item, &group_rules));
                    let mut next_config = config.clone();
                    next_config["sessions"]
                        .as_array_mut()
                        .unwrap()
                        .push(item.clone());
                    if let Ok(mut additions) = sessions(&json!({"sessions":[item]})) {
                        let mut session = additions.remove(0);
                        if session.tail.read(&mut session.observation).is_err() {
                            continue;
                        }
                        session.observation.state = "waiting";
                        save_config(&config_path(), &next_config)?;
                        config = next_config;
                        selected.push(session);
                        reconnect_widgets = true;
                    }
                }
            }
        }
        // Read each time so token-file rotation is picked up without restarting.
        let token = private_read(token_path)?.trim().to_owned();
        if token.is_empty() || token.chars().any(char::is_whitespace) {
            return Err("Invalid CE credential".into());
        }
        let existing = client
            .get(format!("{endpoint}/v1/connections"))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|_| "CE unavailable")?;
        if !existing.status().is_success() {
            return Err("CE rejected connection discovery; check access".into());
        }
        let existing: Value = read_json(existing).await?;
        for session in &mut selected {
            let state = &mut session.observation;
            let id = state.provider_id.clone();
            let registered = existing["connections"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|c| c["id"] == id);
            if !registered {
                let response=client.post(format!("{endpoint}/v1/connections")).bearer_auth(&token).json(&json!({"id":id,"name":state.provider_name,"adapter":"local-observer","nativeUrl":null})).send().await.map_err(|_|"CE registration failed")?;
                if !response.status().is_success() {
                    return Err("CE rejected registration".into());
                }
            }
            // A missing/rotated transcript must not remain healthy.
            if session.tail.read(state).is_err() {
                state.error = Some("Selected transcript unavailable; reconnect explicitly".into());
            }
            let recent = chrono::DateTime::parse_from_rfc3339(&state.observed_at).is_ok_and(|at| {
                (0..120)
                    .contains(&(chrono::Utc::now() - at.with_timezone(&chrono::Utc)).num_seconds())
            });
            let revision = (state.revision, state.error.is_some(), recent);
            if registered && revisions.get(&id) == Some(&revision) {
                continue;
            }
            let snapshot = state.snapshot().map_err(|_| "Cannot project observation")?;
            let mut snapshot =
                serde_json::to_value(snapshot).map_err(|_| "Cannot serialize observation")?;
            snapshot["providers"][0]["extensions"]["dev.miilo.groups"] = session.groups.clone();
            snapshot["actors"] = json!([{"id":"session-agent","providerId":id,"kind":"agent","name":state.title,"health":if state.error.is_some(){"unreachable"}else{"healthy"},"status":if state.error.is_some(){"offline"}else if state.state=="active" && recent {"working"}else{"resting"},"capabilities":[],"updatedAt":state.observed_at,"extensions":{"dev.miilo.session":{"runtimeStatusAuthoritative":false}}}]);
            snapshot["attempts"] = json!([{"id":"observed-session","providerId":id,"taskId":state.thread_id,"sequence":1,"lifecycle":state.state,"phase":"unknown","actorIds":["session-agent"],"startedAt":state.observed_at,"completedAt":null,"summary":"Observed existing session; execution status is not authoritative","extensions":null}]);
            let response = client
                .put(format!("{endpoint}/v1/providers/{id}/snapshot"))
                .bearer_auth(&token)
                .json(&snapshot)
                .send()
                .await
                .map_err(|_| "CE ingestion failed")?;
            if !response.status().is_success() {
                return Err("CE rejected observation".into());
            }
            revisions.insert(id.clone(), revision);
        }
        if reconnect_widgets {
            let helper = env::var_os("MIILO_OWL_BINARY")
                .map(PathBuf::from)
                .unwrap_or(
                    env::current_exe()
                        .map_err(|_| "Cannot locate widget helper")?
                        .with_file_name("miilo-owl"),
                );
            if Command::new(helper)
                .arg("connect-local")
                .arg(config_path())
                .output()
                .await
                .is_ok_and(|r| r.status.success())
            {
                reconnect_widgets = false;
            }
        }
        if once {
            println!(
                "Published {} selected local sessions (read-only)",
                selected.len()
            );
            return Ok(());
        }
        tokio::select! { _=tokio::time::sleep(Duration::from_secs(2))=>{}, _=tokio::signal::ctrl_c()=>return Ok(()) }
    }
}

async fn read_json(mut response: reqwest::Response) -> Result<Value> {
    let mut data = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Cannot read CE response")?
    {
        if data.len() + chunk.len() > 2_000_000 {
            return Err("CE response exceeds local observer limit".into());
        }
        data.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&data).map_err(|_| "Invalid CE response".into())
}
