mod args;
mod credentials;
mod output;
mod watch;

use args::{Args, Command, ConnectionsCommand, WorkCommand, parse_target};
use clap::Parser;
use miilo_adapter_sdk::WorkItem;
use miilo_client::{ExplanationPreferences, ExplanationRequest, MiiloClient};
use serde::Serialize;
use std::process::ExitCode;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DoctorReport<'a> {
    health: &'a miilo_client::HealthResponse,
    snapshot: &'a miilo_adapter_sdk::ObservatorySnapshot,
}

#[tokio::main]
async fn main() -> ExitCode {
    let values: Vec<String> = std::env::args().skip(1).collect();
    let local_index = if values.first().is_some_and(|v| v == "local") {
        Some(0)
    } else if values.first().is_some_and(|v| v == "--json")
        && values.get(1).is_some_and(|v| v == "local")
    {
        Some(1)
    } else {
        None
    };
    if let Some(index) = local_index {
        return match miilo_local::execute(&values[index + 1..]).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => fail(error),
        };
    }
    let args = Args::parse();
    if let Command::Local { args: local_args } = &args.command {
        return match miilo_local::execute(local_args).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => fail(error),
        };
    }
    if let Command::Archive { args } = &args.command {
        return match miilo_local::archive::execute(args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => fail(error),
        };
    }
    let token = match credentials::resolve(args.token.clone(), args.token_file.as_deref()) {
        Ok(token) => token,
        Err(error) => return fail(error),
    };
    let client = match MiiloClient::new(&args.endpoint, token) {
        Ok(client) => client,
        Err(error) => return fail(error),
    };
    match run(&client, args).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => fail(error),
    }
}

async fn run(client: &MiiloClient, args: Args) -> Result<(), Box<dyn std::error::Error>> {
    match args.command {
        Command::Local { .. } | Command::Archive { .. } => {
            unreachable!("local commands are dispatched before HTTP configuration")
        }
        Command::Doctor => {
            let health = client.health().await?;
            let snapshot = client.snapshot().await?;
            if args.json {
                output::json(&DoctorReport {
                    health: &health,
                    snapshot: &snapshot,
                })?;
            } else {
                output::doctor(&health, &snapshot);
            }
        }
        Command::Connections {
            command: ConnectionsCommand::List,
        } => {
            let connections = client.connections().await?;
            if args.json {
                output::json(&connections)?;
            } else {
                output::connections(&connections);
            }
        }
        Command::Work {
            command: WorkCommand::List { connection },
        } => {
            let snapshot = client.snapshot().await?;
            let work: Vec<&WorkItem> = snapshot
                .work_items
                .iter()
                .filter(|item| connection.as_ref().is_none_or(|id| &item.provider_id == id))
                .collect();
            if args.json {
                output::json(&work)?;
            } else {
                output::work(&work.into_iter().cloned().collect::<Vec<_>>());
            }
        }
        Command::Work {
            command: WorkCommand::Show { target },
        } => {
            let item = find_work(client, &target).await?;
            if args.json {
                output::json(&item)?;
            } else {
                output::work_item(&item);
            }
        }
        Command::Work {
            command: WorkCommand::Explain { target, question },
        } => {
            let item = find_work(client, &target).await?;
            let bundle = client
                .explain(&ExplanationRequest {
                    provider_id: item.provider_id,
                    work_item_id: item.id,
                    attempt_id: None,
                    actor_id: None,
                    expected_revision: Some(item.revision),
                    question,
                    preferences: ExplanationPreferences::default(),
                })
                .await?;
            if args.json {
                output::json(&bundle)?;
            } else {
                output::explanation(&bundle);
            }
        }
        Command::Watch => watch::watch(client, args.json).await?,
    }
    Ok(())
}

async fn find_work(
    client: &MiiloClient,
    target: &str,
) -> Result<WorkItem, Box<dyn std::error::Error>> {
    let (provider_id, work_item_id) = parse_target(target).map_err(std::io::Error::other)?;
    client
        .snapshot()
        .await?
        .work_items
        .into_iter()
        .find(|item| item.provider_id == provider_id && item.id == work_item_id)
        .ok_or_else(|| format!("work item {target} was not found").into())
}

fn fail(error: impl std::fmt::Display) -> ExitCode {
    eprintln!("miilo: {error}");
    ExitCode::FAILURE
}
