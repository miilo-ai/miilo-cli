use super::*;
use miilo_adapter_sdk::{ObservatorySnapshot, runtime::ObservatoryRuntime};
pub(super) fn read_snapshot() -> Result<ObservatorySnapshot> {
    let config: Value = serde_json::from_str(&private_read(&config_path())?)
        .map_err(|_| "Invalid local configuration")?;
    let path = config["snapshotFile"]
        .as_str()
        .ok_or("No standalone snapshot configured")?;
    let snapshot: ObservatorySnapshot = serde_json::from_str(&private_read(Path::new(path))?)
        .map_err(|_| "Invalid local snapshot")?;
    if !(0..=15).contains(&(chrono::Utc::now() - snapshot.observed_at).num_seconds()) {
        return Err("Local observation is stale; start miilo local run".into());
    }
    Ok(snapshot)
}
fn project(session: &sessions::Session) -> Result<ObservatorySnapshot> {
    let state = &session.observation;
    let recent = chrono::DateTime::parse_from_rfc3339(&state.observed_at).is_ok_and(|at| {
        (0..120).contains(&(chrono::Utc::now() - at.with_timezone(&chrono::Utc)).num_seconds())
    });
    let mut snapshot =
        serde_json::to_value(state.snapshot().map_err(|_| "Cannot project observation")?)
            .map_err(|_| "Cannot encode observation")?;
    snapshot["workItems"][0]["revision"] = json!(state.cursor(state.revision));
    snapshot["providers"][0]["extensions"]["dev.miilo.groups"] = session.groups.clone();
    snapshot["actors"] = json!([{"id":"session-agent","providerId":state.provider_id,"kind":"agent","name":state.title,"health":if state.error.is_some(){"unreachable"}else{"healthy"},"status":if state.error.is_some(){"offline"}else if state.state=="active"&&recent{"working"}else{"resting"},"capabilities":[],"updatedAt":state.observed_at,"extensions":{"dev.miilo.session":{"runtimeStatusAuthoritative":false}}}]);
    snapshot["attempts"] = json!([{"id":"observed-session","providerId":state.provider_id,"taskId":state.thread_id,"sequence":1,"lifecycle":state.state,"phase":"unknown","actorIds":["session-agent"],"startedAt":state.observed_at,"completedAt":null,"summary":"Observed existing session; execution status is not authoritative","extensions":null}]);
    serde_json::from_value(snapshot).map_err(|_| "Cannot decode observation".into())
}
pub(super) async fn run(mut config: Value, once: bool) -> Result<()> {
    if config["version"] != 2 || config["mode"] != "local" {
        return Err("Unsupported standalone configuration".into());
    }
    let _lock = files::collector_lock()?;
    let cache = PathBuf::from(
        config["snapshotFile"]
            .as_str()
            .ok_or("Missing snapshot file")?,
    );
    if !cache.is_absolute() {
        return Err("Snapshot file must be an absolute path".into());
    }
    let rules = groups::validate(&config["groups"])?;
    let mut selected = sessions(&config)?;
    let mut wakeups = if once {
        None
    } else {
        Some(wakeups::Wakeups::new(&config)?)
    };
    let mut runtime = ObservatoryRuntime::new(
        format!(
            "local-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ),
        512,
    )
    .map_err(|e| e.to_string())?;
    for session in &mut selected {
        if session.tail.read(&mut session.observation).is_err() {
            session.observation.error = Some("Selected transcript unavailable".into());
        }
        session.observation.state = "waiting";
        runtime
            .register(
                serde_json::from_value(session.observation.descriptor())
                    .map_err(|_| "Invalid local descriptor")?,
                chrono::Utc::now(),
            )
            .map_err(|e| e.to_string())?;
    }
    let mut next_discovery = tokio::time::Instant::now() + Duration::from_secs(60);
    loop {
        if rules.iter().any(|r| r["future"] == true)
            && tokio::time::Instant::now() >= next_discovery
        {
            next_discovery = tokio::time::Instant::now() + Duration::from_secs(60);
            let directory = config["project"]
                .as_str()
                .map(PathBuf::from)
                .unwrap_or_else(home);
            let found = inventory(&directory).await;
            let additions: Vec<_> = found["sessions"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|s| rules.iter().any(|r| groups::future_matches(r, s)))
                .filter(|s| {
                    let key = format!(
                        "{}:{}",
                        s["kind"].as_str().unwrap_or(""),
                        s["id"].as_str().unwrap_or("")
                    );
                    !config["knownSessions"]
                        .as_array()
                        .is_some_and(|v| v.iter().any(|i| i == &key))
                        && !config["sessions"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|old| old["kind"] == s["kind"] && old["id"] == s["id"])
                })
                .take(100usize.saturating_sub(selected.len()))
                .cloned()
                .collect();
            for addition in additions {
                if let Ok(mut resolved) = selection::resolve(vec![addition]).await {
                    let mut item = resolved.remove(0);
                    item["groups"] = json!(groups::labels(&item, &rules));
                    let mut session = sessions(&json!({"sessions":[item.clone()]}))?.remove(0);
                    if session.tail.read(&mut session.observation).is_err() {
                        continue;
                    }
                    session.observation.state = "waiting";
                    runtime
                        .register(
                            serde_json::from_value(session.observation.descriptor())
                                .map_err(|_| "Invalid local descriptor")?,
                            chrono::Utc::now(),
                        )
                        .map_err(|e| e.to_string())?;
                    if let Some(wakeups) = &mut wakeups {
                        wakeups.watch(Path::new(
                            item["path"].as_str().ok_or("Missing selected transcript")?,
                        ))?;
                    }
                    config["sessions"].as_array_mut().unwrap().push(item);
                    save_config(&config_path(), &config)?;
                    selected.push(session);
                }
            }
        }
        for session in &mut selected {
            if session.tail.read(&mut session.observation).is_err() {
                session.observation.error =
                    Some("Selected transcript unavailable; reconnect explicitly".into());
            }
            let snapshot = project(session)?;
            runtime
                .replace(
                    &session.observation.provider_id,
                    snapshot,
                    chrono::Utc::now(),
                )
                .map_err(|e| e.to_string())?;
        }
        let mut snapshot = runtime.snapshot(chrono::Utc::now());
        let explanations: Vec<_> = snapshot
            .work_items
            .iter()
            .filter_map(|w| {
                miilo_adapter_sdk::runtime::explain(&snapshot, &w.provider_id, &w.id).ok()
            })
            .collect();
        snapshot.extensions = Some(serde_json::Map::from_iter([(
            "dev.miilo.explanations".into(),
            serde_json::to_value(explanations).map_err(|_| "Cannot encode explanations")?,
        )]));
        save_config(
            &cache,
            &serde_json::to_value(snapshot).map_err(|_| "Cannot serialize snapshot")?,
        )?;
        if once {
            return Ok(());
        }
        tokio::select! {_=tokio::time::sleep(Duration::from_secs(5))=>{},_=wakeups.as_mut().expect("continuous observer has notifications").wait()=>{},_=tokio::signal::ctrl_c()=>return Ok(())}
    }
}
