use super::*;
fn quote(value: &str) -> Result<String> {
    if value.contains(['\r', '\n', '\0']) {
        return Err("Unsupported service path".into());
    }
    Ok(format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('%', "%%")
    ))
}
pub(super) async fn install(configuration: &Path) -> Result<()> {
    if !cfg!(target_os = "linux") {
        return Err("Automatic local observation service setup currently requires Linux; run miilo local run in your terminal".into());
    }
    let executable = env::current_exe().map_err(|_| "Cannot locate observer")?;
    let prefix = if executable.file_stem().is_some_and(|s| s == "miilo") {
        " local"
    } else {
        ""
    };
    let unit_dir = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".config"))
        .join("systemd/user");
    fs::create_dir_all(&unit_dir).map_err(|_| "Cannot create service directory")?;
    let mut unit = format!(
        "[Unit]\nDescription=Miilo selected local session observation\n\n[Service]\nType=simple\nExecStart={}{prefix} run\nEnvironment={}\nRestart=on-failure\nRestartSec=3\n\n[Install]\nWantedBy=default.target\n",
        quote(&executable.to_string_lossy())?,
        quote(&format!("MIILO_LOCAL_CONFIG={}", configuration.display()))?
    );
    let mut additional = String::new();
    for variable in [
        "CODEX_HOME",
        "MIILO_CURSOR_PROJECTS",
        "XDG_STATE_HOME",
        "XDG_DATA_HOME",
        "MIILO_OPENCODE_DB",
        "MIILO_DEVIN_DB",
    ] {
        if let Ok(value) = env::var(variable) {
            additional.push_str(&format!(
                "Environment={}\n",
                quote(&format!("{variable}={value}"))?
            ));
        }
    }
    let codex = env::var("MIILO_CODEX_EXECUTABLE").ok().or_else(|| {
        env::var_os("PATH").and_then(|value| {
            env::split_paths(&value)
                .map(|folder| folder.join("codex"))
                .find(|path| path.is_file())
                .map(|path| path.to_string_lossy().into_owned())
        })
    });
    if let Some(codex) = codex {
        additional.push_str(&format!(
            "Environment={}\n",
            quote(&format!("MIILO_CODEX_EXECUTABLE={codex}"))?
        ));
    }
    unit = unit.replace("Restart=on-failure", &(additional + "Restart=on-failure"));
    fs::write(unit_dir.join("miilo-local-desktop.service"), unit)
        .map_err(|_| "Cannot write observation service")?;
    for args in [
        vec!["daemon-reload"],
        vec!["enable", "--now", "miilo-local-desktop.service"],
        vec!["restart", "miilo-local-desktop.service"],
    ] {
        let status = tokio::time::timeout(
            Duration::from_secs(15),
            Command::new(
                env::var("MIILO_SYSTEMCTL_EXECUTABLE").unwrap_or_else(|_| "systemctl".into()),
            )
            .arg("--user")
            .args(args)
            .kill_on_drop(true)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status(),
        )
        .await
        .map_err(|_| "User service manager timed out")?
        .map_err(|_| "Cannot contact user service manager")?;
        if !status.success() {
            return Err("Cannot start local observation service".into());
        }
    }
    Ok(())
}

fn unit_path() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".config"))
        .join("systemd/user/miilo-local-desktop.service")
}
async fn control(args: &[&str]) -> Result<bool> {
    let status = tokio::time::timeout(
        Duration::from_secs(15),
        Command::new(env::var("MIILO_SYSTEMCTL_EXECUTABLE").unwrap_or_else(|_| "systemctl".into()))
            .arg("--user")
            .args(args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status(),
    )
    .await
    .map_err(|_| "User service manager timed out")?
    .map_err(|_| "Cannot contact user service manager")?;
    Ok(status.success())
}
pub(super) struct Backup {
    unit: Option<String>,
    active: bool,
    enabled: bool,
}
pub(super) async fn pause_owned() -> Result<Backup> {
    let path = unit_path();
    let unit = if path.exists() {
        let metadata =
            fs::symlink_metadata(&path).map_err(|_| "Cannot inspect observer service")?;
        if !metadata.is_file() || metadata.len() > 65_536 {
            return Err("Invalid observer service file".into());
        }
        let text = fs::read_to_string(path).map_err(|_| "Cannot read observer service")?;
        if !text.contains("Description=Miilo selected local") {
            return Err("Existing service does not belong to this Miilo observer; preserve it and choose a separate configuration".into());
        }
        Some(text)
    } else {
        None
    };
    let active = control(&["is-active", "--quiet", "miilo-local-desktop.service"]).await?;
    let enabled = control(&["is-enabled", "--quiet", "miilo-local-desktop.service"]).await?;
    if active && unit.is_none() {
        return Err(
            "The active observer service is outside this configuration; it will not be stopped"
                .into(),
        );
    }
    if active && !control(&["stop", "miilo-local-desktop.service"]).await? {
        return Err("Cannot pause the existing observer for import".into());
    }
    Ok(Backup {
        unit,
        active,
        enabled,
    })
}
pub(super) async fn restore(backup: Backup) -> Result<()> {
    let _ = control(&["stop", "miilo-local-desktop.service"]).await?;
    if let Some(unit) = backup.unit {
        fs::write(unit_path(), unit).map_err(|_| "Cannot restore observer service")?;
    } else {
        let _ = fs::remove_file(unit_path());
    }
    if !control(&["daemon-reload"]).await? {
        return Err("Cannot reload previous observer service".into());
    }
    let operation = if backup.enabled { "enable" } else { "disable" };
    let _ = control(&[operation, "miilo-local-desktop.service"]).await?;
    if backup.active && !control(&["start", "miilo-local-desktop.service"]).await? {
        return Err("Cannot restore running observer".into());
    }
    Ok(())
}

pub(super) async fn ready(config: &Path, previous: &str) -> Result<()> {
    let value: Value = serde_json::from_str(&private_read(config)?)
        .map_err(|_| "Invalid observation configuration")?;
    let path = Path::new(
        value["snapshotFile"]
            .as_str()
            .ok_or("Missing local snapshot")?,
    );
    for _ in 0..50 {
        if let Ok(text) = private_read(path) {
            if let Ok(snapshot) = serde_json::from_str::<Value>(&text) {
                if snapshot["snapshotId"]
                    .as_str()
                    .is_some_and(|id| id != previous)
                    && control(&["is-active", "--quiet", "miilo-local-desktop.service"]).await?
                {
                    return Ok(());
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err("Local observation service did not publish a fresh snapshot; import was rolled back".into())
}
