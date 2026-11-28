use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "miilo", version, about = "Observe Miilo from a terminal")]
pub struct Args {
    #[arg(
        long,
        visible_alias = "url",
        env = "MIILO_URL",
        default_value = "http://127.0.0.1:3700",
        global = true
    )]
    pub endpoint: String,
    #[cfg(unix)]
    #[arg(long, global = true)]
    pub socket: Option<std::path::PathBuf>,
    #[arg(long, env = "MIILO_TOKEN", hide_env_values = true, global = true)]
    pub token: Option<String>,
    #[arg(
        long,
        env = "MIILO_TOKEN_FILE",
        conflicts_with = "token",
        global = true
    )]
    pub token_file: Option<std::path::PathBuf>,
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Compress, verify and restore explicitly selected artifacts locally.
    Archive {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Observe selected local sessions without a server or account.
    Local {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Inspect or explicitly migrate shared ~/.miilo user settings.
    Config {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Inspect and decide incoming tasks. --state selects a private manual inbox.
    Inbox {
        #[arg(long)]
        state: Option<std::path::PathBuf>,
        #[command(subcommand)]
        command: InboxCommand,
    },
    Doctor,
    Connections {
        #[command(subcommand)]
        command: ConnectionsCommand,
    },
    Work {
        #[command(subcommand)]
        command: WorkCommand,
    },
    Watch,
}

#[derive(Debug, Subcommand)]
pub enum ConnectionsCommand {
    List,
}

#[derive(Debug, Subcommand)]
pub enum WorkCommand {
    List {
        #[arg(long)]
        connection: Option<String>,
    },
    Show {
        target: String,
    },
    Explain {
        target: String,
        #[arg(long)]
        question: Option<String>,
        /// Opt in to metadata-only OTLP/HTTP traces at this full /v1/traces URL.
        #[arg(long)]
        telemetry_endpoint: Option<String>,
        /// Private bearer-token file for the telemetry collector, not the Miilo API.
        #[arg(long, requires = "telemetry_endpoint")]
        telemetry_token_file: Option<std::path::PathBuf>,
    },
}

pub fn parse_target(value: &str) -> Result<(&str, &str), String> {
    let (provider, work_item) = value
        .split_once('/')
        .ok_or_else(|| "work target must be written as <connection>/<work-id>".to_owned())?;
    if provider.is_empty() || work_item.is_empty() {
        return Err("work target must be written as <connection>/<work-id>".to_owned());
    }
    Ok((provider, work_item))
}

#[cfg(test)]
mod tests {
    use super::parse_target;

    #[test]
    fn parses_scoped_work_target() {
        assert_eq!(
            parse_target("factory/task-1").unwrap(),
            ("factory", "task-1")
        );
        assert!(parse_target("task-1").is_err());
    }
}

#[derive(Debug, Clone, Subcommand)]
pub enum InboxCommand {
    /// Dispatch/reconcile accepted work through an explicitly configured executor.
    Dispatch {
        #[arg(long)]
        config: std::path::PathBuf,
    },
    /// Serve the local manual inbox through the standard authenticated Miilo API.
    Serve {
        #[arg(long, default_value = "127.0.0.1:3710")]
        bind: std::net::SocketAddr,
        #[arg(long, conflicts_with = "listen_socket")]
        access_token_file: Option<std::path::PathBuf>,
        #[arg(long, conflicts_with = "access_token_file")]
        listen_socket: Option<std::path::PathBuf>,
    },
    List,
    Show {
        target: String,
    },
    /// Explicitly import a submission JSON file into the local manual inbox.
    Import {
        file: std::path::PathBuf,
    },
    /// Invoke an advertised action; request/revision IDs bind the exact decision.
    Decide {
        target: String,
        #[arg(long, value_parser=["accept", "defer", "dismiss"])]
        action: String,
        #[arg(long)]
        revision: String,
        #[arg(long)]
        request_id: String,
        /// Reuse this exact timestamp when retrying an uncertain request.
        #[arg(long)]
        confirmed_at: String,
        #[arg(long, default_value="manual", value_parser=["manual", "voice"])]
        method: String,
        #[arg(long)]
        until: Option<String>,
    },
}
