//! Atomic file adapter. Local OS ownership is the authority boundary; no network listener.
use crate::domain::State;
use fs2::FileExt;
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
pub struct Files {
    root: PathBuf,
}
impl Files {
    #[cfg(not(unix))]
    pub fn new(_root: &Path) -> Result<Self, String> {
        Err("The private local inbox currently requires Unix permission semantics. Connect to an existing intake over HTTPS on this platform.".into())
    }
    #[cfg(unix)]
    pub fn new(root: &Path) -> Result<Self, String> {
        if !root.is_absolute() {
            return Err("Intake state path must be absolute".into());
        }
        if !root.exists() {
            fs::create_dir_all(root).map_err(|_| "Cannot create intake state")?;
            #[cfg(unix)]
            fs::set_permissions(root, fs::Permissions::from_mode(0o700))
                .map_err(|_| "Cannot secure intake state")?;
        }
        let meta = fs::symlink_metadata(root).map_err(|_| "Cannot inspect intake state")?;
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return Err("Intake state must be a real directory".into());
        }
        #[cfg(unix)]
        if meta.permissions().mode() & 0o077 != 0 {
            return Err("Intake directory must be private (mode 700)".into());
        }
        Ok(Self {
            root: root.to_owned(),
        })
    }
    pub fn transaction<T>(
        &self,
        f: impl FnOnce(&mut State) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut opts = OpenOptions::new();
        opts.create(true).read(true).write(true).truncate(false);
        #[cfg(unix)]
        opts.mode(0o600).custom_flags(libc::O_NOFOLLOW);
        let lock = opts
            .open(self.root.join("state.lock"))
            .map_err(|_| "Cannot open intake lock")?;
        if !lock
            .metadata()
            .map_err(|_| "Cannot inspect intake lock")?
            .is_file()
        {
            return Err("Intake lock must be a regular file".into());
        }
        lock.lock_exclusive().map_err(|_| "Cannot lock intake")?;
        let path = self.root.join("state.json");
        let mut state = if fs::symlink_metadata(&path).is_ok() {
            let meta =
                fs::symlink_metadata(&path).map_err(|_| "Cannot inspect intake state file")?;
            if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 16 * 1024 * 1024 {
                return Err("Invalid intake state file".into());
            }
            #[cfg(unix)]
            if meta.permissions().mode() & 0o077 != 0 {
                return Err("Intake state file is not private".into());
            }
            let mut opts = OpenOptions::new();
            opts.read(true);
            #[cfg(unix)]
            opts.custom_flags(libc::O_NOFOLLOW);
            let file = opts.open(&path).map_err(|_| "Cannot open intake state")?;
            let mut bytes = Vec::new();
            file.take(16 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| "Cannot read intake")?;
            if bytes.len() > 16 * 1024 * 1024 {
                return Err("Intake state exceeds size bound".into());
            }
            serde_json::from_slice::<State>(&bytes).map_err(|_| "Invalid intake state")?
        } else {
            State {
                instance_id: uuid::Uuid::new_v4().to_string(),
                ..State::default()
            }
        };
        if state.version != 1 || uuid::Uuid::parse_str(&state.instance_id).is_err() {
            return Err("Unsupported intake state version or identity".into());
        }
        let before = serde_json::to_vec(&state).map_err(|_| "Cannot encode intake state")?;
        let result = f(&mut state)?;
        let bytes = serde_json::to_vec(&state).map_err(|_| "Cannot encode intake state")?;
        if bytes == before && path.exists() {
            return Ok(result);
        }
        if bytes.len() > 16 * 1024 * 1024 {
            return Err("Intake state capacity reached".into());
        }
        let mut temp =
            tempfile::NamedTempFile::new_in(&self.root).map_err(|_| "Cannot stage intake state")?;
        temp.write_all(&bytes)
            .and_then(|_| temp.as_file().sync_all())
            .map_err(|_| "Cannot persist intake state")?;
        temp.persist(&path)
            .map_err(|_| "Cannot publish intake state")?;
        File::open(&self.root)
            .and_then(|f| f.sync_all())
            .map_err(|_| "Intake outcome uncertain; inspect state before retry")?;
        Ok(result)
    }
}

impl crate::ports::Repository for Files {
    fn transaction<T>(
        &self,
        operation: impl FnOnce(&mut State) -> Result<T, String>,
    ) -> Result<T, String> {
        Files::transaction(self, operation)
    }
}
