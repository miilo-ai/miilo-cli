#![cfg(all(unix, feature = "server"))]
use miilo_pairing::{
    adapters::{client, control, files, identity::Identity, server},
    application::Snapshot,
};
use serde_json::{Value, json};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

struct Source(Arc<AtomicBool>);
impl Snapshot for Source {
    fn read(&self) -> miilo_pairing::Result<Value> {
        if self.0.load(Ordering::SeqCst) {
            Ok(json!({"snapshotId":"fresh","workItems":[]}))
        } else {
            Err("Stale".into())
        }
    }
}
#[test]
fn real_tls_pairing_approval_snapshot_staleness_and_revocation() {
    let temp = tempfile::tempdir().unwrap();
    let socket = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = socket.local_addr().unwrap();
    drop(socket);
    let directory = temp.path().to_path_buf();
    let url = format!("https://{address}");
    let readable = Arc::new(AtomicBool::new(true));
    let source = readable.clone();
    let host_dir = directory.clone();
    let host_url = url.clone();
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let host = std::thread::spawn(move || {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            tokio::select! {
                result = server::run(server::Options { bind: address, advertised: host_url, name:"Test host".into(), directory:host_dir }, Arc::new(Source(source))) => result.unwrap(),
                _ = stopped => {}
            }
        });
    });
    let rt = tokio::runtime::Runtime::new().unwrap();
    for _ in 0..100 {
        if rt
            .block_on(control::request(&directory, json!({"command":"devices"})))
            .is_ok()
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let invite = rt
        .block_on(control::request(&directory, json!({"command":"invite"})))
        .unwrap();
    use miilo_pairing::application::Clock;
    let i = client::decode(
        invite["invitation"].as_str().unwrap(),
        miilo_pairing::adapters::system::SystemClock.now(),
    )
    .unwrap();
    let http = client::pinned(&i.certificate_pem, &i.fingerprint).unwrap();
    assert_eq!(
        http.get(format!("{url}/v1/snapshot"))
            .send()
            .unwrap()
            .status(),
        401
    );
    assert!(client::pinned(&i.certificate_pem, &"0".repeat(64)).is_err());
    let other = tempfile::tempdir().unwrap();
    let wrong = Identity::load(other.path(), "Other host", &url).unwrap();
    let wrong_client = client::pinned(&wrong.certificate, &wrong.fingerprint()).unwrap();
    assert!(wrong_client.get(format!("{url}/health")).send().is_err());
    assert_eq!(
        http.post(format!("{url}/pair/request"))
            .header("content-type", "application/json")
            .body("x".repeat(9000))
            .send()
            .unwrap()
            .status(),
        413
    );
    let (codes, received) = std::sync::mpsc::channel();
    let connecting = std::thread::spawn(move || {
        client::connect(i, "Test phone", |code| {
            codes.send(code.to_owned()).unwrap();
            Ok(())
        })
    });
    let code = received.recv_timeout(Duration::from_secs(5)).unwrap();
    let requests = rt
        .block_on(control::request(&directory, json!({"command":"requests"})))
        .unwrap();
    assert_eq!(requests.as_array().unwrap().len(), 1);
    assert!(
        rt.block_on(control::request(
            &directory,
            json!({"command":"approve","id":requests[0]["id"],"code":"incorrect"})
        ))
        .is_err()
    );
    rt.block_on(control::request(
        &directory,
        json!({"command":"approve","id":requests[0]["id"],"code":code}),
    ))
    .unwrap();
    let peer = connecting.join().unwrap().unwrap();
    assert_eq!(client::snapshot(&peer).unwrap()["snapshotId"], "fresh");
    assert_eq!(
        http.post(format!("{url}/v1/actions"))
            .bearer_auth(&peer.token)
            .send()
            .unwrap()
            .status(),
        404
    );
    assert_eq!(
        http.post(format!("{url}/pair/approve"))
            .send()
            .unwrap()
            .status(),
        404
    );
    readable.store(false, Ordering::SeqCst);
    assert!(client::snapshot(&peer).unwrap_err().contains("503"));
    readable.store(true, Ordering::SeqCst);
    assert!(client::snapshot(&peer).is_ok());
    rt.block_on(control::request(
        &directory,
        json!({"command":"revoke","id":peer.device_id}),
    ))
    .unwrap();
    assert!(client::snapshot(&peer).unwrap_err().contains("401"));
    let devices: Value = rt
        .block_on(control::request(&directory, json!({"command":"devices"})))
        .unwrap();
    assert!(devices.as_array().unwrap().is_empty());
    stop.send(()).unwrap();
    host.join().unwrap();
    let identity: Identity = files::read(&directory.join("identity.json")).unwrap();
    assert_eq!(identity.url, url);
}

#[test]
fn malformed_invites_and_identity_changes_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let identity = Identity::load(temp.path(), "Test", "https://127.0.0.1:9971").unwrap();
    let i = miilo_pairing::domain::Invitation {
        version: 1,
        host_id: identity.id.clone(),
        name: "Test".into(),
        url: identity.url.clone(),
        certificate_pem: identity.certificate.clone(),
        fingerprint: identity.fingerprint(),
        secret: "a".repeat(64),
        expires_at: 200,
    };
    let text = client::encode(&i).unwrap();
    assert!(client::decode(&text, 100).is_ok());
    let mut skew = i.clone();
    skew.expires_at = 405;
    assert!(client::decode(&client::encode(&skew).unwrap(), 100).is_ok());
    skew.expires_at = 431;
    assert!(client::decode(&client::encode(&skew).unwrap(), 100).is_err());
    assert!(client::decode(&text, 200).is_err());
    assert!(client::decode(&"x".repeat(20_000), 100).is_err());
    assert!(Identity::load(temp.path(), "Test", "https://127.0.0.1:9972").is_err());
    let mut bad = i;
    bad.url = "https://user:password@example.com/".into();
    assert!(client::decode(&client::encode(&bad).unwrap(), 100).is_err());
}
