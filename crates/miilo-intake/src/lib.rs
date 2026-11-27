//! Local-first reference intake. Domain/application logic has no provider dependency.
pub mod application;
pub mod domain;
pub mod projection;
pub mod storage;

pub mod server;

pub mod execution;
pub mod executor_http;

mod credential;

pub mod ports;

pub mod service;
