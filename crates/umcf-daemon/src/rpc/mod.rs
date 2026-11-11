//! RPC handlers for the `umcfd` daemon.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

pub mod model_registry;

pub use model_registry::{ModelRegistryError, ModelRegistryService};
