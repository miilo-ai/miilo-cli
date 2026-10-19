use crate::{
    Result,
    domain::{Invitation, PairRequest, PairedHost, hash, validate_id, validate_name},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::blocking::Client;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::io::Read;
use std::time::Duration;

pub fn encode(invitation: &Invitation) -> Result<String> {
    Ok(format!(
        "miilo-pair:{}",
        URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(invitation).map_err(|_| "Cannot encode invitation")?)
    ))
}
pub fn decode(value: &str, now: u64) -> Result<Invitation> {
    if value.len() > 16_384 {
        return Err("Invitation too large".into());
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(
            value
                .trim()
                .strip_prefix("miilo-pair:")
                .ok_or("Use a complete Miilo pairing invitation")?,
        )
        .map_err(|_| "Invalid invitation")?;
    let i: Invitation = serde_json::from_slice(&bytes).map_err(|_| "Invalid invitation")?;
    validate_id(&i.host_id)?;
    validate_name(&i.name)?;
    let u = url::Url::parse(&i.url).map_err(|_| "Invalid invitation URL")?;
    if i.expires_at <= now {
        return Err("Invitation expired; create another on the host".into());
    }
    if i.expires_at > now.saturating_add(330) {
        return Err("Device clock differs from host; synchronize both clocks".into());
    }
    if i.version != 1
        || i.secret.len() != 64
        || !i.secret.bytes().all(|b| b.is_ascii_hexdigit())
        || u.scheme() != "https"
        || u.host_str().is_none()
        || !u.username().is_empty()
        || u.password().is_some()
        || u.path() != "/"
        || u.query().is_some()
        || u.fragment().is_some()
    {
        return Err("Invalid or expired invitation".into());
    }
    pinned(&i.certificate_pem, &i.fingerprint)?;
    Ok(i)
}
pub fn pinned(pem: &str, fingerprint: &str) -> Result<Client> {
    let cert =
        reqwest::Certificate::from_pem(pem.as_bytes()).map_err(|_| "Invalid host certificate")?;
    let der = cert_to_der(pem)?;
    if fingerprint.len() != 64 || hash(&der) != fingerprint {
        return Err("Host certificate fingerprint mismatch".into());
    }
    Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .tls_built_in_root_certs(false)
        .add_root_certificate(cert)
        .timeout(Duration::from_secs(8))
        .connect_timeout(Duration::from_secs(4))
        .build()
        .map_err(|_| "Cannot initialize paired connection".into())
}
fn cert_to_der(pem: &str) -> Result<Vec<u8>> {
    let body = pem
        .strip_prefix("-----BEGIN CERTIFICATE-----")
        .and_then(|p| p.trim().strip_suffix("-----END CERTIFICATE-----"))
        .ok_or("Invalid certificate PEM")?;
    base64::engine::general_purpose::STANDARD
        .decode(body.split_whitespace().collect::<String>())
        .map_err(|_| "Invalid certificate encoding".into())
}
pub fn request<T: DeserializeOwned>(
    client: &Client,
    url: &str,
    token: &str,
    body: Option<&impl Serialize>,
) -> Result<T> {
    let mut req = if let Some(body) = body {
        client.post(url).json(body)
    } else {
        client.get(url)
    };
    if !token.is_empty() {
        req = req.bearer_auth(token);
    }
    let response = req
        .send()
        .map_err(|_| "Paired host unavailable or certificate changed")?;
    if !response.status().is_success() {
        return Err(format!(
            "Paired host returned HTTP {}",
            response.status().as_u16()
        ));
    }
    let mut bytes = vec![];
    response
        .take(2_000_001)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read paired response")?;
    if bytes.len() > 2_000_000 {
        return Err("Paired response too large".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "Invalid paired response".into())
}
pub fn connect(
    invitation: Invitation,
    name: &str,
    mut show_code: impl FnMut(&str) -> Result<()>,
) -> Result<PairedHost> {
    validate_name(name)?;
    let client = pinned(&invitation.certificate_pem, &invitation.fingerprint)?;
    let pending: Value = request(
        &client,
        &format!("{}/pair/request", invitation.url),
        "",
        Some(&PairRequest {
            secret: invitation.secret.clone(),
            name: name.into(),
        }),
    )?;
    let poll = pending["poll"]
        .as_str()
        .filter(|p| p.len() == 64 && p.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or("Invalid pairing response")?;
    let code = pending["code"]
        .as_str()
        .filter(|p| p.len() == 6 && p.bytes().all(|b| b.is_ascii_digit()))
        .ok_or("Invalid comparison code")?;
    show_code(code)?;
    let deadline = std::time::Instant::now() + Duration::from_secs(120);
    while std::time::Instant::now() < deadline {
        let reply: Value = request(
            &client,
            &format!("{}/pair/poll", invitation.url),
            "",
            Some(&json!({"poll":poll})),
        )?;
        match reply["status"].as_str() {
            Some("denied") => return Err("Host rejected pairing".into()),
            Some("approved") => {
                let token = reply["token"]
                    .as_str()
                    .filter(|p| p.len() == 64 && p.bytes().all(|b| b.is_ascii_hexdigit()))
                    .ok_or("Invalid paired credential")?;
                let id = reply["deviceId"]
                    .as_str()
                    .ok_or("Missing paired identity")?;
                validate_id(id)?;
                let expiry = reply["expiresAt"]
                    .as_u64()
                    .ok_or("Missing credential expiry")?;
                return Ok(PairedHost {
                    host_id: invitation.host_id,
                    device_id: id.into(),
                    name: invitation.name,
                    url: invitation.url,
                    certificate_pem: invitation.certificate_pem,
                    fingerprint: invitation.fingerprint,
                    token: token.into(),
                    expires_at: expiry,
                });
            }
            Some("pending") => std::thread::sleep(Duration::from_secs(1)),
            _ => return Err("Invalid pairing status".into()),
        }
    }
    Err("Pairing timed out; check the host before requesting another invitation".into())
}
pub fn snapshot(peer: &PairedHost) -> Result<Value> {
    request(
        &pinned(&peer.certificate_pem, &peer.fingerprint)?,
        &format!("{}/v1/snapshot", peer.url),
        &peer.token,
        None::<&Value>,
    )
}
