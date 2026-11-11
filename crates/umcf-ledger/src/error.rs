//! Exhaustive structured error types for `umcf-ledger`.

use thiserror::Error;

/// Structured ledger, metering, and receipt verification errors.
#[derive(Debug, Error)]
pub enum LedgerError {
    /// Cryptographic signature verification failed.
    #[error("Cryptographic signature verification failed: {0}")]
    SignatureVerificationFailed(String),

    /// Receipt hash-chain link is invalid or broken.
    #[error("Receipt hash chain broken: expected prev_receipt_id {expected}, found {actual}")]
    HashChainBroken { expected: String, actual: String },

    /// Sequence number does not match expected monotonic successor.
    #[error("Sequence number anomaly: expected sequence {expected}, found {actual}")]
    InvalidSequenceNumber { expected: u64, actual: u64 },

    /// A worker attempted to claim an implausible or physically impossible resource amount.
    #[error("Inflated resource claim rejected by sanity bounds: {dimension} claimed {claimed}, physical ceiling is {max_plausible} ({reason})")]
    InflatedResourceClaim {
        dimension: String,
        claimed: u64,
        max_plausible: u64,
        reason: String,
    },

    /// The computed output hash does not match the receipt's claimed output digest.
    #[error("Output hash mismatch: reported {reported_hash}, verified {actual_hash}")]
    OutputHashMismatch {
        reported_hash: String,
        actual_hash: String,
    },

    /// Finalized receipt requires requester countersignature.
    #[error("Missing requester countersignature on finalized receipt {receipt_id}")]
    MissingCountersignature { receipt_id: String },

    /// Duplicate receipt ID or replayed sequence in chain.
    #[error("Receipt chain for pair ({worker}, {requester}) already contains receipt {receipt_id} (fork or replay attempt)")]
    DuplicateReceipt {
        worker: String,
        requester: String,
        receipt_id: String,
    },

    /// Serialization error.
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Internal storage or ledger database error.
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Double-entry journal legs do not balance (sum of debits != sum of credits).
    #[error(
        "Unbalanced ledger transaction: sum of debits ({debits}) != sum of credits ({credits})"
    )]
    UnbalancedTransaction { debits: u128, credits: u128 },

    /// An account does not have sufficient balance to perform debit.
    #[error("Insufficient funds in account '{account}': available {available} micro-credits, required {required}")]
    InsufficientBalance {
        account: String,
        available: i128,
        required: u128,
    },

    /// Receipt has already been settled in the ledger (double-spend / replay prevention).
    #[error("Receipt {receipt_id} has already been settled into ledger")]
    ReceiptAlreadySettled { receipt_id: String },

    /// The specified escrow account was not found.
    #[error("Escrow not found for job '{job_id}'")]
    EscrowNotFound { job_id: String },

    /// Escrow balance is depleted or insufficient to cover settlement.
    #[error("Escrow for job '{job_id}' depleted: remaining {remaining} micro-credits, requested payout {requested}")]
    EscrowDepleted {
        job_id: String,
        remaining: u128,
        requested: u128,
    },

    /// Byzantine evidence verification failed or is malformed.
    #[error("Byzantine slashing evidence rejected: {reason}")]
    ByzantineSlashingEvidenceInvalid { reason: String },

    /// Generic ledger or marketplace operation error.
    #[error("Ledger operation error: {0}")]
    OperationFailed(String),
}
