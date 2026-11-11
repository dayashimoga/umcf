// lib.rs
// UMCF Rust Client SDK (`umcf-sdk`)
// Idiomatic strongly-typed client for the Universal Mobile Compute Fabric API.

pub mod client;
pub mod error;
pub mod types;

pub use client::{UmcfClient, UmcfClientBuilder};
pub use error::SdkError;
pub use types::*;

