#![cfg(unix)]
use miilo_pairing::adapters::files;
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};
#[test]
fn private_storage_rejects_symlinks_public_credentials_and_nonregular_files() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("peer.json");
    files::write(&file, &serde_json::json!({"credential":"test"})).unwrap();
    let value: serde_json::Value = files::read(&file).unwrap();
    assert_eq!(value["credential"], "test");
    assert_eq!(
        fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let link = dir.path().join("link.json");
    symlink(&file, &link).unwrap();
    assert!(files::read::<serde_json::Value>(&link).is_err());
    assert!(files::write(&link, &value).is_err());
    fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(files::read::<serde_json::Value>(&file).is_err());
    assert!(files::read::<serde_json::Value>(dir.path()).is_err());
}
