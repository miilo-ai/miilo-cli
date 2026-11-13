use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
pub(crate) struct Counters {
    pub submitted: AtomicU64,
    pub delivered: AtomicU64,
    pub failed_batches: AtomicU64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeliveryStats {
    pub submitted: u64,
    pub delivered: u64,
    pub failed_batches: u64,
}
impl Counters {
    pub fn snapshot(&self) -> DeliveryStats {
        DeliveryStats {
            submitted: self.submitted.load(Ordering::Relaxed),
            delivered: self.delivered.load(Ordering::Relaxed),
            failed_batches: self.failed_batches.load(Ordering::Relaxed),
        }
    }
}
