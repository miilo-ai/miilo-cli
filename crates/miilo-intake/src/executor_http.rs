//! Optional outbound adapter. Enabled only with explicit local configuration.
use crate::execution::{Executor, Report, Request};
use serde::Deserialize;
use std::{
    io::Read,
    path::{Path, PathBuf},
};
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    pub endpoint: String,
    pub token_file: PathBuf,
    pub workflows: Vec<String>,
}
pub struct Http {
    client: reqwest::blocking::Client,
    endpoint: String,
    token: zeroize::Zeroizing<String>,
}
pub fn load(path: &Path) -> Result<(Config, Http), String> {
    let meta =
        std::fs::symlink_metadata(path).map_err(|_| "Cannot inspect executor configuration")?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 16000 {
        return Err("Invalid executor configuration".into());
    }
    let config: Config = serde_json::from_slice(
        &std::fs::read(path).map_err(|_| "Cannot read executor configuration")?,
    )
    .map_err(|_| "Invalid executor configuration")?;
    if config.version != 1
        || config.workflows.is_empty()
        || config.workflows.len() > 32
        || config
            .workflows
            .iter()
            .any(|v| v.trim().is_empty() || v.len() > 200)
        || !config.token_file.is_absolute()
    {
        return Err("Invalid executor bounds".into());
    }
    let mut url = reqwest::Url::parse(&config.endpoint).map_err(|_| "Invalid executor endpoint")?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !(url.scheme() == "https"
            || (url.scheme() == "http"
                && matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))))
    {
        return Err("Executor requires HTTPS or loopback HTTP without URL credentials".into());
    }
    if !url.path().ends_with('/') {
        url.set_path(&format!("{}/", url.path()));
    }
    let token = crate::credential::read(&config.token_file)?;
    let client = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|_| "Cannot construct executor transport")?;
    let endpoint = url
        .join("v1/intake/jobs")
        .map_err(|_| "Invalid executor path")?
        .to_string();
    Ok((
        config,
        Http {
            client,
            endpoint,
            token,
        },
    ))
}
impl Executor for Http {
    fn submit(&self, input: &Request) -> Result<Report, String> {
        let response = self
            .client
            .post(&self.endpoint)
            .bearer_auth(self.token.as_str())
            .header("idempotency-key", &input.job_id)
            .json(input)
            .send()
            .map_err(|_| "Executor outcome uncertain")?;
        if !response.status().is_success() {
            return Err("Executor rejected or could not resolve the job".into());
        }
        let mut bytes = Vec::new();
        response
            .take(16001)
            .read_to_end(&mut bytes)
            .map_err(|_| "Cannot read executor result")?;
        if bytes.len() > 16000 {
            return Err("Executor response exceeded bounds".into());
        }
        let report: Report =
            serde_json::from_slice(&bytes).map_err(|_| "Invalid executor result")?;
        if let Some(link) = &report.pr_url {
            let url = reqwest::Url::parse(link).map_err(|_| "Invalid PR URL")?;
            if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
                return Err("Unsafe PR URL".into());
            }
        }
        Ok(report)
    }
}
