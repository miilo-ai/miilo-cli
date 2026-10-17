use crate::output;
use futures_util::StreamExt;
use miilo_client::{MiiloClient, is_authentication_error, is_snapshot_required};
use std::time::Duration;
pub async fn watch(client: &MiiloClient, json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut backoff = Duration::from_secs(1);
    loop {
        let mut stream = match client.observe().await {
            Ok(stream) => stream,
            Err(error) if is_snapshot_required(&error) => {
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
                        if json { output::json_line(&serde_json::json!({"type":"snapshot","reason":event.reason,"snapshot":event.snapshot}))?; } else {
                            println!("{}: {} providers, {} work items",event.reason,event.snapshot.providers.len(),event.snapshot.work_items.len());
                            output::work(&event.snapshot.work_items);
                        }
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
