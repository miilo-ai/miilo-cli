use crate::{Result, application::Store, domain::Ledger};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
pub struct PrivateStore(pub PathBuf);
pub fn prepare(directory: &Path) -> Result<()> {
    if !directory.is_absolute() {
        return Err("Pairing directory must be absolute".into());
    }
    fs::create_dir_all(directory).map_err(|_| "Cannot create pairing directory")?;
    let m = fs::symlink_metadata(directory).map_err(|_| "Cannot inspect pairing directory")?;
    if !m.is_dir() || m.file_type().is_symlink() {
        return Err("Unsafe pairing directory".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        if m.uid() != unsafe { libc::geteuid() } {
            return Err("Pairing directory has another owner".into());
        }
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700))
            .map_err(|_| "Cannot protect directory")?;
    }
    Ok(())
}
pub fn read<T: DeserializeOwned>(file: &Path) -> Result<T> {
    #[cfg(unix)]
    use std::os::unix::fs::OpenOptionsExt;
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    let opened = options.open(file).map_err(|_| "Private file unavailable")?;
    let m = opened
        .metadata()
        .map_err(|_| "Cannot inspect private file")?;
    if !m.is_file() {
        return Err("Unsafe private file".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        if m.permissions().mode() & 0o077 != 0 || m.uid() != unsafe { libc::geteuid() } {
            return Err("Unsafe private file permissions".into());
        }
    }
    let mut data = Vec::new();
    opened
        .take(2_000_001)
        .read_to_end(&mut data)
        .map_err(|_| "Cannot read private file")?;
    if data.len() > 2_000_000 {
        return Err("Private file too large".into());
    }
    serde_json::from_slice(&data).map_err(|_| "Invalid private file".into())
}
pub fn write<T: Serialize>(file: &Path, value: &T) -> Result<()> {
    let parent = file.parent().ok_or("Missing private directory")?;
    prepare(parent)?;
    if fs::symlink_metadata(file).is_ok_and(|m| !m.is_file() || m.file_type().is_symlink()) {
        return Err("Unsafe private destination".into());
    }
    let mut temp =
        tempfile::NamedTempFile::new_in(parent).map_err(|_| "Cannot stage private file")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temp.as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|_| "Cannot protect private file")?;
    }
    serde_json::to_writer(&mut temp, value).map_err(|_| "Cannot encode private file")?;
    temp.flush()
        .and_then(|_| temp.as_file().sync_all())
        .map_err(|_| "Cannot flush private file")?;
    temp.persist(file).map_err(|_| "Cannot save private file")?;
    Ok(())
}
impl Store for PrivateStore {
    fn save(&self, ledger: &Ledger) -> Result<()> {
        write(&self.0, ledger)
    }
}
