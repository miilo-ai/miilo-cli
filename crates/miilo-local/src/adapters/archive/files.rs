use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::Path,
};
pub fn open(path: &Path) -> Result<File, String> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| "Cannot inspect selected input")?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("Select a regular input file, not a symlink".into());
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options
        .open(path)
        .map_err(|_| "Cannot open selected input")?;
    if !file
        .metadata()
        .map_err(|_| "Cannot inspect input")?
        .is_file()
    {
        return Err("Input must be a regular file".into());
    }
    Ok(file)
}
pub fn read(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    open(path)?
        .take(limit + 1)
        .read_to_end(&mut data)
        .map_err(|_| "Cannot read input")?;
    if data.len() as u64 > limit {
        return Err("Selected input exceeds byte limit".into());
    }
    Ok(data)
}
pub fn publish(
    path: &Path,
    write: impl FnOnce(&mut File) -> std::io::Result<()>,
) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .map_err(|_| "Cannot create private output temporary file")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temp.as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(|_| "Cannot protect output")?;
    }
    write(temp.as_file_mut()).map_err(|e| format!("Output not published: {e}"))?;
    temp.flush()
        .and_then(|_| temp.as_file().sync_all())
        .map_err(|_| "Cannot flush output")?;
    temp.persist_noclobber(path)
        .map_err(|_| "Output exists or could not be published; choose a new path")?;
    Ok(())
}
