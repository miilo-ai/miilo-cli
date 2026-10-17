//! Explicit copy migration; original files and selected local observation stay intact.
use super::{Result, configuration, env, files, fs, home, json};
use std::{
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

fn private_dir(path: &Path) -> Result<()> {
    if let Ok(meta) = fs::symlink_metadata(path) {
        if !meta.is_dir() || meta.uid() != unsafe { libc::geteuid() } {
            return Err(
                "Configuration directory must be owned by this user and not a symlink".into(),
            );
        }
    } else {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(path)
            .map_err(|_| "Cannot create Miilo directory")?;
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|_| "Cannot protect Miilo directory".into())
}

fn read_source(path: &Path, secret: bool) -> Result<Option<String>> {
    if fs::symlink_metadata(path).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound) {
        return Ok(None);
    }
    if secret {
        return files::private_read(path).map(Some);
    }
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| "Cannot read legacy preferences")?;
    let meta = file
        .metadata()
        .map_err(|_| "Cannot inspect legacy preferences")?;
    if !meta.is_file() || meta.uid() != unsafe { libc::geteuid() } || meta.len() > 2_000_000 {
        return Err("Invalid legacy preferences file".into());
    }
    let mut text = String::new();
    file.take(2_000_001)
        .read_to_string(&mut text)
        .map_err(|_| "Cannot read preferences")?;
    if text.len() > 2_000_000 {
        return Err("Preferences exceed size limit".into());
    }
    Ok(Some(text))
}

fn publish(path: &Path, text: &str) -> Result<()> {
    private_dir(path.parent().ok_or("Missing configuration directory")?)?;
    if let Some(existing) = read_source(path, true)? {
        return if existing == text {
            Ok(())
        } else {
            Err("Migration destination differs; existing files were preserved".into())
        };
    }
    let mut file = tempfile::NamedTempFile::new_in(path.parent().unwrap())
        .map_err(|_| "Cannot stage configuration")?;
    file.write_all(text.as_bytes())
        .map_err(|_| "Cannot write configuration")?;
    file.as_file()
        .sync_all()
        .map_err(|_| "Cannot sync configuration")?;
    file.persist_noclobber(path)
        .map_err(|_| "Configuration already exists or cannot be installed")?;
    fs::File::open(path.parent().unwrap())
        .and_then(|directory| directory.sync_all())
        .map_err(|_| "Cannot sync configuration directory")?;
    Ok(())
}

pub(super) fn migrate() -> Result<()> {
    let root = configuration::root()?;
    if configuration::manifest()?.is_some() {
        println!("{}", json!({"active":true,"changed":false,"home":root}));
        return Ok(());
    }
    let helper = env::var_os("MIILO_OWL_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".config/miilo-owl"));
    let state = env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".local/state"));
    let mut copies = Vec::new();
    for (source, destination, secret) in [
        (
            helper.join("config.json"),
            root.join("connections/config.json"),
            true,
        ),
        (
            helper.join("chat.json"),
            root.join("connections/chat.json"),
            true,
        ),
        (
            helper.join("local-discovery.json"),
            root.join("connections/local-discovery.json"),
            true,
        ),
        (
            state.join("miilo/widgets/70-miilo-fleet.owl.json"),
            root.join("preferences/70-miilo-fleet.owl.json"),
            false,
        ),
    ] {
        if let Some(text) = read_source(&source, secret)? {
            let value: serde_json::Value = serde_json::from_str(&text)
                .map_err(|_| "Legacy configuration is invalid JSON; migration stopped")?;
            if !value.is_object() {
                return Err("Legacy configuration must be an object".into());
            }
            copies.push((source, destination, text, secret));
        }
    }
    // Keep the collector's explicit configuration path: no duplicate enrollment,
    // service rewrite, cache move or background process restart is needed.
    let mut local = configuration::local_path()?;
    if env::var_os("MIILO_LOCAL_CONFIG").is_none() && !local.exists() {
        local = root.join("local.json");
    }
    if !local.is_absolute() {
        return Err("Use an absolute MIILO_LOCAL_CONFIG path before migrating".into());
    }
    if local.exists() {
        files::private_read(&local)?;
    }
    let manifest = json!({"schemaVersion":1,"localConfigFile":local});
    private_dir(&root)?;
    for (_, destination, text, _) in &copies {
        publish(destination, text)?;
    }
    for (source, _, text, secret) in &copies {
        if read_source(source, *secret)?.as_ref() != Some(text) {
            return Err(
                "Legacy settings changed during migration; close old clients and retry".into(),
            );
        }
    }
    publish(
        &root.join("config.json"),
        &serde_json::to_string_pretty(&manifest)
            .map_err(|_| "Cannot encode Miilo configuration")?,
    )?;
    println!(
        "{}",
        json!({"active":true,"changed":true,"home":root,
        "copiedFiles":copies.len(),"originalsPreserved":true,
        "message":"Restart updated clients to use shared configuration. Existing credential formats are preserved; no keyring conversion or service restart occurred."})
    );
    Ok(())
}
