use crate::{adapters::server::Shared, domain::PairRequest};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;
type Response = (StatusCode, Json<Value>);
fn error(code: StatusCode, message: &str) -> Response {
    (code, Json(json!({"error":message})))
}
pub async fn health() -> Json<Value> {
    Json(json!({"status":"ok","readOnly":true,"protocolVersion":"1.0"}))
}
pub async fn begin(State(s): State<Arc<Shared>>, Json(body): Json<PairRequest>) -> Response {
    let mut host = s.host.lock().unwrap();
    match host.begin(&body.secret, &body.name) {
        Ok((poll, code, expires_at)) => (
            StatusCode::ACCEPTED,
            Json(json!({"poll":poll,"code":code,"expiresAt":expires_at})),
        ),
        Err(_) => error(
            StatusCode::FORBIDDEN,
            "Invitation unavailable or request limit reached",
        ),
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Poll {
    poll: String,
}
pub async fn poll(State(s): State<Arc<Shared>>, Json(body): Json<Poll>) -> Response {
    if body.poll.len() != 64 {
        return error(StatusCode::FORBIDDEN, "Invalid pairing request");
    }
    let host = s.host.lock().unwrap();
    match host.poll(&body.poll) {
        Err(_) => error(StatusCode::FORBIDDEN, "Pairing request expired"),
        Ok(p) if p.denied => (StatusCode::OK, Json(json!({"status":"denied"}))),
        Ok(p) => {
            if let Some((token, device)) = p
                .token
                .as_ref()
                .zip(host.ledger.devices.iter().find(|d| d.id == p.id))
            {
                return (
                    StatusCode::OK,
                    Json(
                        json!({"status":"approved","token":token,"deviceId":device.id,"expiresAt":device.expires_at}),
                    ),
                );
            }
            (StatusCode::OK, Json(json!({"status":"pending"})))
        }
    }
}
pub async fn snapshot(State(s): State<Arc<Shared>>, headers: HeaderMap) -> Response {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    if token.len() != 64 || !s.host.lock().unwrap().authorized(token) {
        return error(
            StatusCode::UNAUTHORIZED,
            "Device unavailable, expired or revoked",
        );
    }
    let source = s.snapshot.clone();
    match tokio::task::spawn_blocking(move || source.read()).await {
        Ok(Ok(value)) => (StatusCode::OK, Json(value)),
        _ => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Selected observations unavailable or stale",
        ),
    }
}
