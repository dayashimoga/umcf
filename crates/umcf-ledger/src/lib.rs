//! `umcf-ledger`
//!
//! Signed Cryptographic Resource Metering Receipts & Double-Entry Compute Credit Ledger
//! for Universal Mobile Compute Fabric.

#![forbid(unsafe_code)]

pub mod error;
pub mod meter;
pub mod receipts;

// Re-exports
pub use error::LedgerError;
pub use meter::{MeteredResourceVector, ResourceMeteringCollector, SanityBoundsChecker};
pub use receipts::{
    CheckpointEmitter, ReceiptBody, ReceiptChainVerifier, SignedResourceReceipt,
    GENESIS_PREV_RECEIPT_ID,
};
