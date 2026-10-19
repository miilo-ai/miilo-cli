//! Native mutations wake authoritative reads; observer reads never wake themselves.
use super::Result;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::Duration,
};
#[cfg(test)]
#[path = "wakeups_tests.rs"]
mod tests;

type Targets = Arc<RwLock<BTreeSet<PathBuf>>>;
pub(super) struct Wakeups {
    watcher: RecommendedWatcher,
    receiver: tokio::sync::mpsc::Receiver<()>,
    paths: BTreeSet<PathBuf>,
    targets: Targets,
}

fn invalidates(event: &Event, targets: &BTreeSet<PathBuf>) -> bool {
    if event.need_rescan() {
        return true;
    }
    // File reads/open/close are observations, not changed authoritative state.
    if event.kind.is_access() {
        return false;
    }
    if event.paths.is_empty() {
        return true;
    }
    event.paths.iter().any(|path| {
        targets.contains(path)
            || (matches!(event.kind, EventKind::Remove(_))
                && targets.iter().any(|target| target.starts_with(path)))
    })
}

impl Wakeups {
    pub(super) fn new(config: &Value) -> Result<Self> {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        let targets = Targets::default();
        let selected = targets.clone();
        let watcher = RecommendedWatcher::new(
            move |result: notify::Result<Event>| {
                let wake = match result {
                    Ok(event) => selected
                        .read()
                        .map_or(true, |paths| invalidates(&event, &paths)),
                    Err(_) => true, // Repair from authoritative state after notification loss.
                };
                if wake {
                    let _ = sender.try_send(());
                }
            },
            notify::Config::default(),
        )
        .map_err(|_| "Cannot initialize native observation notifications")?;
        let mut out = Self {
            watcher,
            receiver,
            paths: Default::default(),
            targets,
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
        let database = path.extension().is_some_and(|e| e == "db");
        {
            let mut targets = self
                .targets
                .write()
                .map_err(|_| "Cannot select notification path")?;
            targets.insert(path.to_path_buf());
            if database {
                // WAL/journal change data. SHM coordinates access; committed data lives in DB/WAL.
                for suffix in ["-wal", "-journal"] {
                    let mut sibling = path.as_os_str().to_os_string();
                    sibling.push(suffix);
                    targets.insert(PathBuf::from(sibling));
                }
            }
        }
        let watched = if database {
            path.parent().ok_or("Missing database directory")?
        } else {
            path
        };
        if !self.paths.insert(watched.to_path_buf()) {
            return Ok(());
        }
        self.watcher
            .watch(watched, RecursiveMode::NonRecursive)
            .map_err(|_| "Cannot watch selected transcript; reconnect explicitly".into())
    }
    pub(super) async fn wait(&mut self) {
        if self.receiver.recv().await.is_some() {
            tokio::time::sleep(Duration::from_millis(75)).await;
            while self.receiver.try_recv().is_ok() {}
        }
    }
}
