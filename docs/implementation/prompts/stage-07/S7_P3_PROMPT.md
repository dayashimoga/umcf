# Phase S7.P3 Execution Prompt: Resource Accounting, Compute Marketplace Foundation & Admin Consoles

## 1. Role & Context
You are the Distributed Economic Systems, Cryptographic Accounting & Full-Stack UI Lead for UMCF. Phase `S7.P3` builds `crates/umcf-ledger` and `crates/umcf-marketplace`, providing non-repudiable dual-signed cryptographic resource metering receipts across CPU, GPU, NPU, network bandwidth, and storage, an ACID double-entry compute credit ledger with escrow and Byzantine slashing, a decentralized bid/ask compute marketplace matching engine, and the complete production integration of the Flutter mobile/desktop app (`ui/flutter_app`) and Svelte 5 Admin Dashboard (`ui/admin_web`).

This phase consists of three sequential subphases:
1. `S7_P3_SP1.md`: Signed Cryptographic Resource Metering Receipts (CPU/GPU/NPU/Bandwidth/Storage)
2. `S7_P3_SP2.md`: Double-Entry Compute Credit Ledger & Marketplace Order Matching
3. `S7_P3_SP3.md`: Complete Flutter Mobile/Desktop UI & Svelte 5 Admin Dashboard Integration

## 2. Prerequisites & Dependencies
- Completion of Phases `S7.P1` and `S7.P2`, plus all Stage 1–6 compute engines (`umcf-wasm`, `umcf-ai`, `umcf-gpu`, `umcf-media`, `umcf-stream`, `umcf-storage`).

## 3. Implementation Tasks
1. **Subphase S7.P3.SP1 (`crates/umcf-ledger/src/receipts.rs`, `src/meter.rs`)**:
   - Implement `ResourceMeteringCollector` capturing hardware-verified CPU core-milliseconds, WASM fuel units, GPU compute/shader microseconds, AI prompt/decode token counts & layer-bytes, transcoded video frames, network egress/ingress bytes, and storage chunk-byte-hours.
   - Implement dual-signed `SignedResourceReceipt` (signed by both the Worker `NodeId` and Requester/Coordinator `NodeId` over canonical CBOR/bincode bytes, chained via `prev_receipt_blake3` hash links to prevent replay or omission).
2. **Subphase S7.P3.SP2 (`crates/umcf-ledger/src/ledger.rs`, `crates/umcf-marketplace/src/orderbook.rs`)**:
   - Implement `DoubleEntryCreditLedger` using integer micro-credits (`1 Credit = 1_000_000 micro-credits`, `i128` fixed-point math—zero floating-point rounding) with pre-job Escrow Lock, Receipt-driven Settlement, Refund on cancellation, and Byzantine Slashing (triggered by `VerificationFailureEvidence` from Stage 5/6).
   - Implement `MarketplaceMatchingEngine` matching `ComputeAskOffer` (provider device capabilities, availability schedule, min price per unit) with `ComputeBidRequest` (workload requirements, max budget, privacy/trust tier, latency SLA).
3. **Subphase S7.P3.SP3 (`ui/flutter_app`, `ui/admin_web`)**:
   - Wire every screen, chart, control action, and WebSocket event stream in both the Flutter mobile/desktop application and the Svelte 5 web administration console to live backend services with zero hardcoded mock data.

## 4. Acceptance Criteria
- >= 95% line/branch coverage across `crates/umcf-ledger`, `crates/umcf-marketplace`, and UI state/controller layers; 100% test pass rate.
- Every `SignedResourceReceipt` requires valid Ed25519 signatures from both worker and requester (or verified storage PoR challenge) before the ledger settles escrow credits.
- Double-entry ledger invariant $\sum_{a \in \text{Accounts}} \text{Balance}(a) == \text{TotalMintedSupply}$ holds with 0 micro-credit drift across 10,000 concurrent transactions.
- 100% of Flutter screens and Svelte 5 routes render live data from `umcf-api-gateway` and pass automated widget/Playwright end-to-end tests.

## 5. Required Tests
- **`[UNIT]`**: Receipt hash-chain validation, fixed-point micro-credit pricing formulas, double-entry ACID journal balancing, price-time-capability order matching, UI state stores.
- **`[INT]`**: Full lifecycle: Provider publishes Ask -> Requester submits Bid + locks Escrow -> Job executes -> Dual-signed Receipt generated -> Ledger settles Provider payout + refunds unused Escrow; Byzantine worker caught by `CanaryAuditor` -> Stake slashed.
- **`[SIM]`**: 500-node marketplace auction simulation with dynamic surge pricing and concurrent escrow settlements.
- **`[HW]`**: End-to-end Flutter app (Android/Desktop) + Svelte 5 Web Admin live interaction test against a multi-node cluster.
- **`[MANUAL]`**: Complete visual and functional walkthrough of all 8 major feature areas in Flutter and Svelte 5.

## 6. Required Evidence & Artifacts
- `artifacts/stage-07/p3/receipt_chain_verification_audit.json`
- `artifacts/stage-07/p3/ledger_double_entry_conservation_report.json`
- `artifacts/stage-07/p3/ui_e2e_playwright_and_flutter_test_report.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Create `docs/architecture/MARKETPLACE_AND_LEDGER.md` and `docs/ui/ADMIN_AND_MOBILE_UI_GUIDE.md`.
- Mark `S7.P3.SP1`–`S7.P3.SP3` and all of Stage 07 `[x]` in `TODO.md`; update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: Any floating-point arithmetic in ledger balances, any unilateral/unsigned receipt settlement, or any UI screen still referencing mock/static placeholder arrays.
- **Gate**: Stage 07 complete. Unlocks Stage 08 (`stage-08`).
