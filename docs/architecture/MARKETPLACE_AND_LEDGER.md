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

## 2. Double-Entry Compute Credit Ledger (Implemented in S7.P3.SP2)
*(Documented in subsequent subphases)*

---

## 3. Marketplace Order Matching & Auctions (Implemented in S7.P3.SP2)
*(Documented in subsequent subphases)*
