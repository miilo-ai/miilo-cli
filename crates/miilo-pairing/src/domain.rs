use crate::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

pub const INVITE_SECONDS: u64 = 300;
pub const REQUEST_SECONDS: u64 = 120;
pub const DEVICE_SECONDS: u64 = 30 * 86400;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Invitation {
    pub version: u32,
    pub host_id: String,
    pub name: String,
    pub url: String,
    pub certificate_pem: String,
    pub fingerprint: String,
    pub secret: String,
    pub expires_at: u64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PairRequest {
    pub secret: String,
    pub name: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Pending {
    pub id: String,
    pub code: String,
    pub name: String,
    pub expires_at: u64,
    pub poll_hash: String,
    pub token: Option<String>,
    pub denied: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    pub name: String,
    pub token_hash: String,
    pub expires_at: u64,
}
#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ledger {
    pub devices: Vec<Device>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PairedHost {
    pub host_id: String,
    pub device_id: String,
    pub name: String,
    pub url: String,
    pub certificate_pem: String,
    pub fingerprint: String,
    pub token: String,
    pub expires_at: u64,
}
pub fn hash(value: &[u8]) -> String {
    Sha256::digest(value)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
pub fn matches(secret: &str, digest: &str) -> bool {
    hash(secret.as_bytes())
        .as_bytes()
        .ct_eq(digest.as_bytes())
        .into()
}
pub fn validate_name(value: &str) -> Result<()> {
    if value.trim().is_empty() || value.chars().count() > 64 || value.chars().any(char::is_control)
    {
        return Err("Use a device name of 1–64 printable characters".into());
    }
    Ok(())
}
pub fn validate_id(value: &str) -> Result<()> {
    if value.len() != 32 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Invalid device identity".into());
    }
    Ok(())
}
