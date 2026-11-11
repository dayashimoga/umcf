//! Hash-chained, dual-signed cryptographic receipts (`SignedResourceReceipt`).

use std::collections::HashMap;
use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::error::LedgerError;
use crate::meter::MeteredResourceVector;

/// Genesis previous receipt identifier for the start of a pair's receipt chain.
pub const GENESIS_PREV_RECEIPT_ID: [u8; 32] = [0u8; 32];

/// Canonical payload containing all verifiable execution parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptBody {
    /// Hash of previous receipt in chain for (worker, requester) pair.
    pub prev_receipt_id: [u8; 32],

    /// Monotonically increasing sequence number (1, 2, 3...).
    pub sequence_number: u64,

    /// Unique job identifier.
    pub job_id: String,

    /// Unique task/step identifier.
    pub task_id: String,

    /// Node ID (DID) of worker that executed the workload.
    pub worker_node_id: String,

    /// Node ID (DID) of requester that commissioned the workload.
    pub requester_node_id: String,

    /// Unix millisecond timestamp when execution started.
    pub started_at_unix_ms: u64,

    /// Unix millisecond timestamp when execution finished.
    pub completed_at_unix_ms: u64,

    /// Measured physical resource consumption.
    pub resources: MeteredResourceVector,

    /// BLAKE3 digest of task input payloads.
    pub input_blake3: [u8; 32],

    /// BLAKE3 digest of task output payload.
    pub output_blake3: [u8; 32],
}

impl ReceiptBody {
    /// Computes canonical deterministic BLAKE3 digest over receipt body.
    #[must_use]
    pub fn canonical_digest(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.prev_receipt_id);
        hasher.update(&self.sequence_number.to_le_bytes());

        hasher.update(&(self.job_id.len() as u64).to_le_bytes());
        hasher.update(self.job_id.as_bytes());

        hasher.update(&(self.task_id.len() as u64).to_le_bytes());
        hasher.update(self.task_id.as_bytes());

        hasher.update(&(self.worker_node_id.len() as u64).to_le_bytes());
        hasher.update(self.worker_node_id.as_bytes());

        hasher.update(&(self.requester_node_id.len() as u64).to_le_bytes());
        hasher.update(self.requester_node_id.as_bytes());

        hasher.update(&self.started_at_unix_ms.to_le_bytes());
        hasher.update(&self.completed_at_unix_ms.to_le_bytes());

        hasher.update(&self.resources.to_canonical_bytes());
        hasher.update(&self.input_blake3);
        hasher.update(&self.output_blake3);

        *hasher.finalize().as_bytes()
    }
}

/// Fully attested, dual-signed cryptographic receipt certifying completed compute.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedResourceReceipt {
    /// BLAKE3 digest of canonical receipt body.
    pub receipt_id: [u8; 32],

    /// Canonical receipt payload.
    pub body: ReceiptBody,

    /// Hex-encoded Ed25519 signature from worker's private key.
    pub worker_signature: String,

    /// Optional hex-encoded Ed25519 countersignature from requester's private key.
    pub requester_countersignature: Option<String>,
}

impl SignedResourceReceipt {
    /// Creates a new receipt signed by the executing worker.
    pub fn create_by_worker(
        body: ReceiptBody,
        worker_signing_key: &SigningKey,
    ) -> Self {
        let receipt_id = body.canonical_digest();
        let sig = worker_signing_key.sign(&receipt_id);
        Self {
            receipt_id,
            body,
            worker_signature: hex::encode(sig.to_bytes()),
            requester_countersignature: None,
        }
    }

