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
