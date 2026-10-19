//! Optional device pairing. No listener is started by local observation.
pub mod adapters;
pub mod application;
pub mod domain;
pub type Result<T> = std::result::Result<T, String>;
