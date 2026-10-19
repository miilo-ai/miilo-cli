use super::{Agent, Result, authorized_path, open, queries};
use miilo_adapter_sdk::runtime::observation::Observation;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, os::unix::fs::MetadataExt, path::PathBuf};
pub struct DatabaseReader {
    path: PathBuf,
    root: PathBuf,
    agent: Agent,
    id: String,
    identity: Option<(u64, u64)>,
    seen: BTreeMap<String, Vec<u8>>,
}
impl DatabaseReader {
    pub fn new(path: PathBuf, root: PathBuf, agent: Agent, id: String) -> Self {
        Self {
            path,
            root,
            agent,
            id,
            identity: None,
            seen: BTreeMap::new(),
        }
    }
    pub fn read(&mut self, state: &mut Observation) -> Result<()> {
        let path = authorized_path(&self.path, &self.root, self.agent)?;
        if path != self.path {
            return Err("Selected database path changed".into());
        }
        let meta = fs::metadata(&path).map_err(|_| "Cannot inspect selected database")?;
        let identity = (meta.dev(), meta.ino());
        if self.identity.is_some_and(|old| old != identity) {
            return Err("Selected database replaced; reconnect explicitly".into());
        }
        let db = open(&path)?;
        let exists = match self.agent {
            Agent::OpenCode => "SELECT count(*) FROM session WHERE id=?1 AND time_archived IS NULL",
            Agent::Devin => "SELECT count(*) FROM sessions WHERE id=?1 AND hidden=0",
            Agent::Hermes => {
                "SELECT count(*) FROM sessions WHERE id=?1 AND source='cli' AND archived=0"
            }
        };
        let found: i64 = db
            .query_row(exists, [&self.id], |r| r.get(0))
            .map_err(|_| "Unsupported session schema")?;
        if found != 1 {
            return Err("Selected session unavailable; reconnect explicitly".into());
        }
        let mut rows = vec![];
        let sqls = match self.agent {
            Agent::OpenCode => vec![queries::OPENCODE, queries::OPENCODE_ERRORS],
            Agent::Devin => vec![queries::DEVIN],
            Agent::Hermes => vec![queries::HERMES],
        };
        for sql in sqls {
            let mut statement = db.prepare(sql).map_err(|_| "Unsupported message schema")?;
            let values = statement
                .query_map([&self.id], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, Option<String>>(2)?,
                        r.get::<_, String>(3)?,
                    ))
                })
                .map_err(|_| "Cannot read selected session")?;
            for row in values {
                rows.push(row.map_err(|_| "Invalid selected session message")?);
            }
        }
        rows.sort_by(|a, b| {
            a.1.cmp(&b.1).then_with(|| match self.agent {
                Agent::Devin | Agent::Hermes => {
                    a.0.parse::<i64>()
                        .unwrap_or(0)
                        .cmp(&b.0.parse::<i64>().unwrap_or(0))
                }
                Agent::OpenCode => a.0.cmp(&b.0),
            })
        });
        let bootstrap = self.identity.is_none();
        let mut seen = BTreeMap::new();
        for (id, time, text, lifecycle) in rows {
            let Some(text) = text.filter(|s| !s.trim().is_empty()) else {
                continue;
            };
            let text: String = text
                .chars()
                .filter(|c| !c.is_control() || *c == '\n')
                .take(280)
                .collect();
            let digest = Sha256::digest(format!("{time}:{lifecycle}:{text}").as_bytes()).to_vec();
            if self.seen.get(&id) != Some(&digest) {
                let at = match self.agent {
                    Agent::OpenCode | Agent::Hermes => {
                        chrono::DateTime::from_timestamp_millis(time)
                    }
                    Agent::Devin => chrono::DateTime::from_timestamp(time, 0),
                }
                .ok_or("Invalid message timestamp")?
                .to_rfc3339();
                state.public_summary = Some(text.clone());
                state.activity(
                    &at,
                    text,
                    if bootstrap || lifecycle == "waiting" {
                        "waiting"
                    } else {
                        "active"
                    },
                );
            }
            seen.insert(id, digest);
        }
        self.seen = seen;
        self.identity = Some(identity);
        Ok(())
    }
}
