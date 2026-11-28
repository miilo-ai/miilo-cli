mod args;
mod credentials;
mod inbox;
mod output;
mod telemetry;
mod watch;
mod work;

use args::{Args, Command, ConnectionsCommand, WorkCommand};
use clap::Parser;
use miilo_adapter_sdk::WorkItem;
use miilo_client::{ExplanationPreferences, ExplanationRequest, MiiloClient};
use serde::Serialize;
use std::process::ExitCode;
use work::find_work;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DoctorReport<'a> {
    health: &'a miilo_client::HealthResponse,
    snapshot: &'a miilo_adapter_sdk::ObservatorySnapshot,
}

#[tokio::main]
async fn main() -> ExitCode {
    let values: Vec<String> = std::env::args().skip(1).collect();
    let archive_index = if values.first().is_some_and(|v| v == "archive") {
        Some(0)
    } else if values.first().is_some_and(|v| v == "--json")
        && values.get(1).is_some_and(|v| v == "archive")
    {
        Some(1)
    } else {
        None
    };
    if let Some(index) = archive_index {
        return match miilo_local::archive::execute(&values[index + 1..]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => fail(error),
        };
    }
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
    if let Command::Config { args: config_args } = &args.command {
        let mut local_args = vec!["config".to_owned()];
        local_args.extend(config_args.iter().cloned());
        return match miilo_local::execute(&local_args).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => fail(error),
        };
    }
    if let Command::Inbox {
        state,
        command:
            args::InboxCommand::Serve {
                bind,
                access_token_file,
                listen_socket,
            },
    } = &args.command
    {
        let Some(path) = state else {
            return fail("inbox serve requires --state");
        };
        let result = if let Some(socket) = listen_socket {
            #[cfg(unix)]
            {
                miilo_intake::server::serve_unix(
                    std::sync::Arc::new(miilo_intake::service::Manual { root: path.clone() }),
                    socket,
                )
                .await
            }
            #[cfg(not(unix))]
            {
                let _ = socket;
                Err("Unix sockets are unavailable on this platform".into())
            }
        } else if let Some(token_file) = access_token_file {
            miilo_intake::server::serve(path, *bind, token_file).await
        } else {
            Err("Choose --access-token-file for loopback HTTP or --listen-socket for a private Unix socket".into())
        };
        return match result {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => fail(error),
        };
    }
    if let Command::Inbox {
        state: Some(path),
        command,
    } = &args.command
    {
        let path = path.clone();
        let command = command.clone();
        let result = tokio::task::spawn_blocking(move || {
            inbox::local(&path, &command).map_err(|e| e.to_string())
        })
        .await;
        return match result.unwrap_or_else(|_| Err("Intake command interrupted".into())) {
            Ok(value) => match output::json(&value) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => fail(error),
            },
            Err(error) => fail(error),
        };
    }
    #[cfg(unix)]
    let using_socket = args.socket.is_some();
    #[cfg(not(unix))]
    let using_socket = false;
    let token = match if using_socket {
        Ok(None)
    } else {
        credentials::resolve(args.token.clone(), args.token_file.as_deref())
    } {
        Ok(token) => token,
        Err(error) => return fail(error),
    };
    #[cfg(unix)]
    let connection = match &args.socket {
        Some(socket) => MiiloClient::unix(socket),
        None => MiiloClient::new(&args.endpoint, token),
    };
    #[cfg(not(unix))]
    let connection = MiiloClient::new(&args.endpoint, token);
    let client = match connection {
        Ok(client) => client,
        Err(error) => return fail(error),
    };
    let telemetry = match telemetry::start(&args.command).await {
        Ok(value) => value,
        Err(error) => return fail(error),
    };
    let result = if telemetry.is_some() {
        tokio::select! {
            result = run(&client, args, telemetry.as_ref()) => result,
            _ = tokio::signal::ctrl_c() => Err("operation cancelled".into()),
        }
    } else {
        run(&client, args, None).await
    };
    telemetry::shutdown(telemetry).await;
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => fail(error),
    }
}

async fn run(
    client: &MiiloClient,
    args: Args,
    telemetry: Option<&miilo_telemetry::Telemetry>,
) -> Result<(), Box<dyn std::error::Error>> {
    match args.command {
        Command::Local { .. } | Command::Archive { .. } | Command::Config { .. } => {
            unreachable!("local commands are dispatched before HTTP configuration")
        }
        Command::Inbox {
            state: None,
            command,
        } => {
            let value = inbox::remote(client, command).await?;
            output::json(&value)?;
        }
        Command::Inbox { state: Some(_), .. } => {
            unreachable!("local inbox dispatched before credentials")
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
            command: WorkCommand::Explain {
                target, question, ..
            },
        } => {
            let item = find_work(client, &target).await?;
            let operation = telemetry.map(|telemetry| telemetry.explanation());
            let result = client
                .explain(&ExplanationRequest {
                    provider_id: item.provider_id,
                    work_item_id: item.id,
                    attempt_id: None,
                    actor_id: None,
                    expected_revision: Some(item.revision),
                    question,
                    preferences: ExplanationPreferences::default(),
                })
                .await;
            if let Some(operation) = operation {
                operation.finish(match &result {
                    Ok(_) => miilo_telemetry::Outcome::Success,
                    Err(error) => telemetry::outcome(error),
                });
            }
            let bundle = result?;
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

fn fail(error: impl std::fmt::Display) -> ExitCode {
    eprintln!("miilo: {error}");
    ExitCode::FAILURE
}
