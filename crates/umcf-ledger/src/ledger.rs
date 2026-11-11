//! ACID double-entry compute credit ledger, escrow management, and slashing penalties.

use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::error::LedgerError;
use crate::meter::MeteredResourceVector;
use crate::receipts::SignedResourceReceipt;

/// Monetary denomination represented strictly as an integer in micro-credits.
///
/// 1_000_000 micro-credits = 1.000000 UMCF Credit.
/// Floating-point numbers are strictly forbidden in accounting logic.
pub type MicroCredits = i128;

/// Conversion factor: 1 UMCF Credit = 1,000,000 micro-credits.
pub const MICRO_CREDITS_PER_CREDIT: i128 = 1_000_000;

/// Standard typed account classifications in the UMCF ledger.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccountKind {
    /// Readily spendable credit balance owned by a user or node.
    AvailableBalance(String),
    /// Pre-allocated escrow locked for a specific job execution.
    EscrowLocked(String),
    /// Collateral deposit bonded by a worker node to participate in marketplace jobs.
    ProviderStakeBond(String),
    /// Destination pool for slashed collateral from Byzantine behavior.
    SlashedPenaltyPool,
    /// Originating treasury source account for initial credit minting.
    TreasuryMint,
}

impl AccountKind {
    /// Human-readable account identifier string.
    pub fn to_account_string(&self) -> String {
        match self {
            AccountKind::AvailableBalance(id) => format!("available:{id}"),
            AccountKind::EscrowLocked(job) => format!("escrow:{job}"),
            AccountKind::ProviderStakeBond(worker) => format!("stake:{worker}"),
            AccountKind::SlashedPenaltyPool => "slashed:penalty_pool".to_string(),
            AccountKind::TreasuryMint => "treasury:mint".to_string(),
        }
    }
}

/// A single balanced leg of a double-entry transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalLeg {
    /// Target account for this leg.
    pub account: AccountKind,
    /// Debit amount in micro-credits (reduces equity/liability, increases asset).
    pub debit: u128,
    /// Credit amount in micro-credits (increases equity/liability, reduces asset).
    pub credit: u128,
}

impl JournalLeg {
    /// Creates a debit leg.
    pub fn debit(account: AccountKind, amount: u128) -> Self {
        Self {
            account,
            debit: amount,
            credit: 0,
        }
    }

    /// Creates a credit leg.
    pub fn credit(account: AccountKind, amount: u128) -> Self {
        Self {
            account,
            debit: 0,
            credit: amount,
        }
    }
}

/// An immutable, atomic double-entry ledger transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerTransaction {
    /// Unique cryptographic transaction identifier.
    pub tx_id: String,
    /// UNIX timestamp in milliseconds when the transaction occurred.
    pub timestamp_unix_ms: u64,
    /// Balanced legs of the transaction (must satisfy sum(debit) == sum(credit)).
    pub legs: Vec<JournalLeg>,
    /// Human-readable description or audit reference for this transaction.
    pub memo: String,
}

/// Dynamic pricing schedule defining unit costs in micro-credits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PricingSchedule {
    /// Base flat setup fee per task invocation (micro-credits).
    pub base_task_fee_micro: u128,
    /// Rate per 1,000,000 CPU thread microseconds (1 CPU core-second).
    pub cpu_thread_sec_rate_micro: u128,
    /// Rate per 1,000,000 WASM fuel units.
    pub wasm_fuel_million_rate_micro: u128,
    /// Rate per 1,000,000 GPU compute microseconds.
    pub gpu_compute_sec_rate_micro: u128,
    /// Rate per 1,000,000 NPU compute microseconds.
    pub npu_compute_sec_rate_micro: u128,
    /// Rate per GiB-second of VRAM residency (1 GiB = 1,073,741,824 bytes).
    pub vram_gib_sec_rate_micro: u128,
    /// Rate per 1,000 AI prompt tokens.
    pub ai_prompt_1k_tokens_rate_micro: u128,
    /// Rate per 1,000 AI generated tokens.
    pub ai_gen_1k_tokens_rate_micro: u128,
    /// Rate per 1,000 media frames encoded.
    pub media_1k_frames_rate_micro: u128,
    /// Rate per 1,000,000 render pixels completed (1 Megapixel).
    pub render_1m_pixels_rate_micro: u128,
    /// Rate per GiB network egress.
    pub network_egress_gib_rate_micro: u128,
    /// Rate per GiB-hour of storage chunk persistence.
    pub storage_gib_hour_rate_micro: u128,
}

