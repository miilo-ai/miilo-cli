use futures_util::StreamExt;
use miilo_client::{ClientError, MiiloClient};
use serde_json::json;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};
struct Server {
    port: u16,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
    events: Arc<Mutex<Option<TcpStream>>>,
    calls: Arc<Mutex<Vec<String>>>,
}
impl Server {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let events = Arc::new(Mutex::new(None));
        let calls = Arc::new(Mutex::new(vec![]));
        let stopping = stop.clone();
        let event_writer = events.clone();
        let log = calls.clone();
        let revision = Arc::new(AtomicUsize::new(0));
        let worker = thread::spawn(move || {
            while !stopping.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut socket, _)) => {
                        socket.set_nonblocking(false).unwrap();
                        socket
                            .set_read_timeout(Some(Duration::from_secs(2)))
                            .unwrap();
                        let mut data = Vec::new();
                        let mut buf = [0u8; 2048];
                        while !data.windows(4).any(|b| b == b"\r\n\r\n") {
                            let Ok(n) = socket.read(&mut buf) else { break };
                            if n == 0 {
                                break;
                            }
                            data.extend_from_slice(&buf[..n]);
                        }
                        let path = String::from_utf8_lossy(&data)
                            .split_whitespace()
                            .nth(1)
                            .unwrap_or("")
                            .to_owned();
                        if path == "/v1/stream" {
                            log.lock().unwrap().push("subscribe".into());
                            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n:connected\n\n").unwrap();
                            *event_writer.lock().unwrap() = Some(socket);
                        } else if path == "/v1/snapshot" {
                            log.lock().unwrap().push("snapshot".into());
                            let sequence = revision.fetch_add(1, Ordering::SeqCst) + 1;
                            let body=json!({"protocolVersion":"0.1","snapshotId":sequence.to_string(),"observedAt":"2026-10-16T00:00:00Z","cursor":format!("boot:{sequence}"),"providers":[],"workItems":[],"actors":[],"attempts":[],"artifacts":[],"evidence":[],"attention":[]}).to_string();
                            write!(socket,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2))
                    }
                    Err(e) => panic!("{e}"),
                }
            }
        });
        Self {
            port,
            stop,
            worker: Some(worker),
            events,
            calls,
        }
    }
    fn send(&self, frame: &str) {
        self.events
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .write_all(frame.as_bytes())
            .unwrap();
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.worker.take().unwrap().join().unwrap();
    }
}
#[tokio::test]
async fn subscription_precedes_fresh_bootstrap_and_reconnect_never_reuses_cache() {
    let server = Server::new();
    let client = MiiloClient::new(&format!("http://127.0.0.1:{}", server.port), None).unwrap();
    let mut updates = client.observe().await.unwrap();
    let first = updates.next().await.unwrap().unwrap();
    assert_eq!(first.reason, "bootstrap");
    assert_eq!(first.snapshot.snapshot_id, "1");
    assert_eq!(*server.calls.lock().unwrap(), vec!["subscribe", "snapshot"]);
    let event = json!({"protocolVersion":"0.1","eventId":"event","cursor":"boot:2","providerId":"one","occurredAt":"2026-10-16T00:00:00Z","kind":"updated","subject":{"kind":"provider","id":"one"},"summary":"Changed","payload":{}});
    server.send(&format!(
        "id: boot:2\nevent: miilo.event\ndata: {event}\n\n"
    ));
    let next = updates.next().await.unwrap().unwrap();
    assert_eq!(next.reason, "event");
    assert_eq!(next.snapshot.snapshot_id, "2");
    drop(updates);
    let mut updates = client.observe().await.unwrap();
    assert_eq!(
        updates.next().await.unwrap().unwrap().snapshot.snapshot_id,
        "3"
    );
    server.send("event: miilo.error\ndata: {\"reason\":\"snapshot_required\",\"message\":\"Lost history\"}\n\n");
    assert!(matches!(
        updates.next().await,
        Some(Err(ClientError::SnapshotRequired))
    ));
}
