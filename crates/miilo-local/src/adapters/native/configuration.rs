//! Shared user configuration. No project discovery, enrollment or service changes.
use super::{Result, env, files, fs, home, json};
use serde_json::Value;
use std::path::PathBuf;

pub(super) fn root() -> Result<PathBuf> {
    let path = env::var_os("MIILO_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".miilo"));
    if !path.is_absolute() {
        return Err("MIILO_HOME must be an absolute user configuration directory".into());
    }
    if fs::symlink_metadata(&path).is_ok_and(|m| !m.is_dir() || m.file_type().is_symlink()) {
        return Err("MIILO_HOME must be a directory, not a symlink".into());
    }
    Ok(path)
}

pub(super) fn manifest() -> Result<Option<Value>> {
    let path = root()?.join("config.json");
    if fs::symlink_metadata(&path).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound) {
        return Ok(None);
    }
    let value: Value = serde_json::from_str(&files::private_read(&path)?)
        .map_err(|_| "Invalid Miilo configuration")?;
    if value["schemaVersion"] != 1
        || !value.is_object()
        || value.get("localConfigFile").is_some_and(|p| {
            !p.as_str()
                .is_some_and(|s| std::path::Path::new(s).is_absolute())
        })
    {
        return Err("Unsupported Miilo configuration; expected schemaVersion 1 and an absolute localConfigFile".into());
    }
    Ok(Some(value))
}

pub(super) fn legacy_local() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".config"))
        .join("miilo/local.json")
}

pub(super) fn local_path() -> Result<PathBuf> {
    if let Some(path) = env::var_os("MIILO_LOCAL_CONFIG") {
        return Ok(path.into());
    }
    if let Some(value) = manifest()? {
        return Ok(value["localConfigFile"]
            .as_str()
            .map(PathBuf::from)
            .unwrap_or(root()?.join("local.json")));
    }
    Ok(legacy_local())
}

pub(super) fn execute(args: &[String]) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("paths") => {
            let root = root()?;
            let active = manifest()?.is_some();
            let helper = env::var_os("MIILO_OWL_CONFIG_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    if active {
                        root.join("connections")
                    } else {
                        home().join(".config/miilo-owl")
                    }
                });
            let preferences = if active {
                root.join("preferences")
            } else {
                env::var_os("XDG_STATE_HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| home().join(".local/state"))
                    .join("miilo/widgets")
            };
            println!(
                "{}",
                json!({"home":root,"active":active,
                "localConfiguration":local_path()?,
                "connections":helper.join("config.json"),
                "preferences":preferences})
            );
            Ok(())
        }
        Some("init" | "migrate") => super::configuration_migration::migrate(),
        Some("check") => {
            let value =
                manifest()?.ok_or("Run miilo config migrate to initialize user configuration")?;
            println!(
                "{}",
                json!({"schemaVersion":value["schemaVersion"],"valid":true})
            );
            Ok(())
        }
        _ => Err("Use miilo config paths | init | migrate | check".into()),
    }
}
