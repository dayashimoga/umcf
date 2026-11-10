# Subphase S7.P3.SP2 Execution Prompt: Double-Entry Compute Credit Ledger & Marketplace Order Matching

## 1. Role & Context
You are the Financial Ledger & Decentralized Marketplace Engine Architect for UMCF. In Subphase `S7.P3.SP2`, you will implement `DoubleEntryCreditLedger` in `crates/umcf-ledger/src/ledger.rs` and `MarketplaceMatchingEngine` in `crates/umcf-marketplace/src/orderbook.rs`. This subsystem turns verified `SignedResourceReceipt`s into provably balanced double-entry micro-credit settlements, manages pre-job escrow locks and Byzantine slashing penalties, and matches compute providers (`Ask`) with workload requesters (`Bid`).

## 2. Prerequisites & Dependencies
- `S7.P3.SP1` (`SignedResourceReceipt`, `MeteredResourceVector`).
- `S6.P1.SP3` & `S6.P2.SP3` (`VerificationFailureEvidence` for slashing).

## 3. Implementation Tasks
1. **ACID Double-Entry Compute Credit Ledger (`crates/umcf-ledger/src/ledger.rs`)**:
   - Represent all monetary/credit values in integer **Micro-Credits** (`type MicroCredits = i128`, where `1_000_000 micro-credits == 1.000000 UMCF Credit`). Forbid `f32`/`f64` in all accounting calculations.
   - Define standard account types per `NodeId` / `UserId`:
     - `AccountKind::AvailableBalance(AccountId)`
     - `AccountKind::EscrowLocked(JobId)`
     - `AccountKind::ProviderStakeBond(NodeId)`
     - `AccountKind::SlashedPenaltyPool`
     - `AccountKind::TreasuryMint`
   - Enforce the **Double-Entry Journal Invariant** on every `LedgerTransaction`:
     - Every transaction consists of $\ge 2$ `JournalLeg { account, debit: u128, credit: u128 }` entries where $\sum \text{debits} == \sum \text{credits}$ and no `AvailableBalance` or `EscrowLocked` account may ever go negative (`Balance < 0` aborts the ACID transaction).
   - Implement the 4 core settlement flows:
     1. **`lock_job_escrow(requester, job_id, max_budget_micro)`**: Debits `AvailableBalance(requester)`, Credits `EscrowLocked(job_id)`.
     2. **`settle_verified_receipt(receipt: &SignedResourceReceipt, pricing: &PricingSchedule)`**: Verifies both Ed25519 signatures and receipt idempotency (`!settled_receipts.contains(receipt_id)`), calculates exact payout from `MeteredResourceVector`, Debits `EscrowLocked(job_id)`, Credits `AvailableBalance(worker)`.
     3. **`finalize_job_and_refund_unused_escrow(job_id)`**: Refunds remaining `EscrowLocked(job_id)` balance back to `AvailableBalance(requester)`.
     4. **`slash_byzantine_worker(evidence: &VerificationFailureEvidence)`**: Verifies cryptographic proof of corrupted tile/segment/activation submission, Debits `ProviderStakeBond(worker)`, Credits `SlashedPenaltyPool` (and compensates the requester for wasted time).
2. **Compute Marketplace Order Book & Matching Engine (`crates/umcf-marketplace/src/orderbook.rs`)**:
   - Define `ComputeAskOffer` (published by idle/semi-idle devices):
     - Advertises hardware profile (CPU cores, GPU backend & VRAM, NPU TOPS, available storage, network bandwidth), availability constraints (`OnlyWhenCharging`, `OnlyOnUnmeteredWifi`, `MaxThermalLevel`, time windows), minimum rate table (`MicroCredits` per GPU-second, per 1k tokens, per GiB-hour), and reputation score.
   - Define `ComputeBidRequest` (submitted by jobs):
     - Specifies required hardware capabilities, minimum provider reputation, privacy/trust tier, maximum latency RTT, and max budget (`MicroCredits`).
   - Implement `MarketplaceMatchingEngine::match_bid(&ComputeBidRequest) -> Result<MatchedLeaseContract, MarketError>`:
     - Filters eligible `ComputeAskOffer`s satisfying all hard capability, privacy, and battery/Wi-Fi constraints.
     - Ranks matching offers using a utility score combining lowest total cost, highest throughput/lowest latency, and provider reliability reputation.
     - Emits a dual-signed `MatchedLeaseContract` locking the agreed `PricingSchedule` for the job's duration.
3. **UI Integration**:
   - Flutter & Svelte 5 Marketplace & Wallet screens: Balance & Escrow overview card, interactive "Contribute My Device (Earn Credits)" toggle with "Only while charging on Wi-Fi" safety rules, Live Order Book (Active Asks & Bids), and Double-Entry Transaction Ledger table with CSV/JSON export.

## 4. Acceptance Criteria
- >= 95% line/branch coverage in `ledger.rs` and `orderbook.rs`; 100% test pass rate.
- **Zero-Sum Conservation Law**: Across any sequence of mints, escrow locks, partial receipt settlements, refunds, and slashings, the sum of all account balances equals the exact minted supply with **0 micro-credit discrepancy**.
- **Idempotency & Double-Spend Prevention**: Submitting the same `SignedResourceReceipt` twice returns `LedgerError::ReceiptAlreadySettled` on the second attempt without crediting the worker twice; attempting to lock escrow exceeding `AvailableBalance` fails atomically.
- **Mobile Safety Constraints**: A mobile provider node with `OnlyWhenCharging = true` and `OnlyOnUnmeteredWifi = true` is immediately delisted/paused from matching the instant it is unplugged from AC power or switches to cellular data.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_double_entry_journal_rejects_unbalanced_legs_and_negative_balances`
  - `test_escrow_lock_partial_receipt_settlement_and_unused_refund_exact_math`
  - `test_idempotent_receipt_settlement_prevents_double_payout`
  - `test_byzantine_verification_failure_evidence_slashes_provider_bond`
  - `test_marketplace_matcher_respects_charging_wifi_and_privacy_constraints`
- **`[INT]`**:
  - `test_full_marketplace_lifecycle_ask_bid_match_escrow_execute_receipt_settle_refund`
  - `test_concurrent_64_thread_settlements_preserve_global_zero_sum_invariant`
- **`[SIM]`**:
  - Property-based (`proptest`) simulation of 5,000 randomized market bids, asks, cancellations, and slashing events asserting global balance conservation after every operation.
- **`[HW]`**:
  - Benchmark matching engine latency across 10,000 active `ComputeAskOffer`s (< 5 ms per match target).
- **`[MANUAL]`**:
  - Enable "Contribute Device Compute" in Flutter/Svelte UI, configure pricing and battery rules, run a job from a second node, and verify live credit transfer in the Wallet Ledger tab.

## 6. Required Evidence & Artifacts
- `artifacts/stage-07/p3/sp2_proptest_5000_tx_conservation_proof.json`
- `artifacts/stage-07/p3/sp2_marketplace_matching_benchmark.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document double-entry accounting rules, escrow/slashing state machine, and marketplace matching algorithm in `docs/architecture/MARKETPLACE_AND_LEDGER.md` (Sections 2–3).
- Mark `S7.P3.SP2` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If any transaction can create or destroy micro-credits out of thin air, if duplicate receipts can be settled twice, or if unplugged/cellular mobile devices are matched when user safety rules forbid it.
- **Completion Gate**: All tests green with >= 95% coverage before proceeding to `S7.P3.SP3`.
