use super::*;
pub(super) fn home() -> PathBuf {
    PathBuf::from(env::var_os("HOME").unwrap_or_default())
}
pub(super) fn config_path() -> PathBuf {
    env::var_os("MIILO_LOCAL_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home().join(".config"))
                .join("miilo/local.json")
        })
}
pub(super) fn private_read(path: &Path) -> Result<String> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| "Cannot read protected configuration")?;
    let metadata = file
        .metadata()
        .map_err(|_| "Cannot inspect protected file")?;
    if !metadata.is_file()
        || metadata.permissions().mode() & 0o077 != 0
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.len() > 2_000_000
    {
        return Err("Expected a regular owner-only file belonging to this user (chmod 600)".into());
    }
    let mut text = String::new();
    file.take(2_000_001)
        .read_to_string(&mut text)
        .map_err(|_| "Cannot read protected configuration")?;
    if text.len() > 2_000_000 {
        return Err("Protected file exceeds size limit".into());
    }
    Ok(text)
}
pub(super) fn save_config(path: &Path, value: &Value) -> Result<()> {
    let parent = path.parent().ok_or("Invalid configuration path")?;
    fs::create_dir_all(parent).map_err(|_| "Cannot create configuration directory")?;
    let mut file = tempfile::NamedTempFile::new_in(parent)
        .map_err(|_| "Cannot create private temporary file")?;
    file.as_file()
        .set_permissions(fs::Permissions::from_mode(0o600))
        .map_err(|_| "Cannot protect file")?;
    let bytes = serde_json::to_vec(value).map_err(|_| "Cannot encode configuration")?;
    if bytes.len() > 2_000_000 {
        return Err("Local data exceeds size limit".into());
    }
    file.write_all(&bytes)
        .map_err(|_| "Cannot write configuration")?;
    file.as_file()
        .sync_all()
        .map_err(|_| "Cannot sync configuration")?;
    file.persist(path)
        .map_err(|_| "Cannot save configuration")?;
    Ok(())
}

pub(super) fn cache_path() -> PathBuf {
    env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".local/state"))
        .join("miilo/snapshot.json")
}

pub(super) fn collector_lock() -> Result<fs::File> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    let path = config_path().with_extension("lock");
    let parent = path.parent().ok_or("Invalid configuration path")?;
    fs::create_dir_all(parent).map_err(|_| "Cannot create collector lock directory")?;
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(&path)
        .map_err(|_| "Cannot open private collector lock")?;
    let meta = file
        .metadata()
        .map_err(|_| "Cannot inspect collector lock")?;
    if !meta.is_file()
        || meta.permissions().mode() & 0o077 != 0
        || meta.uid() != unsafe { libc::geteuid() }
    {
        return Err("Collector lock must belong to this user and be private".into());
    }
    fs2::FileExt::try_lock_exclusive(&file)
        .map_err(|_| "Observation is already running for this configuration")?;
    Ok(file)
}
