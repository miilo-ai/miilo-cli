use crate::{
    Result,
    adapters::{files, system::Random},
    application::Entropy,
    domain::{hash, validate_name},
};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Arc};
use tokio_rustls::rustls;

#[derive(Serialize, Deserialize)]
pub struct Identity {
    pub id: String,
    pub name: String,
    pub url: String,
    pub certificate: String,
    pub key_der: Vec<u8>,
    pub certificate_der: Vec<u8>,
}
pub fn endpoint(value: &str) -> Result<url::Url> {
    let u = url::Url::parse(value).map_err(|_| "Invalid host URL")?;
    if u.scheme() != "https"
        || u.host_str().is_none()
        || !u.username().is_empty()
        || u.password().is_some()
        || u.query().is_some()
        || u.fragment().is_some()
        || u.path() != "/"
    {
        return Err("Advertise an HTTPS origin without credentials or a path".into());
    }
    Ok(u)
}
impl Identity {
    pub fn load(directory: &Path, name: &str, advertised: &str) -> Result<Self> {
        validate_name(name)?;
        let u = endpoint(advertised)?;
        files::prepare(directory)?;
        let file = directory.join("identity.json");
        if file.try_exists().map_err(|_| "Cannot inspect identity")? {
            let identity: Self = files::read(&file)?;
            if identity.url != u.as_str().trim_end_matches('/') {
                return Err(
                    "Host URL changed; use a separate pairing directory and pair again".into(),
                );
            }
            return Ok(identity);
        }
        let generated = rcgen::generate_simple_self_signed(vec![
            u.host_str().unwrap().trim_matches(['[', ']']).into(),
        ])
        .map_err(|_| "Cannot create host certificate")?;
        let identity = Self {
            id: Random.id(),
            name: name.into(),
            url: u.as_str().trim_end_matches('/').into(),
            certificate: generated.cert.pem(),
            certificate_der: generated.cert.der().to_vec(),
            key_der: generated.signing_key.serialize_der(),
        };
        files::write(&file, &identity)?;
        Ok(identity)
    }
    pub fn fingerprint(&self) -> String {
        hash(&self.certificate_der)
    }
    pub fn tls(&self) -> Result<Arc<rustls::ServerConfig>> {
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let config = rustls::ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|_| "Cannot configure TLS")?
            .with_no_client_auth()
            .with_single_cert(
                vec![rustls::pki_types::CertificateDer::from(
                    self.certificate_der.clone(),
                )],
                rustls::pki_types::PrivatePkcs8KeyDer::from(self.key_der.clone()).into(),
            )
            .map_err(|_| "Invalid host certificate")?;
        Ok(Arc::new(config))
    }
}
