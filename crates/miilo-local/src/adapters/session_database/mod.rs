//! Read-only adapters for version-qualified local session databases.
mod discovery;
mod queries;
mod reader;
pub use discovery::sessions;
pub use reader::DatabaseReader;
use rusqlite::{Connection, OpenFlags};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
type Result<T> = std::result::Result<T, String>;
#[derive(Clone, Copy)]
pub enum Agent {
    OpenCode,
    Devin,
}
impl Agent {
    pub fn parse(kind: &str) -> Result<Self> {
        match kind {
            "opencode" => Ok(Self::OpenCode),
            "devin" => Ok(Self::Devin),
            _ => Err("Unsupported database adapter".into()),
        }
    }
    pub fn kind(self) -> &'static str {
        match self {
            Self::OpenCode => "opencode",
            Self::Devin => "devin",
        }
    }
    pub fn filename(self) -> &'static str {
        match self {
            Self::OpenCode => "opencode.db",
            Self::Devin => "sessions.db",
        }
    }
}
pub fn authorized_path(path: &Path, root: &Path, agent: Agent) -> Result<PathBuf> {
    if fs::symlink_metadata(path)
        .map_err(|_| "Session database unavailable")?
        .file_type()
        .is_symlink()
    {
        return Err("Session database must not be a symlink".into());
    }
    let selected = path
        .canonicalize()
        .map_err(|_| "Session database unavailable")?;
    let root = root
        .canonicalize()
        .map_err(|_| "Session database root unavailable")?;
    if selected.parent() != Some(root.as_path())
        || selected.file_name().and_then(|s| s.to_str()) != Some(agent.filename())
        || !selected.is_file()
    {
        return Err("Session database outside selected storage".into());
    }
    Ok(selected)
}
fn open(path: &Path) -> Result<Connection> {
    let db = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_NOFOLLOW
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|_| "Cannot open session database read-only")?;
    db.busy_timeout(Duration::from_millis(200))
        .map_err(|_| "Cannot bound database reads")?;
    db.pragma_update(None, "query_only", true)
        .map_err(|_| "Cannot enforce read-only observation")?;
    Ok(db)
}
