//! Native notifications wake a full authoritative read; bursts are coalesced.
use super::*;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
pub(super) struct Wakeups {
    watcher: RecommendedWatcher,
    receiver: tokio::sync::mpsc::Receiver<()>,
    paths: std::collections::BTreeSet<PathBuf>,
}
impl Wakeups {
    pub(super) fn new(config: &Value) -> Result<Self> {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        let watcher = RecommendedWatcher::new(
            move |_: notify::Result<notify::Event>| {
                let _ = sender.try_send(());
            },
            notify::Config::default(),
        )
        .map_err(|_| "Cannot initialize native observation notifications")?;
        let mut out = Self {
            watcher,
            receiver,
            paths: Default::default(),
        };
        for session in config["sessions"].as_array().into_iter().flatten() {
            out.watch(Path::new(
                session["path"]
                    .as_str()
                    .ok_or("Missing selected transcript")?,
            ))?;
        }
        Ok(out)
    }
    pub(super) fn watch(&mut self, path: &Path) -> Result<()> {
        // SQLite activity may exist only in the WAL until checkpointing.
        let path = if path.extension().is_some_and(|e| e == "db") {
            path.parent().ok_or("Missing database directory")?
        } else {
            path
        };
        if !self.paths.insert(path.to_path_buf()) {
            return Ok(());
        }
        self.watcher
            .watch(path, RecursiveMode::NonRecursive)
            .map_err(|_| "Cannot watch selected transcript; reconnect explicitly".into())
    }
    pub(super) async fn wait(&mut self) {
        if self.receiver.recv().await.is_some() {
            tokio::time::sleep(Duration::from_millis(75)).await;
        }
    }
}
