//! Node runtime daemon library for Universal Mobile Compute Fabric.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

pub mod rpc;

pub use rpc::{ModelRegistryError, ModelRegistryService};

