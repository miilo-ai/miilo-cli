//! Authoritative snapshot reconciliation. Events invalidate, never patch arbitrary state.
use crate::{ClientError, MiiloClient, StreamEvent};
use futures_util::{Stream, StreamExt, stream};
use miilo_adapter_sdk::ObservatorySnapshot;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationUpdate {
    pub snapshot: ObservatorySnapshot,
    pub reason: String,
}
impl MiiloClient {
    /// Opens the subscription before reading state. Every new subscription has a fresh
    /// bootstrap, and every event triggers an authoritative read rather than a patch.
    pub async fn observe(
        &self,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<ObservationUpdate, ClientError>> + Send>>,
        ClientError,
    > {
        let events = self.stream(None).await?;
        let initial = ObservationUpdate {
            snapshot: self.snapshot().await?,
            reason: "bootstrap".into(),
        };
        let client = self.clone();
        let updates = stream::unfold((client, events), |(client, mut events)| async move {
            let event = events.next().await?;
            let update = match event {
                Ok(StreamEvent::Event { .. }) => {
                    client.snapshot().await.map(|snapshot| ObservationUpdate {
                        snapshot,
                        reason: "event".into(),
                    })
                }
                Ok(StreamEvent::Error { reason, .. })
                    if reason == "unauthorized" || reason == "authentication_expired" =>
                {
                    Err(ClientError::Http {
                        status: 401,
                        message: "Stream access expired".into(),
                    })
                }
                Ok(StreamEvent::Error { .. }) => Err(ClientError::SnapshotRequired),
                Err(error) => Err(error),
            };
            Some((update, (client, events)))
        });
        Ok(Box::pin(
            stream::once(async move { Ok(initial) }).chain(updates),
        ))
    }
}
