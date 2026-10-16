mod args;
mod credentials;
mod output;

use args::{Args, Command, ConnectionsCommand, WorkCommand, parse_target};
use clap::Parser;
use futures_util::StreamExt;
use miilo_adapter_sdk::WorkItem;
use miilo_client::{
    ExplanationPreferences, ExplanationRequest, MiiloClient, StreamEvent, is_authentication_error,
    is_snapshot_required,
};
use serde::Serialize;
use std::{process::ExitCode, time::Duration};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DoctorReport<'a> {
    health: &'a miilo_client::HealthResponse,
    snapshot: &'a miilo_adapter_sdk::ObservatorySnapshot,
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();
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
        Command::Watch => watch(client, args.json).await?,
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

async fn watch(client: &MiiloClient, json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut cursor = None;
    let mut backoff = Duration::from_secs(1);
    loop {
        let mut stream = match client.stream(cursor.as_deref()).await {
            Ok(stream) => stream,
            Err(error) if is_snapshot_required(&error) => {
                cursor = Some(client.snapshot().await?.cursor);
                continue;
            }
            Err(error) if is_authentication_error(&error) => return Err(error.into()),
            Err(error) => {
                eprintln!(
                    "Miilo stream unavailable: {error}; retrying in {}s",
                    backoff.as_secs()
                );
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(30));
                continue;
            }
        };
        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => return Ok(()),
                item = stream.next() => match item {
                    Some(Ok(event)) => {
                        backoff = Duration::from_secs(1);
                        if let StreamEvent::Event { stream_cursor, .. } = &event {
                            cursor = Some(stream_cursor.clone());
                        }
                        if json { output::json_line(&event)?; } else { output::stream_event(&event); }
                    }
                    Some(Err(error)) if is_authentication_error(&error) => return Err(error.into()),
                    Some(Err(error)) => {
                        eprintln!("Miilo stream interrupted: {error}");
                        break;
                    }
                    None => break,
                }
            }
        }
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_secs(30));
    }
}

fn fail(error: impl std::fmt::Display) -> ExitCode {
    eprintln!("miilo: {error}");
    ExitCode::FAILURE
}
