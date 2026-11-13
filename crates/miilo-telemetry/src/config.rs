use std::{fmt, net::IpAddr};
use url::Url;

/// Explicit host configuration. No OTEL environment discovery or persisted opt-in.
pub struct Config {
    pub(crate) endpoint: Url,
    pub(crate) token: Option<String>,
}

impl Config {
    pub fn new(endpoint: &str, token: Option<String>) -> Result<Self, ConfigError> {
        let endpoint = Url::parse(endpoint).map_err(|_| ConfigError)?;
        let loopback = endpoint
            .host_str()
            .and_then(|host| host.trim_matches(['[', ']']).parse::<IpAddr>().ok())
            .is_some_and(|ip| ip.is_loopback());
        if !(endpoint.scheme() == "https" || endpoint.scheme() == "http" && loopback)
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
            || endpoint.host_str().is_none()
            || token.as_ref().is_some_and(|token| {
                token.is_empty()
                    || token.len() > 4096
                    || !token.bytes().all(|b| (33..=126).contains(&b))
            })
        {
            return Err(ConfigError);
        }
        Ok(Self { endpoint, token })
    }
}

#[derive(Debug)]
pub struct ConfigError;
impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("telemetry requires an HTTPS or numeric-loopback HTTP endpoint without URL credentials, query or fragment, and an optional valid bearer token")
    }
}
impl std::error::Error for ConfigError {}