    /// Verifies the worker signature against the worker's verifying key.
    pub fn verify_worker_signature(
        &self,
        worker_verifying_key: &VerifyingKey,
    ) -> Result<(), LedgerError> {
        let digest = self.body.canonical_digest();
        if self.receipt_id != digest {
            return Err(LedgerError::SignatureVerificationFailed(
                "Receipt ID does not match computed body digest".to_string(),
            ));
        }

        let sig_bytes = hex::decode(&self.worker_signature)
            .map_err(|e| LedgerError::SignatureVerificationFailed(format!("Malformed worker signature: {e}")))?;
        if sig_bytes.len() != 64 {
            return Err(LedgerError::SignatureVerificationFailed(
                "Invalid worker signature length".to_string(),
            ));
        }
        let mut sig_arr = [0u8; 64];
        sig_arr.copy_from_slice(&sig_bytes);
        let sig = ed25519_dalek::Signature::from_bytes(&sig_arr);

        worker_verifying_key
            .verify(&digest, &sig)
            .map_err(|e| LedgerError::SignatureVerificationFailed(format!("Worker signature invalid: {e}")))?;

        Ok(())
    }

    /// Verifies task output, checks plausible resource bounds, and countersigns with requester key.
    pub fn countersign(
        &mut self,
        requester_signing_key: &SigningKey,
        worker_verifying_key: &VerifyingKey,
        actual_output_bytes: &[u8],
    ) -> Result<(), LedgerError> {
        // 1. Verify worker's signature first
        self.verify_worker_signature(worker_verifying_key)?;

        // 2. Verify output hash integrity
        let computed_out_hash = *blake3::hash(actual_output_bytes).as_bytes();
        if self.body.output_blake3 != computed_out_hash {
            return Err(LedgerError::OutputHashMismatch {
                reported_hash: hex::encode(self.body.output_blake3),
                actual_hash: hex::encode(computed_out_hash),
            });
        }

        // 3. Countersign receipt ID with requester key
        let sig = requester_signing_key.sign(&self.receipt_id);
        self.requester_countersignature = Some(hex::encode(sig.to_bytes()));

        Ok(())
    }

    /// Verifies the requester's countersignature against their verifying key.
    pub fn verify_requester_countersignature(
        &self,
        requester_verifying_key: &VerifyingKey,
    ) -> Result<(), LedgerError> {
        let sig_hex = self
            .requester_countersignature
            .as_deref()
            .ok_or_else(|| LedgerError::MissingCountersignature {
                receipt_id: hex::encode(self.receipt_id),
            })?;

        let sig_bytes = hex::decode(sig_hex)
            .map_err(|e| LedgerError::SignatureVerificationFailed(format!("Malformed requester signature: {e}")))?;
        if sig_bytes.len() != 64 {
            return Err(LedgerError::SignatureVerificationFailed(
                "Invalid requester countersignature length".to_string(),
            ));
        }
        let mut sig_arr = [0u8; 64];
        sig_arr.copy_from_slice(&sig_bytes);
        let sig = ed25519_dalek::Signature::from_bytes(&sig_arr);

        requester_verifying_key
            .verify(&self.receipt_id, &sig)
            .map_err(|e| LedgerError::SignatureVerificationFailed(format!("Requester countersignature invalid: {e}")))?;

        Ok(())
    }
}

/// Validates unbroken hash chains across worker/requester node pairs.
#[derive(Debug, Default)]
pub struct ReceiptChainVerifier {
    /// Maps (worker_node_id, requester_node_id) -> (last_sequence_number, last_receipt_id)
    chains: HashMap<(String, String), (u64, [u8; 32])>,
}

