use miilo_adapter_sdk::compression::{Codec, Limits, codecs};
use miilo_client::{ClientError, MiiloClient};
use std::{
    io::{Read, Write},
    net::TcpListener,
};
#[tokio::test]
async fn automatic_decoding_preserves_json_and_limits_decoded_bytes() {
    for (name, codec) in [
        ("gzip", Codec::Gzip),
        ("br", Codec::Brotli),
        ("zstd", Codec::Zstd),
    ] {
        for oversized in [false, true] {
            let body = if oversized {
                vec![b' '; 3 * 1024 * 1024]
            } else {
                serde_json::json!({"protocolVersion":"0.1","snapshotId":"encoded","observedAt":"2026-10-17T00:00:00Z","cursor":"test:1","providers":[],"workItems":[],"actors":[],"attempts":[],"artifacts":[],"evidence":[],"attention":[]}).to_string().into_bytes()
            };
            let compressed = codecs::compress(codec, &body, Limits::default()).unwrap();
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let server = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(std::time::Duration::from_secs(3)))
                    .unwrap();
                let mut request = Vec::new();
                let mut buffer = [0; 1024];
                while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                    let n = socket.read(&mut buffer).unwrap();
                    if n == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..n]);
                }
                assert!(
                    String::from_utf8_lossy(&request)
                        .to_lowercase()
                        .contains("accept-encoding:")
                );
                write!(socket,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Encoding: {name}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",compressed.len()).unwrap();
                socket.write_all(&compressed).unwrap();
            });
            let result = MiiloClient::new(&format!("http://{address}"), None)
                .unwrap()
                .snapshot()
                .await;
            if oversized {
                assert!(matches!(result, Err(ClientError::ResponseTooLarge)));
            } else {
                assert_eq!(result.unwrap().snapshot_id, "encoded");
            }
            server.join().unwrap();
        }
    }
}
