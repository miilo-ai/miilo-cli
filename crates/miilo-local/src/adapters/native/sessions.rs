use super::*;
pub(super) fn provider_id(kind: &str, id: &str) -> String {
    format!("{kind}-{:x}", Sha256::digest(id.as_bytes()))[..kind.len() + 17].to_owned()
}
pub(super) enum Tail {
    Codex(crate::codex::transcript::TranscriptTail),
    Cursor(crate::transcript::TranscriptTail),
    Claude(crate::claude::transcript::TranscriptTail),
    Grok(crate::grok::transcript::TranscriptTail),
    Database(crate::session_database::DatabaseReader),
}
impl Tail {
    pub(super) fn read(&mut self, state: &mut Observation) -> Result<()> {
        match self {
            Self::Codex(t) => t.read(state),
            Self::Cursor(t) => t.read(state),
            Self::Claude(t) => t.read(state),
            Self::Grok(t) => t.read(state),
            Self::Database(t) => t.read(state),
        }
    }
}
pub(super) struct Session {
    pub(super) observation: Observation,
    pub(super) tail: Tail,
    pub(super) groups: Value,
}
pub(super) fn sessions(config: &Value) -> Result<Vec<Session>> {
    let mut out = vec![];
    for item in config["sessions"]
        .as_array()
        .ok_or("No configured sessions")?
    {
        let id = item["id"].as_str().ok_or("Invalid session ID")?;
        let kind = item["kind"].as_str().ok_or("Invalid session kind")?;
        let path = Path::new(item["path"].as_str().ok_or("No selected transcript")?);
        let root = Path::new(item["root"].as_str().ok_or("No transcript root")?);
        let selected = match kind {
            "codex" => discovery::authorized_path(path, root)?,
            "cursor" => crate::cursor::discovery::authorized_path(path, root)?,
            "claude" => crate::claude::discovery::authorized_path(path, root)?,
            "grok" => crate::grok::discovery::authorized_path(path, root)?,
            "opencode" | "devin" | "hermes" => crate::session_database::authorized_path(
                path,
                root,
                crate::session_database::Agent::parse(kind)?,
            )?,
            _ => return Err("Unsupported observation adapter".into()),
        };
        let tail = if kind == "codex" {
            Tail::Codex(crate::codex::transcript::TranscriptTail::new(selected))
        } else if kind == "grok" {
            Tail::Grok(crate::grok::transcript::TranscriptTail::new(selected))
        } else if kind == "claude" {
            Tail::Claude(crate::claude::transcript::TranscriptTail::new(selected))
        } else if kind == "cursor" {
            Tail::Cursor(crate::transcript::TranscriptTail::new(
                selected,
                crate::cursor::transcript::apply,
            ))
        } else {
            Tail::Database(crate::session_database::DatabaseReader::new(
                selected,
                root.to_path_buf(),
                crate::session_database::Agent::parse(kind)?,
                id.into(),
            ))
        };
        let name = format!(
            "{} · {}",
            kind,
            item["name"].as_str().unwrap_or("Local session")
        );
        let mut observation = Observation::new(
            &provider_id(kind, id),
            &name,
            &format!(
                "{kind}_{}",
                if matches!(kind, "opencode" | "devin" | "hermes") {
                    "session"
                } else {
                    "transcript"
                }
            ),
            item,
        );
        // Enrollment or collector restart is not session activity.
        observation.observed_at = "1970-01-01T00:00:00Z".into();
        out.push(Session {
            groups: item["groups"].clone(),
            observation,
            tail,
        });
    }
    if out.is_empty() || out.len() > 100 {
        return Err("Select between one and 100 sessions".into());
    }
    Ok(out)
}
