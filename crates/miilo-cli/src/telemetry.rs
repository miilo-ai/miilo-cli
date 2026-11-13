use crate::{
    args::{Command, WorkCommand},
    credentials,
};
use miilo_client::ClientError;
use miilo_telemetry::{Config, ErrorKind, Outcome, Telemetry};

pub async fn start(command: &Command) -> Result<Option<Telemetry>, &'static str> {
    let Command::Work {
        command:
            WorkCommand::Explain {
                telemetry_endpoint: Some(endpoint),
                telemetry_token_file,
                ..
            },
    } = command
    else {
        return Ok(None);
    };
    let token = credentials::resolve(None, telemetry_token_file.as_deref())?;
    let config =
        Config::new(endpoint, token).map_err(|_| "invalid telemetry endpoint or credential")?;
    tokio::task::spawn_blocking(move || Telemetry::start(config))
        .await
        .map_err(|_| "telemetry initialization failed")?
        .map(Some)
}

pub async fn shutdown(telemetry: Option<Telemetry>) {
    if let Some(telemetry) = telemetry {
        match tokio::task::spawn_blocking(move || telemetry.shutdown()).await {
            Ok(Ok(())) => {}
            _ => eprintln!("miilo: telemetry shutdown failed; work result is unchanged"),
        }
    }
}

pub fn outcome(error: &ClientError) -> Outcome {
    Outcome::Error(match error {
        ClientError::Transport(error) if error.is_timeout() => ErrorKind::Timeout,
        ClientError::InvalidPayload(_) | ClientError::ResponseTooLarge => {
            ErrorKind::InvalidResponse
        }
        ClientError::Http { .. } | ClientError::Transport(_) => ErrorKind::Provider,
        _ => ErrorKind::Other,
    })
}
