//! Exhaustive structured error types for `umcf-marketplace`.

use thiserror::Error;
use umcf_ledger::LedgerError;

/// Structured errors in compute marketplace order matching and lease management.
#[derive(Debug, Error)]
pub enum MarketError {
    /// No available provider offer satisfied all bid constraints.
    #[error("No matching compute offer found for bid '{bid_id}'")]
    NoMatchingOffer { bid_id: String },

    /// The specified offer ID was not found.
    #[error("Compute offer '{offer_id}' not found in order book")]
    OfferNotFound { offer_id: String },

    /// The specified bid ID was not found.
    #[error("Compute bid '{bid_id}' not found")]
    BidNotFound { bid_id: String },

    /// Provider is inactive or paused.
    #[error("Provider '{offer_id}' is inactive: {reason}")]
    ProviderInactive { offer_id: String, reason: String },

    /// Mobile battery, charging, or network safety constraints were violated.
    #[error("Safety constraint violated for offer '{offer_id}': {constraint}")]
    SafetyConstraintViolated {
        offer_id: String,
        constraint: String,
    },

    /// Invalid cryptographic signature on lease contract.
    #[error("Invalid lease contract signature: {0}")]
    InvalidContractSignature(String),

    /// Bid budget is insufficient to cover provider's minimum rates.
    #[error("Bid budget {budget} micro-credits insufficient; minimum required is {required}")]
    BudgetExceeded { budget: u128, required: u128 },

    /// Underlying ledger error.
    #[error("Underlying ledger error: {0}")]
    Ledger(#[from] LedgerError),
}
