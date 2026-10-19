use crate::{
    Result,
    domain::{self, Device, Ledger, Pending},
};

pub trait Store {
    fn save(&self, ledger: &Ledger) -> Result<()>;
}
pub trait Clock {
    fn now(&self) -> u64;
}
pub trait Entropy {
    fn secret(&self) -> String;
    fn id(&self) -> String;
    fn code(&self) -> String;
}
pub trait Snapshot {
    fn read(&self) -> Result<serde_json::Value>;
}

pub struct Host<S, C, E> {
    pub ledger: Ledger,
    pub pending: Vec<Pending>,
    invitation: Option<(String, u64)>,
    pub store: S,
    pub clock: C,
    pub entropy: E,
}
impl<S: Store, C: Clock, E: Entropy> Host<S, C, E> {
    pub fn new(ledger: Ledger, store: S, clock: C, entropy: E) -> Self {
        Self {
            ledger,
            pending: vec![],
            invitation: None,
            store,
            clock,
            entropy,
        }
    }
    pub fn invite(&mut self) -> (String, u64) {
        let secret = self.entropy.secret();
        let expiry = self.clock.now() + domain::INVITE_SECONDS;
        self.invitation = Some((domain::hash(secret.as_bytes()), expiry));
        (secret, expiry)
    }
    pub fn begin(&mut self, secret: &str, name: &str) -> Result<(String, String, u64)> {
        domain::validate_name(name)?;
        let now = self.clock.now();
        self.pending.retain(|p| p.expires_at > now);
        if self.pending.len() >= 8 {
            return Err("Too many pending requests".into());
        }
        if !self
            .invitation
            .as_ref()
            .is_some_and(|(h, expiry)| *expiry > now && domain::matches(secret, h))
        {
            return Err("Invitation expired or already used".into());
        }
        let poll = self.entropy.secret();
        let code = self.entropy.code();
        self.pending.push(Pending {
            id: self.entropy.id(),
            code: code.clone(),
            name: name.into(),
            expires_at: now + domain::REQUEST_SECONDS,
            poll_hash: domain::hash(poll.as_bytes()),
            token: None,
            denied: false,
        });
        self.invitation = None;
        Ok((poll, code, now + domain::REQUEST_SECONDS))
    }
    pub fn decide(&mut self, id: &str, code: &str, approve: bool) -> Result<()> {
        let now = self.clock.now();
        let p = self
            .pending
            .iter_mut()
            .find(|p| p.id == id && p.expires_at > now && p.token.is_none() && !p.denied)
            .ok_or("Request expired or already decided")?;
        if !approve {
            p.denied = true;
            return Ok(());
        }
        if code != p.code {
            return Err("Comparison code does not match".into());
        }
        let mut next = self.ledger.clone();
        next.devices.retain(|d| d.expires_at > now);
        if next.devices.len() >= 64 {
            return Err("Revoke an old device first".into());
        }
        let token = self.entropy.secret();
        next.devices.push(Device {
            id: p.id.clone(),
            name: p.name.clone(),
            token_hash: domain::hash(token.as_bytes()),
            expires_at: now + domain::DEVICE_SECONDS,
        });
        self.store.save(&next)?;
        self.ledger = next;
        p.token = Some(token);
        Ok(())
    }
    pub fn poll(&self, poll: &str) -> Result<&Pending> {
        self.pending
            .iter()
            .find(|p| p.expires_at > self.clock.now() && domain::matches(poll, &p.poll_hash))
            .ok_or_else(|| "Pairing request expired".into())
    }
    pub fn authorized(&self, token: &str) -> bool {
        self.ledger
            .devices
            .iter()
            .any(|d| d.expires_at > self.clock.now() && domain::matches(token, &d.token_hash))
    }
    pub fn revoke(&mut self, id: &str) -> Result<()> {
        domain::validate_id(id)?;
        let mut next = self.ledger.clone();
        next.devices.retain(|d| d.id != id);
        self.store.save(&next)?;
        self.ledger = next;
        self.pending.retain(|p| p.id != id);
        Ok(())
    }
}
