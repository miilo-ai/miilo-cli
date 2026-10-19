use super::{Agent, Result, authorized_path, open};
use serde_json::{Value, json};
use std::path::Path;
pub fn sessions(path: &Path, agent: Agent) -> Result<Vec<Value>> {
    let root = path.parent().ok_or("Missing database root")?;
    let path = authorized_path(path, root, agent)?;
    let db = open(&path)?;
    let sql = match agent {
        Agent::OpenCode => {
            "SELECT substr(id,1,256),substr(title,1,280),substr(directory,1,4096),time_created,time_updated FROM session WHERE time_archived IS NULL AND length(id)<=256 ORDER BY time_updated DESC LIMIT 100"
        }
        Agent::Devin => {
            "SELECT substr(id,1,256),substr(coalesce(title,'Local session'),1,280),substr(working_directory,1,4096),created_at,last_activity_at FROM sessions WHERE hidden=0 AND length(id)<=256 ORDER BY last_activity_at DESC LIMIT 100"
        }
        Agent::Hermes => {
            "SELECT substr(id,1,256),substr(coalesce(title,'Local session'),1,280),substr(cwd,1,4096),cast(started_at*1000 AS INTEGER),cast(coalesce((SELECT max(timestamp) FROM messages WHERE session_id=s.id AND active=1),started_at)*1000 AS INTEGER) FROM sessions s WHERE source='cli' AND archived=0 AND length(id)<=256 ORDER BY started_at DESC LIMIT 100"
        }
    };
    let mut statement = db
        .prepare(sql)
        .map_err(|_| "Unsupported session database schema")?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })
        .map_err(|_| "Cannot discover local sessions")?;
    let mut found = vec![];
    for row in rows {
        let (id, name, cwd, created, updated) = row.map_err(|_| "Invalid session metadata")?;
        if id.is_empty() || id.chars().any(char::is_control) || !Path::new(&cwd).is_absolute() {
            continue;
        }
        let timestamp = |n| {
            match agent {
                Agent::OpenCode | Agent::Hermes => chrono::DateTime::from_timestamp_millis(n),
                Agent::Devin => chrono::DateTime::from_timestamp(n, 0),
            }
            .map(|t| t.to_rfc3339())
        };
        found.push(json!({"kind":agent.kind(),"id":id,"name":name.chars().filter(|c|!c.is_control()).collect::<String>(),"cwd":cwd,"createdAt":timestamp(created),"updatedAt":timestamp(updated),"observable":true,"path":path,"root":path.parent()}));
    }
    Ok(found)
}