impl Default for PricingSchedule {
    fn default() -> Self {
        Self {
            base_task_fee_micro: 100,              // 0.000100 UMCF
            cpu_thread_sec_rate_micro: 500,        // 0.000500 UMCF / CPU-s
            wasm_fuel_million_rate_micro: 200,     // 0.000200 UMCF / M-fuel
            gpu_compute_sec_rate_micro: 5_000,     // 0.005000 UMCF / GPU-s
            npu_compute_sec_rate_micro: 3_000,     // 0.003000 UMCF / NPU-s
            vram_gib_sec_rate_micro: 1_000,        // 0.001000 UMCF / GiB-s
            ai_prompt_1k_tokens_rate_micro: 1_000, // 0.001000 UMCF / 1k prompt
            ai_gen_1k_tokens_rate_micro: 4_000,    // 0.004000 UMCF / 1k gen
            media_1k_frames_rate_micro: 2_500,     // 0.002500 UMCF / 1k frames
            render_1m_pixels_rate_micro: 3_000,    // 0.003000 UMCF / Mpix
            network_egress_gib_rate_micro: 10_000, // 0.010000 UMCF / GiB
            storage_gib_hour_rate_micro: 2_000,    // 0.002000 UMCF / GiB-hr
        }
    }
}

impl PricingSchedule {
    /// Computes the exact integer payout in micro-credits for a metered resource vector.
    pub fn calculate_payout(&self, resources: &MeteredResourceVector) -> u128 {
        let mut total = self.base_task_fee_micro;

        // CPU
        total +=
            (resources.cpu_thread_time_us as u128 * self.cpu_thread_sec_rate_micro) / 1_000_000;

        // WASM Fuel
        total +=
            (resources.wasm_fuel_consumed as u128 * self.wasm_fuel_million_rate_micro) / 1_000_000;

        // GPU
        total +=
            (resources.gpu_compute_time_us as u128 * self.gpu_compute_sec_rate_micro) / 1_000_000;

        // NPU
        total +=
            (resources.npu_compute_time_us as u128 * self.npu_compute_sec_rate_micro) / 1_000_000;

        // VRAM Byte-Seconds (1 GiB = 1073741824 bytes)
        total += (resources.vram_peak_byte_seconds as u128 * self.vram_gib_sec_rate_micro)
            / 1_073_741_824;

        // AI Tokens
        total += (resources.ai_prompt_tokens as u128 * self.ai_prompt_1k_tokens_rate_micro) / 1_000;
        total += (resources.ai_generated_tokens as u128 * self.ai_gen_1k_tokens_rate_micro) / 1_000;

        // Media Frames
        total += (resources.media_frames_encoded as u128 * self.media_1k_frames_rate_micro) / 1_000;

        // Render Pixels
        total += (resources.render_pixels_completed as u128 * self.render_1m_pixels_rate_micro)
            / 1_000_000;

        // Network Egress
        total += (resources.network_egress_bytes as u128 * self.network_egress_gib_rate_micro)
            / 1_073_741_824;

        // Storage Byte-Hours (1 GiB-Hour = 1073741824 * 3600 byte-seconds)
        total += (resources.storage_byte_seconds as u128 * self.storage_gib_hour_rate_micro)
            / (1_073_741_824 * 3600);

        total
    }
}

/// Classified Byzantine fault reasons for slashing worker stake bonds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ByzantineFaultType {
    /// Worker submitted corrupted rendering tiles.
    CorruptedTile,
    /// Worker submitted corrupted video/audio segment.
    CorruptedAnnexBSegment,
    /// Worker submitted non-deterministic/divergent activation tensors.
    InvalidActivationTensor,
    /// Worker forged or replayed receipt hash.
    ForgedReceiptHash,
    /// Worker claimed impossible physical resource metrics.
    PhysicalBoundsInflation,
}

/// Cryptographic evidence proving Byzantine failure by a worker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationFailureEvidence {
    /// Unique identifier for this evidence bundle.
    pub evidence_id: String,
    /// Job ID during which fault occurred.
    pub job_id: String,
    /// Specific task identifier.
    pub task_id: String,
    /// Byzantine worker node ID.
    pub worker_node_id: String,
    /// Affected requester node ID.
    pub requester_node_id: String,
    /// Fault classification.
    pub fault_type: ByzantineFaultType,
    /// Cryptographic digest of the corrupted output or audit proof.
    pub evidence_blake3: [u8; 32],
    /// Worker's signature over the rejected output or receipt.
    pub worker_signature_hex: String,
    /// UNIX timestamp when the violation was verified.
    pub timestamp_unix_ms: u64,
}

