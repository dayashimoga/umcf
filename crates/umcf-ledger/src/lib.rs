//! `umcf-ledger`
//!
//! Signed Cryptographic Resource Metering Receipts & Double-Entry Compute Credit Ledger
//! for Universal Mobile Compute Fabric.

#![forbid(unsafe_code)]

pub mod error;
pub mod ledger;
pub mod meter;
pub mod receipts;

// Re-exports
pub use error::LedgerError;
pub use ledger::{
    AccountKind, ByzantineFaultType, DoubleEntryCreditLedger, JournalLeg, LedgerTransaction,
    MicroCredits, PricingSchedule, VerificationFailureEvidence, MICRO_CREDITS_PER_CREDIT,
};
pub use meter::{MeteredResourceVector, ResourceMeteringCollector, SanityBoundsChecker};
pub use receipts::{
    CheckpointEmitter, ReceiptBody, ReceiptChainVerifier, SignedResourceReceipt,
    GENESIS_PREV_RECEIPT_ID,
};
