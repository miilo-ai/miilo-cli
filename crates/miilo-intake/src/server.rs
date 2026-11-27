//! API adapter: bearer-protected loopback HTTP or owner-only Unix sockets.
use crate::{
    domain::Submission,
    service::{Manual, Service},
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post},
};
use futures_util::stream;
use miilo_adapter_sdk::{
    ActionInvocation, EventKind, ObservatoryEvent, SubjectKind, SubjectReference,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{convert::Infallible, net::SocketAddr, path::Path, sync::Arc, time::Duration};
use subtle::ConstantTimeEq;
#[derive(Clone)]
struct Context {
    service: Arc<dyn Service>,
    token_hash: Option<[u8; 32]>,
}
type Error = (StatusCode, Json<Value>);
fn error(status: StatusCode, message: &str) -> Error {
    (status, Json(json!({"error":message})))
}
fn authorize(c: &Context, headers: &HeaderMap) -> Result<(), Error> {
    let Some(expected) = &c.token_hash else {
        return Ok(());
    };
    let bearer = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "Authentication required"))?;
    let hash: [u8; 32] = Sha256::digest(bearer.as_bytes()).into();
    if hash.ct_eq(expected).unwrap_u8() != 1 {
        return Err(error(StatusCode::UNAUTHORIZED, "Authentication required"));
    }
    Ok(())
}
async fn call<T: Send + 'static>(
    c: Context,
    f: impl FnOnce(&dyn Service) -> Result<T, String> + Send + 'static,
) -> Result<T, Error> {
    tokio::task::spawn_blocking(move || f(c.service.as_ref()))
        .await
        .map_err(|_| {
            error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Intake operation interrupted",
            )
        })?
        .map_err(|_| {
            error(
                StatusCode::CONFLICT,
                "Intake operation could not complete; refresh and reconcile the request",
            )
        })
}
async fn health(State(c): State<Context>, h: HeaderMap) -> Result<Json<Value>, Error> {
    authorize(&c, &h)?;
    let snapshot = call(c, |s| s.snapshot()).await?;
    Ok(Json(
        json!({"ok":true,"service":"miilo-intake","protocolVersion":"0.1","providers":snapshot.providers.len(),"identity":true,"storage":true}),
    ))
}
async fn connections(State(c): State<Context>, h: HeaderMap) -> Result<Json<Value>, Error> {
    authorize(&c, &h)?;
    let snapshot = call(c, |s| s.snapshot()).await?;
    Ok(Json(
        json!({"connections":snapshot.providers.into_iter().map(|p|json!({"id":p.id,"name":p.name,"adapter":"intake","enabled":true,"credentialConfigured":true})).collect::<Vec<_>>()}),
    ))
}
async fn snapshot(State(c): State<Context>, h: HeaderMap) -> Result<Json<Value>, Error> {
    authorize(&c, &h)?;
    let snapshot = call(c, |s| s.snapshot()).await?;
    Ok(Json(serde_json::to_value(snapshot).map_err(|_| {
        error(StatusCode::INTERNAL_SERVER_ERROR, "Cannot encode snapshot")
    })?))
}
async fn invoke(
    State(c): State<Context>,
    h: HeaderMap,
    Json(request): Json<ActionInvocation>,
) -> Result<Json<Value>, Error> {
    authorize(&c, &h)?;
    let result = call(c, move |s| s.invoke(request)).await?;
    Ok(Json(serde_json::to_value(result).map_err(|_| {
        error(StatusCode::INTERNAL_SERVER_ERROR, "Cannot encode receipt")
    })?))
}
async fn import(
    State(c): State<Context>,
    h: HeaderMap,
    Json(request): Json<Submission>,
) -> Result<Json<Value>, Error> {
    authorize(&c, &h)?;
    call(c, move |s| s.import(request))
        .await
        .map(|imported| Json(json!({"imported":imported})))
}
async fn events(
    State(c): State<Context>,
    h: HeaderMap,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>, Error> {
    authorize(&c, &h)?;
    let snapshot = call(c.clone(), |s| s.snapshot()).await?;
    let last = h
        .get("last-event-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    if last.as_ref().is_some_and(|v| v != &snapshot.cursor) {
        return Err(error(
            StatusCode::CONFLICT,
            "Fetch a current snapshot and resubscribe",
        ));
    }
    let stream = stream::unfold((c, last), |(c, mut last)| async move {
        loop {
            let Ok(snapshot) = call(c.clone(), |s| s.snapshot()).await else {
                return None;
            };
            let cursor = snapshot.cursor;
            if last.as_ref() != Some(&cursor) {
                let event = ObservatoryEvent {
                    protocol_version: "0.1".into(),
                    event_id: format!("intake-{cursor}"),
                    cursor: cursor.clone(),
                    provider_id: "intake".into(),
                    occurred_at: chrono::Utc::now(),
                    kind: EventKind::Updated,
                    subject: SubjectReference {
                        kind: SubjectKind::Provider,
                        id: "intake".into(),
                    },
                    summary: "Inbox changed; fetch current snapshot".into(),
                    payload: serde_json::Map::new(),
                    extensions: None,
                };
                last = Some(cursor.clone());
                let data = serde_json::to_string(&event).ok()?;
                return Some((
                    Ok(Event::default().id(cursor).event("miilo.event").data(data)),
                    (c, last),
                ));
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}
fn router(service: Arc<dyn Service>, token_hash: Option<[u8; 32]>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/connections", get(connections))
        .route("/v1/snapshot", get(snapshot))
        .route("/v1/actions", post(invoke))
        .route("/v1/intake/import", post(import))
        .route("/v1/stream", get(events))
        .layer(DefaultBodyLimit::max(20_000))
        .with_state(Context {
            service,
            token_hash,
        })
}
pub async fn serve(root: &Path, bind: SocketAddr, token_file: &Path) -> Result<(), String> {
    serve_backend(
        Arc::new(Manual {
            root: root.to_owned(),
        }),
        bind,
        token_file,
    )
    .await
}
pub async fn serve_backend(
    service: Arc<dyn Service>,
    bind: SocketAddr,
    token_file: &Path,
) -> Result<(), String> {
    if !bind.ip().is_loopback() {
        return Err("The reference intake server binds loopback only".into());
    }
    let token = crate::credential::read(token_file)?;
    let hash = Sha256::digest(token.as_bytes()).into();
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(|_| "Cannot bind intake listener")?;
    axum::serve(listener, router(service, Some(hash)))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .map_err(|_| "Intake listener failed".into())
}
#[cfg(unix)]
pub async fn serve_unix(service: Arc<dyn Service>, socket: &Path) -> Result<(), String> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    if !socket.is_absolute() {
        return Err("Socket path must be absolute".into());
    }
    let parent = socket.parent().ok_or("Missing socket directory")?;
    let meta =
        std::fs::symlink_metadata(parent).map_err(|_| "Create a private socket directory first")?;
    if !meta.is_dir()
        || meta.file_type().is_symlink()
        || meta.permissions().mode() & 0o077 != 0
        || meta.uid() != unsafe { libc::geteuid() }
    {
        return Err("Socket directory must be owner-only (mode 700)".into());
    }
    if std::fs::symlink_metadata(socket).is_ok() {
        return Err("Socket path already exists; reconcile its owner before reuse".into());
    }
    let listener =
        tokio::net::UnixListener::bind(socket).map_err(|_| "Cannot bind intake socket")?;
    std::fs::set_permissions(socket, std::fs::Permissions::from_mode(0o600))
        .map_err(|_| "Cannot secure intake socket")?;
    let socket_meta =
        std::fs::symlink_metadata(socket).map_err(|_| "Cannot inspect bound intake socket")?;
    let inode = socket_meta.ino();
    let app = router(service, None);
    let result = axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .map_err(|_| "Intake socket listener failed".into());
    // A private directory prevents other identities replacing this socket.
    if std::fs::symlink_metadata(socket).is_ok_and(|meta| meta.ino() == inode) {
        let _ = std::fs::remove_file(socket);
    }
    result
}