/// State tracking for an active escrow account.
#[derive(Debug, Clone, PartialEq, Eq)]
struct EscrowState {
    requester_account: String,
    initial_budget: u128,
    remaining_balance: u128,
}

/// ACID Double-Entry Compute Credit Ledger.
///
/// Enforces:
/// 1. Zero-Sum Balance Conservation: sum(all_accounts) == 0 at all times.
/// 2. Non-negative customer balances: Available, Escrow, and Stake accounts cannot go below 0.
/// 3. Idempotency: Duplicate receipts cannot be settled twice.
#[derive(Debug, Clone, Default)]
pub struct DoubleEntryCreditLedger {
    accounts: HashMap<AccountKind, i128>,
    transactions: Vec<LedgerTransaction>,
    settled_receipts: HashSet<[u8; 32]>,
    active_escrows: HashMap<String, EscrowState>,
    total_minted_micro: u128,
}

impl DoubleEntryCreditLedger {
    /// Initializes an empty ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queries the current balance of an account in micro-credits.
    pub fn get_balance(&self, account: &AccountKind) -> i128 {
        self.accounts.get(account).copied().unwrap_or(0)
    }

    /// Returns the number of transactions recorded in the journal.
    pub fn transaction_count(&self) -> usize {
        self.transactions.len()
    }

    /// Returns the total circulating minted supply in micro-credits.
    pub fn total_minted_micro(&self) -> u128 {
        self.total_minted_micro
    }

    /// Checks if a receipt ID has already been settled.
    pub fn is_receipt_settled(&self, receipt_id: &[u8; 32]) -> bool {
        self.settled_receipts.contains(receipt_id)
    }

    /// Atomically commits a balanced multi-leg ledger transaction.
    ///
    /// Aborts with an error if:
    /// - Debits and credits do not balance.
    /// - Fewer than 2 legs are provided.
    /// - Any non-treasury account would fall below zero balance.
    pub fn commit_transaction(&mut self, tx: LedgerTransaction) -> Result<(), LedgerError> {
        if tx.legs.len() < 2 {
            return Err(LedgerError::OperationFailed(
                "Transaction must have at least 2 legs".to_string(),
            ));
        }

        let mut total_debits: u128 = 0;
        let mut total_credits: u128 = 0;

        for leg in &tx.legs {
            total_debits = total_debits
                .checked_add(leg.debit)
                .ok_or_else(|| LedgerError::OperationFailed("Debit overflow".to_string()))?;
            total_credits = total_credits
                .checked_add(leg.credit)
                .ok_or_else(|| LedgerError::OperationFailed("Credit overflow".to_string()))?;
        }

        if total_debits != total_credits {
            return Err(LedgerError::UnbalancedTransaction {
                debits: total_debits,
                credits: total_credits,
            });
        }

        // Pre-validate account balance constraints
        let mut projected_balances: HashMap<AccountKind, i128> = HashMap::new();
        for leg in &tx.legs {
            let current = projected_balances
                .get(&leg.account)
                .copied()
                .unwrap_or_else(|| self.get_balance(&leg.account));

            // In double-entry convention:
            // For credit accounts (balance tracking): Balance_new = Balance_old + credit - debit
            let delta = leg.credit as i128 - leg.debit as i128;
            let next = current + delta;

            // Non-treasury accounts must never be negative
            if !matches!(leg.account, AccountKind::TreasuryMint) && next < 0 {
                return Err(LedgerError::InsufficientBalance {
                    account: leg.account.to_account_string(),
                    available: current,
                    required: leg.debit,
                });
            }

            projected_balances.insert(leg.account.clone(), next);
        }

        // Apply state mutations
        for (account, new_balance) in projected_balances {
            self.accounts.insert(account, new_balance);
        }

        self.transactions.push(tx);
        Ok(())
    }

