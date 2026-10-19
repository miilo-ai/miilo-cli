use miilo_pairing::{
    application::{Clock, Entropy, Host, Store},
    domain::{self, Ledger},
};
use std::cell::{Cell, RefCell};
struct Time(Cell<u64>);
impl Clock for Time {
    fn now(&self) -> u64 {
        self.0.get()
    }
}
struct Random(Cell<u64>);
impl Entropy for Random {
    fn secret(&self) -> String {
        let n = self.0.get() + 1;
        self.0.set(n);
        format!("{n:064x}")
    }
    fn id(&self) -> String {
        self.secret()[32..].into()
    }
    fn code(&self) -> String {
        "483215".into()
    }
}
#[derive(Default)]
struct Memory {
    fail: Cell<bool>,
    saved: RefCell<Ledger>,
}
impl Store for Memory {
    fn save(&self, ledger: &Ledger) -> miilo_pairing::Result<()> {
        if self.fail.get() {
            return Err("Storage failed".into());
        }
        *self.saved.borrow_mut() = ledger.clone();
        Ok(())
    }
}
fn host() -> Host<Memory, Time, Random> {
    Host::new(
        Ledger::default(),
        Memory::default(),
        Time(Cell::new(100)),
        Random(Cell::new(0)),
    )
}
#[test]
fn invitation_is_single_use_expiring_and_does_not_authorize_a_reader() {
    let mut h = host();
    let (secret, _) = h.invite();
    assert!(!h.authorized(&secret));
    assert!(h.begin("wrong", "Phone").is_err());
    let (poll, _, _) = h.begin(&secret, "Phone").unwrap();
    assert!(h.begin(&secret, "Another").is_err());
    assert!(!h.authorized(&poll));
    assert!(h.poll(&poll).unwrap().token.is_none());
    let (secret, expiry) = h.invite();
    h.clock.0.set(expiry);
    assert!(h.begin(&secret, "Phone").is_err());
}
#[test]
fn approval_requires_code_and_revocation_cancels_reads_and_polling() {
    let mut h = host();
    let (secret, _) = h.invite();
    let (poll, code, _) = h.begin(&secret, "Phone").unwrap();
    let id = h.pending[0].id.clone();
    assert!(h.decide(&id, "000000", true).is_err());
    h.decide(&id, &code, true).unwrap();
    let token = h.poll(&poll).unwrap().token.clone().unwrap();
    assert!(h.authorized(&token));
    assert!(h.decide(&id, &code, true).is_err());
    assert!(h.poll("wrong").is_err());
    h.store.fail.set(true);
    assert!(h.revoke(&id).is_err());
    assert!(h.authorized(&token));
    h.store.fail.set(false);
    h.revoke(&id).unwrap();
    assert!(!h.authorized(&token));
    assert!(h.poll(&poll).is_err());
}
#[test]
fn denial_expiry_and_persistence_failures_do_not_grant_access() {
    let mut h = host();
    let (secret, _) = h.invite();
    let (poll, code, expiry) = h.begin(&secret, "Phone").unwrap();
    let id = h.pending[0].id.clone();
    h.store.fail.set(true);
    assert!(h.decide(&id, &code, true).is_err());
    assert!(h.ledger.devices.is_empty());
    h.store.fail.set(false);
    h.decide(&id, "", false).unwrap();
    assert!(h.poll(&poll).unwrap().denied);
    assert!(h.decide(&id, &code, true).is_err());
    h.clock.0.set(expiry);
    assert!(h.poll(&poll).is_err());
}
#[test]
fn device_credentials_survive_restart_but_expire_and_are_isolated() {
    let mut h = host();
    let (secret, _) = h.invite();
    let (poll, code, _) = h.begin(&secret, "One").unwrap();
    h.decide(&h.pending[0].id.clone(), &code, true).unwrap();
    let first = h.poll(&poll).unwrap().token.clone().unwrap();
    let (secret, _) = h.invite();
    let (poll, code, _) = h.begin(&secret, "Two").unwrap();
    let id = h.pending[1].id.clone();
    h.decide(&id, &code, true).unwrap();
    let second = h.poll(&poll).unwrap().token.clone().unwrap();
    h.revoke(&id).unwrap();
    assert!(h.authorized(&first));
    assert!(!h.authorized(&second));
    let restarted = Host::new(
        h.store.saved.borrow().clone(),
        Memory::default(),
        Time(Cell::new(101)),
        Random(Cell::new(50)),
    );
    assert!(restarted.authorized(&first));
    restarted.clock.0.set(100 + domain::DEVICE_SECONDS);
    assert!(!restarted.authorized(&first));
}
#[test]
fn request_limits_and_unsafe_names_are_rejected() {
    let mut h = host();
    assert!(domain::validate_name("\u{1b}[31m").is_err());
    assert!(domain::validate_id("../../credentials").is_err());
    for _ in 0..8 {
        let (secret, _) = h.invite();
        h.begin(&secret, "Device").unwrap();
    }
    let (secret, _) = h.invite();
    assert!(h.begin(&secret, "Device").is_err());
    h.clock.0.set(221);
    assert!(h.begin(&secret, "Device").is_ok());
}
