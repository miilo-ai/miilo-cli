mod error;
mod models;
mod reconcile;
mod sse;
pub use reconcile::ObservationUpdate;

pub use error::ClientError;
pub use models::{
    ConnectionSummary, ConnectionsResponse, ExplanationAction, ExplanationBundle,
    ExplanationChange, ExplanationConfidence, ExplanationDetail, ExplanationEvidenceRef,
    ExplanationLead, ExplanationPreferences, ExplanationRequest, HealthResponse, StreamEvent,
};

use futures_util::Stream;
use miilo_adapter_sdk::ObservatorySnapshot;
use reqwest::RequestBuilder;
use serde::de::DeserializeOwned;
use std::pin::Pin;
use url::Url;

#[derive(Clone)]
pub struct MiiloClient {
    endpoint: Url,
    bearer_token: Option<String>,
    http: reqwest::Client,
}

impl MiiloClient {
    pub fn new(endpoint: &str, bearer_token: Option<String>) -> Result<Self, ClientError> {
        let mut endpoint = Url::parse(endpoint)?;
        if !matches!(endpoint.scheme(), "http" | "https") {
            return Err(ClientError::UnsupportedEndpointScheme(
                endpoint.scheme().to_owned(),
            ));
        }
        if !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
        {
            return Err(ClientError::UnsafeEndpoint);
        }
        if !endpoint.path().ends_with('/') {
            endpoint.set_path(&format!("{}/", endpoint.path()));
        }
        Ok(Self {
            endpoint,
            bearer_token,
            http: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(std::time::Duration::from_secs(5))
                .read_timeout(std::time::Duration::from_secs(45))
                .build()?,
        })
    }

    pub async fn health(&self) -> Result<HealthResponse, ClientError> {
        self.get_json("health").await
    }

    pub async fn connections(&self) -> Result<ConnectionsResponse, ClientError> {
        self.get_json("v1/connections").await
    }

    pub async fn snapshot(&self) -> Result<ObservatorySnapshot, ClientError> {
        self.get_json("v1/snapshot").await
    }

    pub async fn explain(
        &self,
        request: &ExplanationRequest,
    ) -> Result<ExplanationBundle, ClientError> {
        let url = self.endpoint.join("v1/explanations")?;
        self.send_json(self.authorize(self.http.post(url)).json(request))
            .await
    }

    pub async fn stream(
        &self,
        cursor: Option<&str>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, ClientError>> + Send>>, ClientError>
    {
        let url = self.endpoint.join("v1/stream")?;
        let mut request = self.authorize(self.http.get(url));
        if let Some(cursor) = cursor {
            request = request.header("last-event-id", cursor);
        }
        let response = request.send().await?;
        let response = ensure_success(response, self.bearer_token.as_deref()).await?;
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        if !content_type.starts_with("text/event-stream") {
            return Err(ClientError::UnexpectedContentType(content_type.to_owned()));
        }
        Ok(sse::decode(response.bytes_stream()))
    }

    async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, ClientError> {
        let url = self.endpoint.join(path)?;
        self.send_json(self.authorize(self.http.get(url))).await
    }

    async fn send_json<T: DeserializeOwned>(
        &self,
        request: RequestBuilder,
    ) -> Result<T, ClientError> {
        let response = ensure_success(
            request
                .timeout(std::time::Duration::from_secs(15))
                .send()
                .await?,
            self.bearer_token.as_deref(),
        )
        .await?;
        let bytes = bounded_body(response, 2 * 1024 * 1024).await?;
        serde_json::from_slice(&bytes).map_err(ClientError::from)
    }

    fn authorize(&self, request: RequestBuilder) -> RequestBuilder {
        match &self.bearer_token {
            Some(token) => request.bearer_auth(token),
            None => request,
        }
    }
}

async fn ensure_success(
    response: reqwest::Response,
    token: Option<&str>,
) -> Result<reqwest::Response, ClientError> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let message = match bounded_body(response, 64 * 1024).await {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(_) => status
            .canonical_reason()
            .unwrap_or("remote error response unavailable or oversized")
            .into(),
    };
    Err(ClientError::Http {
        status: status.as_u16(),
        message: redact_credential(message, token),
    })
}

pub fn is_authentication_error(error: &ClientError) -> bool {
    matches!(
        error,
        ClientError::Http {
            status: 401 | 403,
            ..
        }
    )
}

pub fn is_snapshot_required(error: &ClientError) -> bool {
    matches!(
        error,
        ClientError::Http {
            status,
            ..
        } if *status == 409
    )
}

fn redact_credential(message: String, token: Option<&str>) -> String {
    match token.filter(|value| !value.is_empty()) {
        Some(token) => message.replace(token, "[redacted]"),
        None => message,
    }
}

async fn bounded_body(
    mut response: reqwest::Response,
    limit: usize,
) -> Result<Vec<u8>, ClientError> {
    let mut out = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if out.len().saturating_add(chunk.len()) > limit {
            return Err(ClientError::ResponseTooLarge);
        }
        out.extend_from_slice(&chunk);
    }
    Ok(out)
}

#[cfg(test)]
mod credential_errors {
    #[test]
    fn remote_errors_cannot_echo_the_configured_credential() {
        let message = super::redact_credential(
            "Rejected secret-token; header Bearer secret-token".into(),
            Some("secret-token"),
        );
        assert!(!message.contains("secret-token"));
        assert!(message.contains("Rejected"));
    }
}