    /// Mints initial credit supply to a designated account.
    pub fn mint_initial_credits(
        &mut self,
        recipient: &str,
        amount_micro: u128,
        memo: &str,
    ) -> Result<String, LedgerError> {
        let tx_id = format!("tx-mint-{}", self.transactions.len() + 1);
        let tx = LedgerTransaction {
            tx_id: tx_id.clone(),
            timestamp_unix_ms: 1_731_322_000_000,
            legs: vec![
                // Treasury debited
                JournalLeg::debit(AccountKind::TreasuryMint, amount_micro),
                // Recipient credited
                JournalLeg::credit(
                    AccountKind::AvailableBalance(recipient.to_string()),
                    amount_micro,
                ),
            ],
            memo: memo.to_string(),
        };

        self.commit_transaction(tx)?;
        self.total_minted_micro += amount_micro;
        Ok(tx_id)
    }

    /// Locks funds from a requester's available balance into a job escrow account.
    pub fn lock_job_escrow(
        &mut self,
        requester: &str,
        job_id: &str,
        max_budget_micro: u128,
    ) -> Result<String, LedgerError> {
        if self.active_escrows.contains_key(job_id) {
            return Err(LedgerError::OperationFailed(format!(
                "Escrow for job '{job_id}' is already locked"
            )));
        }

        let tx_id = format!("tx-escrow-lock-{}", job_id);
        let tx = LedgerTransaction {
            tx_id: tx_id.clone(),
            timestamp_unix_ms: 1_731_322_000_000,
            legs: vec![
                JournalLeg::debit(
                    AccountKind::AvailableBalance(requester.to_string()),
                    max_budget_micro,
                ),
                JournalLeg::credit(
                    AccountKind::EscrowLocked(job_id.to_string()),
                    max_budget_micro,
                ),
            ],
            memo: format!("Lock escrow budget for job {job_id}"),
        };

        self.commit_transaction(tx)?;

        self.active_escrows.insert(
            job_id.to_string(),
            EscrowState {
                requester_account: requester.to_string(),
                initial_budget: max_budget_micro,
                remaining_balance: max_budget_micro,
            },
        );

        Ok(tx_id)
    }

    /// Settles a verified `SignedResourceReceipt` from the job's locked escrow into the worker's balance.
    ///
    /// Verifies both Ed25519 signatures and guarantees idempotent settlement.
    pub fn settle_verified_receipt(
        &mut self,
        receipt: &SignedResourceReceipt,
        pricing: &PricingSchedule,
        worker_vk: &VerifyingKey,
        requester_vk: &VerifyingKey,
    ) -> Result<u128, LedgerError> {
        // 1. Idempotency check: prevent double-spend
        if self.is_receipt_settled(&receipt.receipt_id) {
            return Err(LedgerError::ReceiptAlreadySettled {
                receipt_id: hex::encode(receipt.receipt_id),
            });
        }

        // 2. Cryptographic signature verification
        receipt.verify_worker_signature(worker_vk)?;
        receipt.verify_requester_countersignature(requester_vk)?;

        // 3. Escrow existence and balance check
        let job_id = receipt.body.job_id.clone();
        let remaining_balance = self
            .active_escrows
            .get(&job_id)
            .map(|e| e.remaining_balance)
            .ok_or_else(|| LedgerError::EscrowNotFound {
                job_id: job_id.clone(),
            })?;

        // 4. Calculate exact payout in micro-credits
        let payout = pricing.calculate_payout(&receipt.body.resources);
        if payout > remaining_balance {
            return Err(LedgerError::EscrowDepleted {
                job_id: job_id.clone(),
                remaining: remaining_balance,
                requested: payout,
            });
        }

        // 5. Commit settlement transaction: Debit Escrow, Credit Worker
        let tx_id = format!("tx-settle-{}", hex::encode(&receipt.receipt_id[0..8]));
        let tx = LedgerTransaction {
            tx_id,
            timestamp_unix_ms: receipt.body.completed_at_unix_ms,
            legs: vec![
                JournalLeg::debit(AccountKind::EscrowLocked(job_id.clone()), payout),
                JournalLeg::credit(
                    AccountKind::AvailableBalance(receipt.body.worker_node_id.clone()),
                    payout,
                ),
            ],
            memo: format!(
                "Receipt settlement seq {} for job {job_id}",
                receipt.body.sequence_number
            ),
        };

        self.commit_transaction(tx)?;

        // 6. Update local escrow state and record settled receipt
        if let Some(escrow) = self.active_escrows.get_mut(&job_id) {
            escrow.remaining_balance -= payout;
        }
        self.settled_receipts.insert(receipt.receipt_id);

        Ok(payout)
    }

