use crate::{
    Result,
    adapters::{
        files::{self, PrivateStore},
        identity::Identity,
        system::{Random, SystemClock},
    },
    application::{Host, Snapshot},
};
use axum::{
    Router,
    routing::{get, post},
};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    service::TowerToHyperService,
};
use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::{net::TcpListener, sync::Semaphore};

pub type PairHost = Host<PrivateStore, SystemClock, Random>;
pub struct Shared {
    pub host: Mutex<PairHost>,
    pub identity: Identity,
    pub snapshot: Arc<dyn Snapshot + Send + Sync>,
}
pub struct Options {
    pub bind: SocketAddr,
    pub advertised: String,
    pub name: String,
    pub directory: PathBuf,
}

pub async fn run(options: Options, snapshot: Arc<dyn Snapshot + Send + Sync>) -> Result<()> {
    if options.bind.ip().is_unspecified() || options.bind.port() == 0 {
        return Err("Bind to an explicit local or private-network address and port".into());
    }
    files::prepare(&options.directory)?;
    let _lease = lease(&options.directory)?;
    let identity = Identity::load(&options.directory, &options.name, &options.advertised)?;
    let tls = tokio_rustls::TlsAcceptor::from(identity.tls()?);
    let ledger_file = options.directory.join("devices.json");
    let ledger = if ledger_file.exists() {
        files::read(&ledger_file)?
    } else {
        Default::default()
    };
    let shared = Arc::new(Shared {
        host: Mutex::new(Host::new(
            ledger,
            PrivateStore(ledger_file),
            SystemClock,
            Random,
        )),
        identity,
        snapshot,
    });
    let app = Router::new()
        .route("/health", get(super::routes::health))
        .route("/pair/request", post(super::routes::begin))
        .route("/pair/poll", post(super::routes::poll))
        .route("/v1/snapshot", get(super::routes::snapshot))
        .layer(axum::extract::DefaultBodyLimit::max(8192))
        .with_state(shared.clone());
    let listener = TcpListener::bind(options.bind)
        .await
        .map_err(|_| "Cannot bind pairing listener")?;
    #[cfg(unix)]
    let control = super::control::serve(options.directory.join("control.sock"), shared).await?;
    let permits = Arc::new(Semaphore::new(32));
    println!(
        "Read-only sharing active. Run `miilo local pair invite` in another terminal. Only already-selected sessions are shared."
    );
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            incoming = listener.accept() => {
                let (tcp, _) = incoming.map_err(|_| "Pairing accept failed")?;
                let Ok(permit) = permits.clone().try_acquire_owned() else { continue; };
                let tls = tls.clone(); let router = app.clone();
                tokio::spawn(async move {
                    let _permit = permit;
                    let _ = tokio::time::timeout(std::time::Duration::from_secs(10), async move {
                        if let Ok(stream) = tls.accept(tcp).await {
                            let mut builder = hyper_util::server::conn::auto::Builder::new(TokioExecutor::new());
                            builder.http1().keep_alive(false);
                            let _ = builder.serve_connection(TokioIo::new(stream), TowerToHyperService::new(router)).await;
                        }
                    }).await;
                });
            }
        }
    }
    #[cfg(unix)]
    {
        control.abort();
        let _ = std::fs::remove_file(options.directory.join("control.sock"));
    }
    Ok(())
}
fn lease(directory: &std::path::Path) -> Result<std::fs::File> {
    use fs2::FileExt;
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options
        .open(directory.join("host.lock"))
        .map_err(|_| "Cannot open host lock")?;
    if !file
        .metadata()
        .map_err(|_| "Cannot inspect host lock")?
        .is_file()
    {
        return Err("Unsafe host lock".into());
    }
    file.try_lock_exclusive()
        .map_err(|_| "Pairing host already running")?;
    Ok(file)
}
