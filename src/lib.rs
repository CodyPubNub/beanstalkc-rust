//! A simple Beanstalkd client.
//!
//! This crate provides a simple and easy-to-use beanstalkd client, which is inspired
//! by [beanstalkc](https://github.com/earl/beanstalkc/) and [rust-beanstalkd](https://github.com/schickling/rust-beanstalkd).
//!
//! # Usage
//!
//! ```toml
//! [dependencies]
//! beanstalkc = "^0.2.0"
//! ```
//!
//! Producer
//!
//! ```no_run
//! #[tokio::main]
//! async fn main() {
//! use std::time::Duration;
//! use beanstalkc::Beanstalkc;
//!
//! let mut conn = Beanstalkc::new()
//!      .connect()
//!      .await
//!      .expect("connect to beanstalkd server failed");
//!
//! conn.use_tube("jobs").await.unwrap();
//! conn.put_default(b"hello, world").await.unwrap();
//! conn.put(b"hello, rust", 1, Duration::from_secs(10), Duration::from_secs(1800)).await.unwrap();
//! }
//! ```
//!
//! Worker
//!
//! ```no_run
//! #[tokio::main]
//! async fn main() {
//! use beanstalkc::Beanstalkc;
//!
//! let mut conn = Beanstalkc::new()
//!      .connect()
//!      .await
//!      .expect("connect to beanstalkd server failed");
//!
//! conn.watch("jobs").await.unwrap();
//!
//! let mut job = conn.reserve().await.unwrap();
//! // execute job here...
//! job.delete().await.unwrap();
//! }
//! ```
pub use crate::beanstalkc::Beanstalkc;
pub use crate::error::{BeanstalkcError, BeanstalkcResult};
pub use crate::job::Job;

mod beanstalkc;
mod command;
mod config;
mod error;
mod job;
mod request;
mod response;
