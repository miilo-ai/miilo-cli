//! Explicit import. Local mode never provisions a server, account or credential.
use super::*;
use std::io::BufRead;
pub(super) async fn apply(request: Value) -> Result<Value> {
    let directory = request["project"]
        .as_str()
        .map(PathBuf::from)
        .unwrap_or_else(home);
    if !directory.is_dir() {
        return Err("Choose an existing project directory".into());
    }
    let rules = groups::validate(&request["groups"])?;
    let available = inventory(&directory).await;
    let chosen = if request["sessions"].as_array().is_some_and(Vec::is_empty)
        && request.get("groups").is_some()
    {
        vec![]
    } else {
        selection::select(&request, &available)?
    };
    let selected = selection::resolve(chosen).await?;
    let path = config_path();
    let previous = if path.exists() {
        Some(private_read(&path)?)
    } else {
        None
    };
    let old: Value = previous
        .as_ref()
        .map(|s| serde_json::from_str(s).map_err(|_| "Invalid existing local configuration"))
        .transpose()?
        .unwrap_or(Value::Null);
    if old["version"] == 1 {
        return Err("Existing CE configuration is preserved. Select a separate MIILO_LOCAL_CONFIG file for standalone import.".into());
    }
    if !old.is_null() && old["version"] != 2 {
        return Err("Unsupported existing local configuration".into());
    }
    let mut combined = old["sessions"].as_array().cloned().unwrap_or_default();
    for session in selected {
        if let Some(item) = combined
            .iter_mut()
            .find(|s| s["kind"] == session["kind"] && s["id"] == session["id"])
        {
            *item = session;
        } else {
            combined.push(session);
        }
    }
    if combined.len() > 100 {
        return Err("At most 100 selected sessions".into());
    }
    let rules = if request.get("groups").is_some() {
        rules
    } else {
        groups::validate(&old["groups"])?
    };
    for session in &mut combined {
        session["groups"] = json!(groups::labels(session, &rules));
    }
    let known: Vec<_> = available["sessions"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|s| {
            format!(
                "{}:{}",
                s["kind"].as_str().unwrap_or(""),
                s["id"].as_str().unwrap_or("")
            )
        })
        .collect();
    let snapshot_file = old["snapshotFile"]
        .as_str()
        .map(PathBuf::from)
        .unwrap_or_else(cache_path);
    let config = json!({"version":2,"mode":"local","snapshotFile":snapshot_file,"sessions":combined,"groups":rules,"project":directory,"knownSessions":known});
    let start_service = request["startService"] != false && cfg!(target_os = "linux");
    let previous_cache = if snapshot_file.exists() {
        Some(private_read(&snapshot_file)?)
    } else {
        None
    };
    let service_backup = if start_service {
        Some(service::pause_owned().await?)
    } else {
        None
    };
    let outcome = async {
        save_config(&path, &config)?;
        local_run::run(config, true).await?;
        if start_service {
            let initial: Value = serde_json::from_str(&private_read(&snapshot_file)?)
                .map_err(|_| "Invalid initial observation")?;
            service::install(&path).await?;
            service::ready(
                &path,
                initial["snapshotId"]
                    .as_str()
                    .ok_or("Missing initial snapshot identity")?,
            )
            .await?;
        }
        Ok(())
    }
    .await;
    if let Err(error) = outcome {
        if let Some(old) = previous {
            save_config(
                &path,
                &serde_json::from_str(&old).map_err(|_| "Cannot restore configuration")?,
            )?;
        } else {
            let _ = fs::remove_file(&path);
        }
        if let Some(old) = previous_cache {
            save_config(
                &snapshot_file,
                &serde_json::from_str(&old).map_err(|_| "Cannot restore snapshot")?,
            )?;
        } else {
            let _ = fs::remove_file(&snapshot_file);
        }
        if let Some(backup) = service_backup {
            service::restore(backup).await?;
        }
        return Err(error);
    }
    if request["connectWidget"] != false {
        let widget = env::var_os("MIILO_OWL_BINARY")
            .map(PathBuf::from)
            .unwrap_or(
                env::current_exe()
                    .map_err(|_| "Cannot locate installation")?
                    .with_file_name("miilo-owl"),
            );
        if widget.is_file() {
            let response = Command::new(widget)
                .arg("connect-local")
                .arg(&path)
                .kill_on_drop(true)
                .stdout(std::process::Stdio::null())
                .status()
                .await
                .map_err(|_| "Cannot connect widget")?;
            if !response.success() {
                return Err("Observation saved, but widget connection failed; retry connect-local with this configuration".into());
            }
        }
    }
    Ok(
        json!({"status":"connected","mode":"local","sessions":combined.len(),"snapshotFile":snapshot_file,"message":"Selected local sessions imported. Run miilo local run for live updates."}),
    )
}
pub(super) async fn execute() -> Result<()> {
    let mut line = String::new();
    io::stdin()
        .lock()
        .take(65_537)
        .read_line(&mut line)
        .map_err(|_| "Cannot read import selection")?;
    if line.len() > 65_536 {
        return Err("Import selection too large".into());
    }
    let request = serde_json::from_str(&line).map_err(|_| "Invalid import selection")?;
    match apply(request).await {
        Ok(value) => {
            println!("{value}");
            Ok(())
        }
        Err(message) => {
            println!("{}", json!({"status":"failed","message":message}));
            Err("Local import was not completed".into())
        }
    }
}
fn prompt(label: &str) -> Result<String> {
    print!("{label}");
    io::stdout().flush().map_err(|_| "Cannot prompt")?;
    let mut value = String::new();
    if io::stdin()
        .read_line(&mut value)
        .map_err(|_| "Cannot read selection")?
        == 0
    {
        return Err("Setup cancelled".into());
    }
    Ok(value.trim().into())
}
pub(super) async fn setup(directory: &Path) -> Result<()> {
    println!("Choose existing sessions to observe locally. Nothing will be started or resumed.");
    let found = inventory(directory).await;
    let sessions = found["sessions"].as_array().ok_or("Invalid discovery")?;
    for (i, s) in sessions.iter().enumerate() {
        println!(
            "{}. {} · {}",
            i + 1,
            s["kind"].as_str().unwrap_or("agent"),
            s["name"].as_str().unwrap_or("Local session")
        );
    }
    let answer = prompt("Session numbers separated by commas: ")?;
    let mut selected = vec![];
    for part in answer.split(',') {
        let n: usize = part.trim().parse().map_err(|_| "Invalid session number")?;
        selected.push(
            sessions
                .get(n.checked_sub(1).ok_or("Invalid session number")?)
                .ok_or("Invalid session number")?
                .clone(),
        );
    }
    let response =
        apply(json!({"project":directory,"sessions":selected,"connectWidget":false})).await?;
    println!("{response}");
    Ok(())
}