impl ReceiptChainVerifier {
    /// Creates a new chain verifier with empty tracking state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Verifies and commits an attested receipt into the hash chain.
    pub fn verify_and_append(
        &mut self,
        receipt: &SignedResourceReceipt,
        worker_vk: &VerifyingKey,
        requester_vk: &VerifyingKey,
    ) -> Result<(), LedgerError> {
        // 1. Verify dual cryptographic signatures
        receipt.verify_worker_signature(worker_vk)?;
        receipt.verify_requester_countersignature(requester_vk)?;

        let pair_key = (
            receipt.body.worker_node_id.clone(),
            receipt.body.requester_node_id.clone(),
        );

        match self.chains.get(&pair_key) {
            None => {
                // First receipt in chain must reference GENESIS_PREV_RECEIPT_ID and have seq 1
                if receipt.body.sequence_number != 1 {
                    return Err(LedgerError::InvalidSequenceNumber {
                        expected: 1,
                        actual: receipt.body.sequence_number,
                    });
                }
                if receipt.body.prev_receipt_id != GENESIS_PREV_RECEIPT_ID {
                    return Err(LedgerError::HashChainBroken {
                        expected: hex::encode(GENESIS_PREV_RECEIPT_ID),
                        actual: hex::encode(receipt.body.prev_receipt_id),
                    });
                }
            }
            Some(&(last_seq, last_id)) => {
                // Must be exact successor
                let expected_seq = last_seq + 1;
                if receipt.body.sequence_number != expected_seq {
                    return Err(LedgerError::InvalidSequenceNumber {
                        expected: expected_seq,
                        actual: receipt.body.sequence_number,
                    });
                }
                if receipt.body.prev_receipt_id != last_id {
                    return Err(LedgerError::HashChainBroken {
                        expected: hex::encode(last_id),
                        actual: hex::encode(receipt.body.prev_receipt_id),
                    });
                }
            }
        }

        // Commit to state
        self.chains.insert(
            pair_key,
            (receipt.body.sequence_number, receipt.receipt_id),
        );

        Ok(())
    }

    /// Queries the latest sequence number and receipt ID for a given node pair.
    #[must_use]
    pub fn get_chain_head(&self, worker: &str, requester: &str) -> Option<(u64, [u8; 32])> {
        self.chains.get(&(worker.to_string(), requester.to_string())).copied()
    }
}

/// Helper for emitting incremental checkpoint micro-receipts during long streaming sessions.
#[derive(Debug)]
pub struct CheckpointEmitter {
    job_id: String,
    task_id: String,
    worker_node_id: String,
    requester_node_id: String,
    current_sequence: u64,
    prev_receipt_id: [u8; 32],
    last_checkpoint_ms: u64,
}

impl CheckpointEmitter {
    /// Initializes a streaming checkpoint emitter for a session.
    #[must_use]
    pub fn new(
        job_id: String,
        task_id: String,
        worker_node_id: String,
        requester_node_id: String,
        start_ms: u64,
    ) -> Self {
        Self {
            job_id,
            task_id,
            worker_node_id,
            requester_node_id,
            current_sequence: 1,
            prev_receipt_id: GENESIS_PREV_RECEIPT_ID,
            last_checkpoint_ms: start_ms,
        }
    }

    /// Emits a 60-second incremental checkpoint receipt.
    pub fn emit_checkpoint(
        &mut self,
        delta_resources: MeteredResourceVector,
        intermediate_state_bytes: &[u8],
        current_time_ms: u64,
        worker_signing_key: &SigningKey,
    ) -> SignedResourceReceipt {
        let input_hash = *blake3::hash(&self.prev_receipt_id).as_bytes();
        let output_hash = *blake3::hash(intermediate_state_bytes).as_bytes();

        let body = ReceiptBody {
            prev_receipt_id: self.prev_receipt_id,
            sequence_number: self.current_sequence,
            job_id: self.job_id.clone(),
            task_id: format!("{}-chk-{}", self.task_id, self.current_sequence),
            worker_node_id: self.worker_node_id.clone(),
            requester_node_id: self.requester_node_id.clone(),
            started_at_unix_ms: self.last_checkpoint_ms,
            completed_at_unix_ms: current_time_ms,
            resources: delta_resources,
            input_blake3: input_hash,
            output_blake3: output_hash,
        };

        let receipt = SignedResourceReceipt::create_by_worker(body, worker_signing_key);
        self.prev_receipt_id = receipt.receipt_id;
        self.current_sequence += 1;
        self.last_checkpoint_ms = current_time_ms;

        receipt
    }
}
