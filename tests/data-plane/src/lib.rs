//! Data-plane zero-copy and mobile power optimization verification library.
//! Adheres strictly to `#![forbid(unsafe_code)]` and zero-placeholder invariants.

pub use umcf_core::buffer_pool::*;
pub use umcf_hwprobe::power_governor::*;
pub use umcf_net::zero_copy::*;

