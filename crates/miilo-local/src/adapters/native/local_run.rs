use super::observation_ports::{FileSnapshot, SystemClock};
use super::*;
use crate::application::{collector::Collector, ports::ObservationSource};
use miilo_adapter_sdk::ObservatorySnapshot;
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
    let mut collector = Collector::new(format!(
        "local-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    ))
    .map_err(|e| e.to_string())?;
    for session in &mut selected {
        if session.tail.read(&mut session.observation).is_err() {
            session.observation.error = Some("Selected transcript unavailable".into());
        }
        session.observation.state = "waiting";
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
        let mut sources: Vec<&mut dyn ObservationSource> = selected
            .iter_mut()
            .map(|s| s as &mut dyn ObservationSource)
            .collect();
        collector.collect(&mut sources, &SystemClock, &mut FileSnapshot(cache.clone()))?;
        if once {
            return Ok(());
        }
        tokio::select! {_=tokio::time::sleep(Duration::from_secs(5))=>{},_=wakeups.as_mut().expect("continuous observer has notifications").wait()=>{},_=tokio::signal::ctrl_c()=>return Ok(())}
    }
}
