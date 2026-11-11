//! `umcf-marketplace`
//!
//! Compute Marketplace Order Book, Constraint-Aware Matching Engine, and Lease Contracts
//! for Universal Mobile Compute Fabric.

#![forbid(unsafe_code)]

pub mod error;
pub mod orderbook;

// Re-exports
pub use error::MarketError;
pub use orderbook::{
    ComputeAskOffer, ComputeBidRequest, MarketplaceMatchingEngine, MatchedLeaseContract,
};

