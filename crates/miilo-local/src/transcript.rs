use miilo_adapter_sdk::runtime::observation::Observation;
use serde_json::Value;
use std::{
    fs::OpenOptions,
    io::{Read, Seek, SeekFrom},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::PathBuf,
};

const BOOTSTRAP_BYTES: u64 = 2 * 1024 * 1024;
const MAX_LINE: usize = 4 * 1024 * 1024;
pub struct TranscriptTail {
    path: PathBuf,
    offset: u64,
    identity: Option<(u64, u64)>,
    pending: Vec<u8>,
    discarding: bool,
    apply: fn(&Value, &mut Observation),
}
impl TranscriptTail {
    pub fn new(path: PathBuf, apply: fn(&Value, &mut Observation)) -> Self {
        Self {
            path,
            apply,
            offset: 0,
            identity: None,
            pending: Vec::new(),
            discarding: false,
        }
    }
    pub fn read(&mut self, state: &mut Observation) -> Result<(), String> {
        if self.path.canonicalize().map_err(|e| e.to_string())? != self.path {
            return Err("selected transcript path changed".into());
        }
        let mut file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&self.path)
            .map_err(|_| "Selected transcript unavailable")?;
        let metadata = file
            .metadata()
            .map_err(|_| "Cannot inspect selected transcript")?;
        if !metadata.is_file() {
            return Err("Selected transcript is not a regular file".into());
        }
        let identity = (metadata.dev(), metadata.ino());
        if self.identity.is_some_and(|old| old != identity) {
            return Err("Selected transcript was replaced; reconnect explicitly".into());
        }
        self.identity = Some(identity);
        let len = file.metadata().map_err(|e| e.to_string())?.len();
        if len < self.offset {
            return Err("selected transcript was truncated; reconnect explicitly".into());
        }
        if self.offset == 0 && len > BOOTSTRAP_BYTES {
            self.offset = len - BOOTSTRAP_BYTES;
            self.discarding = true;
        }
        file.seek(SeekFrom::Start(self.offset))
            .map_err(|e| e.to_string())?;
        let mut chunk = [0u8; 65536];
        let mut remaining = BOOTSTRAP_BYTES;
        while remaining > 0 {
            let limit = chunk.len().min(remaining as usize);
            let n = file.read(&mut chunk[..limit]).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            remaining -= n as u64;
            self.offset += n as u64;
            self.consume(&chunk[..n], state);
        }
        Ok(())
    }
    pub fn consume(&mut self, bytes: &[u8], state: &mut Observation) {
        for &byte in bytes {
            if byte == b'\n' {
                if !self.discarding {
                    if let Ok(row) = serde_json::from_slice::<Value>(&self.pending) {
                        (self.apply)(&row, state);
                    }
                }
                self.pending.clear();
                self.discarding = false;
            } else if !self.discarding {
                self.pending.push(byte);
                if self.pending.len() > MAX_LINE {
                    self.pending.clear();
                    self.discarding = true;
                }
            }
        }
    }
}
