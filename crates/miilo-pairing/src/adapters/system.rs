use crate::application::{Clock, Entropy};
use rand::Rng;
pub struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}
pub struct Random;
impl Entropy for Random {
    fn secret(&self) -> String {
        rand::rng()
            .random::<[u8; 32]>()
            .iter()
            .map(|v| format!("{v:02x}"))
            .collect()
    }
    fn id(&self) -> String {
        rand::rng()
            .random::<[u8; 16]>()
            .iter()
            .map(|v| format!("{v:02x}"))
            .collect()
    }
    fn code(&self) -> String {
        format!("{:06}", rand::rng().random_range(0..1_000_000))
    }
}
