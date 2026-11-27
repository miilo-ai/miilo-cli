//! Caller-owned credential-file adapter. Never copy credentials into inbox state.
use std::{
    fs::{self, OpenOptions},
    io::Read,
    path::Path,
};
use zeroize::Zeroizing;
pub fn read(path: &Path) -> Result<Zeroizing<String>, String> {
    let before = fs::symlink_metadata(path).map_err(|_| "Cannot inspect intake credential")?;
    if !before.is_file() || before.file_type().is_symlink() || before.len() > 4096 {
        return Err("Invalid intake credential file".into());
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
        .map_err(|_| "Cannot open intake credential")?;
    let meta = file
        .metadata()
        .map_err(|_| "Cannot inspect open credential")?;
    if !meta.is_file() || meta.len() > 4096 {
        return Err("Invalid intake credential file".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        if meta.permissions().mode() & 0o077 != 0
            || before.dev() != meta.dev()
            || before.ino() != meta.ino()
        {
            return Err("Credential must be a private unchanged regular file (mode 600)".into());
        }
    }
    let mut value = Zeroizing::new(String::new());
    file.take(4097)
        .read_to_string(&mut value)
        .map_err(|_| "Cannot read intake credential")?;
    if value.len() > 4096 {
        return Err("Credential exceeds size bound".into());
    }
    let trimmed = value.trim();
    if trimmed.len() < 32 || trimmed.len() > 1024 || !trimmed.bytes().all(|c| c.is_ascii_graphic())
    {
        return Err("Credential must contain 32–1024 non-whitespace ASCII characters".into());
    }
    Ok(Zeroizing::new(trimmed.to_owned()))
}
