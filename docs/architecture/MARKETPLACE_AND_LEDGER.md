# UMCF Architecture: Marketplace, Credit Accounting & Cryptographic Ledger

This document defines the mathematical, cryptographic, and accounting architecture of the Universal Mobile Compute Fabric (UMCF) compute credit ledger, marketplace order book, and resource metering system.

---

## 1. Multi-Dimensional Resource Metering & Cryptographic Receipts

### 1.1 Invariant: Zero Floating-Point Ambiguity
In accordance with `ENGINEERING_CONTRACT.md` and Rule 01 (`crates/umcf-ledger`), floating-point values (`f32`, `f64`) are **strictly prohibited** in any metering, accounting, receipt, or balance calculation. Floats introduce platform-dependent rounding discrepancies across x86_64, ARM64, and WebAssembly targets, destroying deterministic signature and hash-chain verification.

All resource counters are represented strictly as unsigned 64-bit integers (`u64`):

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeteredResourceVector {
    pub cpu_thread_time_us: u64,
    pub wasm_fuel_consumed: u64,
    pub gpu_compute_time_us: u64,
    pub npu_compute_time_us: u64,
    pub vram_peak_byte_seconds: u64,
    pub ai_prompt_tokens: u64,
    pub ai_generated_tokens: u64,
    pub ai_layer_activations_transferred: u64,
    pub media_frames_encoded: u64,
    pub render_pixels_completed: u64,
    pub network_egress_bytes: u64,
    pub network_ingress_bytes: u64,
    pub storage_byte_seconds: u64,
}
```

### 1.2 Canonical 104-Byte Little-Endian Serialization
A `MeteredResourceVector` serializes deterministically to an exact 104-byte little-endian binary slice (13 fields $\times$ 8 bytes = 104 bytes). This ensures canonical BLAKE3 digests across all architectures.

### 1.3 Two-Phase Dual-Signed Countersigning Protocol
To prevent unilateral worker overbilling or requester non-payment:
1. **Worker Task Execution**: Worker node executes the requested workload, measures real hardware consumption via `ResourceMeteringCollector`, and computes the BLAKE3 digest of the produced output.
2. **Worker Signature**: Worker signs the canonical `ReceiptBody` with its Ed25519 private key:
   $$\text{receipt\_id} = \text{BLAKE3}(\text{canonical}(\text{ReceiptBody}))$$
   $$\sigma_{\text{worker}} = \text{Ed25519}_{\text{sign}}(K_{\text{worker\_sk}}, \text{receipt\_id})$$
3. **Requester Verification & Sanity Bounds Check**: Upon receiving the task result and worker-signed receipt, the requester:
   - Verifies $\sigma_{\text{worker}}$ against $K_{\text{worker\_vk}}$.
   - Verifies $\text{BLAKE3}(\text{actual\_output}) == \text{body.output\_blake3}$.
   - Runs `SanityBoundsChecker` to assert that claimed metrics fit physical limits (e.g. generated tokens $\le$ requested max tokens; render pixels $\le$ canvas dimensions; CPU thread time $\le \text{duration} \times \text{cores}$).
4. **Requester Countersignature**: If all checks pass, requester countersigns:
   $$\sigma_{\text{requester}} = \text{Ed25519}_{\text{sign}}(K_{\text{requester\_sk}}, \text{receipt\_id})$$

### 1.4 Hash-Chained Auditing (`prev_receipt_id`)
Every `(worker_node, requester_node)` peer pair maintains a monotonically increasing sequence number ($1, 2, 3, \dots$) and links each receipt to the preceding receipt's `receipt_id`:
$$\text{prev\_receipt\_id}_{k} = \text{receipt\_id}_{k-1}$$
The initial receipt in a pair uses `GENESIS_PREV_RECEIPT_ID` ($32 \times \text{0x00}$). Any missing sequence number or mismatched `prev_receipt_id` halts verification and flags a replay or omission attack.

### 1.5 60-Second Incremental Checkpoints for Streams
For long-lived interactive sessions (e.g. AI token streaming, remote desktop, cloud gaming via `umcf-stream`), `CheckpointEmitter` issues micro-receipts every 60 seconds. If either node disconnects or defaults, the uncompensated risk is capped at 60 seconds.

---

## 2. Double-Entry Compute Credit Ledger

### 2.1 Integer Micro-Credit Denomination
All monetary and compute credit calculations use integer micro-credits:
$$1\text{ UMCF Credit} = 1{,}000{,}000\text{ Micro-Credits}$$
Floating-point values are strictly forbidden to guarantee consistent ledger outcomes across all platforms.

### 2.2 Account Taxonomy
1. **`AvailableBalance(account_id)`**: Spendable funds owned by a node or user.
2. **`EscrowLocked(job_id)`**: Funds locked during job execution to guarantee worker compensation.
3. **`ProviderStakeBond(node_id)`**: Collateral deposited by compute providers to participate in marketplace workloads. Subject to slashing upon Byzantine failure.
4. **`SlashedPenaltyPool`**: Sink pool for forfeited collateral from slashed Byzantine providers.
5. **`TreasuryMint`**: Originating debit source for newly minted credit supply.

### 2.3 Double-Entry Conservation Invariant
Every transaction $\mathcal{T}$ comprises $N \ge 2$ journal legs $\mathcal{L}_i = (\text{account}_i, \text{debit}_i, \text{credit}_i)$ satisfying:
$$\sum_{i=1}^N \text{debit}_i = \sum_{i=1}^N \text{credit}_i$$
No non-treasury account may ever have a balance $< 0$:
$$\forall a \ne \text{TreasuryMint}: \text{Balance}(a) \ge 0$$
Global conservation of value guarantees:
$$\sum_{a \ne \text{TreasuryMint}} \text{Balance}(a) = \text{TotalMintedSupply}$$

### 2.4 Four Core Settlement Flows
1. **`lock_job_escrow(requester, job_id, max_budget)`**:
   - $\text{Debit: AvailableBalance(requester)}, \text{max\_budget}$
   - $\text{Credit: EscrowLocked(job\_id)}, \text{max\_budget}$
2. **`settle_verified_receipt(receipt, pricing)`**:
   - Computes $\text{payout} = \text{pricing.calculate\_payout}(\text{receipt.resources})$.
   - Checks idempotency: rejects duplicate `receipt_id`.
   - $\text{Debit: EscrowLocked(job\_id)}, \text{payout}$
   - $\text{Credit: AvailableBalance(worker)}, \text{payout}$
3. **`finalize_job_and_refund_unused_escrow(job_id)`**:
   - $\text{Debit: EscrowLocked(job\_id)}, \text{remaining}$
   - $\text{Credit: AvailableBalance(requester)}, \text{remaining}$
4. **`slash_byzantine_worker(evidence, slash_amount, compensation)`**:
   - $\text{Debit: ProviderStakeBond(worker)}, \text{slash\_amount}$
   - $\text{Credit: AvailableBalance(requester)}, \text{compensation}$
   - $\text{Credit: SlashedPenaltyPool}, (\text{slash\_amount} - \text{compensation})$

---

## 3. Compute Marketplace Order Matching & Auctions

### 3.1 Order Book Schemas
- **`ComputeAskOffer`**: Published by providers offering compute capacity. Specifies CPU cores, RAM, GPU/VRAM, NPU TOPS, storage, bandwidth, baseline latency, minimum `PricingSchedule`, reputation score, and safety constraints (`only_when_charging`, `only_on_unmetered_wifi`, `max_thermal_level`).
- **`ComputeBidRequest`**: Published by clients requesting capacity. Specifies minimum hardware requirements, privacy tier (`StrictLocal`, `TrustedDevices`, `PublicMarket`), maximum RTT latency, and maximum budget.

### 3.2 Constraint Filtering
An ask offer is eligible for a bid if and only if:
1. `is_active == true`
2. $\text{only\_when\_charging} \implies \text{is\_charging}$
3. $\text{only\_on\_unmetered\_wifi} \implies \text{is\_unmetered\_wifi}$
4. $\text{current\_thermal\_level} \le \text{max\_thermal\_level}$
5. $\text{HardwareProfile} \ge \text{BidRequirements}$
6. $\text{reputation\_score} \ge \text{min\_reputation\_score}$
7. $\text{baseline\_rtt\_ms} \le \text{max\_latency\_rtt\_ms}$
8. $\text{max\_budget} \ge \text{pricing.base\_task\_fee}$

### 3.3 Utility Score Ranking
Eligible offers are ranked using a multi-factor utility function:
$$\mathcal{U} = 10 \cdot \text{reputation} + \max(0, 5000 - 20 \cdot \text{rtt}) + \text{HeadroomBonus} + \max(0, 5000 - \text{base\_fee})$$
The highest-utility offer is matched.

### 3.4 Cryptographically Sealed Lease Contracts
Once matched, both provider and client sign the canonical contract digest:
$$\text{contract\_digest} = \text{BLAKE3}(\text{lease\_id} \parallel \text{offer\_id} \parallel \text{bid\_id} \parallel \text{budget} \parallel \text{timestamp})$$
Both Ed25519 signatures are stored in `MatchedLeaseContract` to lock the agreed rates for the duration of the job.