    /// Finalizes a completed job, releasing all remaining unused escrow back to the requester.
    pub fn finalize_job_and_refund_unused_escrow(
        &mut self,
        job_id: &str,
    ) -> Result<u128, LedgerError> {
        let escrow =
            self.active_escrows
                .remove(job_id)
                .ok_or_else(|| LedgerError::EscrowNotFound {
                    job_id: job_id.to_string(),
                })?;

        let refund_amount = escrow.remaining_balance;
        if refund_amount > 0 {
            let tx_id = format!("tx-refund-{}", job_id);
            let tx = LedgerTransaction {
                tx_id,
                timestamp_unix_ms: 1_731_322_000_000,
                legs: vec![
                    JournalLeg::debit(AccountKind::EscrowLocked(job_id.to_string()), refund_amount),
                    JournalLeg::credit(
                        AccountKind::AvailableBalance(escrow.requester_account.clone()),
                        refund_amount,
                    ),
                ],
                memo: format!("Refund unused escrow for job {job_id}"),
            };

            self.commit_transaction(tx)?;
        }

        Ok(refund_amount)
    }

    /// Deposits collateral into a worker's provider stake bond account.
    pub fn deposit_provider_stake(
        &mut self,
        worker: &str,
        amount_micro: u128,
    ) -> Result<String, LedgerError> {
        let tx_id = format!("tx-stake-{}", worker);
        let tx = LedgerTransaction {
            tx_id: tx_id.clone(),
            timestamp_unix_ms: 1_731_322_000_000,
            legs: vec![
                JournalLeg::debit(
                    AccountKind::AvailableBalance(worker.to_string()),
                    amount_micro,
                ),
                JournalLeg::credit(
                    AccountKind::ProviderStakeBond(worker.to_string()),
                    amount_micro,
                ),
            ],
            memo: format!("Bond collateral stake for worker {worker}"),
        };

        self.commit_transaction(tx)?;
        Ok(tx_id)
    }

    /// Slashes a Byzantine worker's bonded collateral stake.
    ///
    /// Debits `ProviderStakeBond(worker)`:
    /// - Credits `AvailableBalance(requester)` with `compensation_micro` (compensates requester).
    /// - Credits `SlashedPenaltyPool` with remaining penalty.
    pub fn slash_byzantine_worker(
        &mut self,
        evidence: &VerificationFailureEvidence,
        slash_amount_micro: u128,
        compensation_micro: u128,
    ) -> Result<String, LedgerError> {
        if slash_amount_micro < compensation_micro {
            return Err(LedgerError::ByzantineSlashingEvidenceInvalid {
                reason: "Slash amount must be >= compensation amount".to_string(),
            });
        }

        let penalty_pool_credit = slash_amount_micro - compensation_micro;
        let mut legs = vec![
            JournalLeg::debit(
                AccountKind::ProviderStakeBond(evidence.worker_node_id.clone()),
                slash_amount_micro,
            ),
            JournalLeg::credit(
                AccountKind::AvailableBalance(evidence.requester_node_id.clone()),
                compensation_micro,
            ),
        ];

        if penalty_pool_credit > 0 {
            legs.push(JournalLeg::credit(
                AccountKind::SlashedPenaltyPool,
                penalty_pool_credit,
            ));
        }

        let tx_id = format!("tx-slash-{}", evidence.evidence_id);
        let tx = LedgerTransaction {
            tx_id: tx_id.clone(),
            timestamp_unix_ms: evidence.timestamp_unix_ms,
            legs,
            memo: format!(
                "Byzantine slash on worker {} for {:?}",
                evidence.worker_node_id, evidence.fault_type
            ),
        };

        self.commit_transaction(tx)?;
        Ok(tx_id)
    }

    /// Mathematical zero-sum conservation proof:
    /// Sum of all circulating balances (Available, Escrow, Stake, Penalty) must equal
    /// the exact total minted supply, with 0 micro-credits discrepancy.
    pub fn verify_global_zero_sum_invariant(&self) -> bool {
        let mut circulating_sum: i128 = 0;
        let mut treasury_balance: i128 = 0;

        for (account, balance) in &self.accounts {
            if matches!(account, AccountKind::TreasuryMint) {
                treasury_balance += balance;
            } else {
                circulating_sum += balance;
            }
        }

        // In double entry: Treasury balance is negative of minted supply
        let matches_treasury = treasury_balance + circulating_sum == 0;
        let matches_minted = circulating_sum == (self.total_minted_micro as i128);

        matches_treasury && matches_minted
    }
}
