#![allow(dead_code)]
use opentelemetry_proto::tonic::collector::trace::v1::ExportTraceServiceRequest;
use prost::Message;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

#[derive(Clone, Debug)]
pub struct Request {
    pub headers: String,
    pub body: Vec<u8>,
}
impl Request {
    pub fn trace(&self) -> ExportTraceServiceRequest {
        ExportTraceServiceRequest::decode(self.body.as_slice()).unwrap()
    }
}
pub struct Server {
    pub url: String,
    pub requests: Arc<Mutex<Vec<Request>>>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Server {
    pub fn new(statuses: Vec<u16>, delay: Duration, body: Vec<u8>) -> Self {
        Self::respond(delay, move |index, _| {
            (
                statuses
                    .get(index)
                    .or(statuses.last())
                    .copied()
                    .unwrap_or(200),
                body.clone(),
            )
        })
    }
    pub fn respond(
        delay: Duration,
        mut response: impl FnMut(usize, &Request) -> (u16, Vec<u8>) + Send + 'static,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/v1/traces", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let records = requests.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let worker = thread::spawn(move || {
            while !stopping.load(Ordering::Acquire) {
                let Ok((mut stream, _)) = listener.accept() else {
                    thread::sleep(Duration::from_millis(2));
                    continue;
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                let Some(request) = read(&mut stream) else {
                    continue;
                };
                let (status, body) = {
                    let mut records = records.lock().unwrap();
                    let result = response(records.len(), &request);
                    records.push(request);
                    result
                };
                thread::sleep(delay);
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status} Test\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nLocation: http://127.0.0.1:1/private\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(&body);
            }
        });
        Self {
            url,
            requests,
            stop,
            worker: Some(worker),
        }
    }
    pub fn received(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.worker.take().unwrap().join().unwrap();
    }
}
fn read(stream: &mut TcpStream) -> Option<Request> {
    let mut bytes = Vec::new();
    let split = loop {
        let mut chunk = [0; 4096];
        let size = stream.read(&mut chunk).ok()?;
        if size == 0 || bytes.len() > 1_048_576 {
            return None;
        }
        bytes.extend_from_slice(&chunk[..size]);
        if let Some(index) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            break index + 4;
        }
    };
    let headers = String::from_utf8(bytes[..split].to_vec()).ok()?;
    let len = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    while bytes.len() < split + len {
        let mut chunk = [0; 4096];
        let size = stream.read(&mut chunk).ok()?;
        if size == 0 {
            return None;
        }
        bytes.extend_from_slice(&chunk[..size]);
    }
    Some(Request {
        headers,
        body: bytes[split..split + len].to_vec(),
    })
}
