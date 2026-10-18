use super::{PathBuf, env, home};
use crate::session_database::Agent;
pub(super) fn configured() -> Vec<(Agent, PathBuf)> {
    let data = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".local/share"));
    [
        (
            Agent::OpenCode,
            "MIILO_OPENCODE_DB",
            data.join("opencode/opencode.db"),
        ),
        (
            Agent::Devin,
            "MIILO_DEVIN_DB",
            data.join("devin/cli/sessions.db"),
        ),
    ]
    .into_iter()
    .map(|(agent, key, default)| {
        (
            agent,
            env::var_os(key).map(PathBuf::from).unwrap_or(default),
        )
    })
    .collect()
}
