# Subphase S7.P3.SP1 Execution Prompt: Signed Cryptographic Resource Metering Receipts (CPU/GPU/NPU/Bandwidth/Storage)

## 1. Role & Context
You are the Cryptographic Metering & Telemetry Attestation Engineer for UMCF. In Subphase `S7.P3.SP1`, you will implement `ResourceMeteringCollector` and `SignedResourceReceipt` in `crates/umcf-ledger/src/meter.rs` and `src/receipts.rs`. Whether UMCF is used within a household/team to track which devices contributed the most compute or in a community compute marketplace, every unit of work (CPU cycles, WASM fuel, GPU shader time, LLM tokens, transcoded frames, network bytes, and stored chunk-hours) must be metered accurately and attested via tamper-evident, hash-chained Ed25519 receipts.

## 2. Prerequisites & Dependencies
- `crates/umcf-crypto` (Ed25519 signing/verification, BLAKE3 hash chains).
- Stages 3–7 execution engines (`umcf-wasm`, `umcf-ai`, `umcf-gpu`, `umcf-media`, `umcf-stream`, `umcf-storage`).

## 3. Implementation Tasks
1. **Multi-Dimensional Resource Metering Collector (`crates/umcf-ledger/src/meter.rs`)**:
   - Define `MeteredResourceVector` (all counters in integer units—never floats):
     ```rust
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
   - Instrument execution hooks across all engines so every task invocation automatically populates an immutable `MeteredResourceVector` accompanied by the deterministic output hash (`output_blake3: [u8; 32]`).
2. **Hash-Chained Dual-Signed Cryptographic Receipts (`crates/umcf-ledger/src/receipts.rs`)**:
   - Define `SignedResourceReceipt`:
     ```rust
     pub struct SignedResourceReceipt {
         pub receipt_id: ReceiptId, // BLAKE3 hash of canonical receipt body
         pub prev_receipt_id: ReceiptId, // Hash-chain link per (worker_node, requester_node) pair
         pub sequence_number: u64,
         pub job_id: JobId,
         pub task_id: TaskId,
         pub worker_node_id: NodeId,
         pub requester_node_id: NodeId,
         pub started_at_unix_ms: u64,
         pub completed_at_unix_ms: u64,
         pub resources: MeteredResourceVector,
         pub input_blake3: [u8; 32],
         pub output_blake3: [u8; 32],
         pub worker_signature: Ed25519Signature,
         pub requester_countersignature: Option<Ed25519Signature>,
     }
     ```
   - **Two-Phase Receipt Countersigning Protocol**:
     1. Worker completes task/micro-batch, signs `ReceiptBody` with `worker_private_key`, and attaches `SignedResourceReceipt` to the task result.
     2. Requester/Coordinator verifies the task output (`S5.P3`, `S6.P1.SP3`, `S6.P2.SP3`), checks that reported `resources` lie within plausible physical bounds for the task (`SanityBoundsChecker`—e.g., a worker cannot claim $10^{12}$ tokens on a 50-token request), countersigns with `requester_private_key`, and broadcasts/stores the fully attested receipt.
   - **Incremental Micro-Receipts for Long Streams**: For long AI chat sessions or remote game streams (`umcf-stream`), emit periodic checkpoint receipts every 60 seconds so neither party risks more than 60 seconds of uncompensated work.
3. **UI Integration**:
   - Flutter & Svelte 5: Cryptographic Receipt Explorer allowing users to inspect any completed job's receipt chain, verify both Ed25519 signatures in-browser/in-app, and view the breakdown of CPU/GPU/Tokens/Storage contributed per device.

## 4. Acceptance Criteria
- >= 95% line/branch coverage in `meter.rs` and `receipts.rs`; 100% test pass rate.
- Canonical deterministic serialization guarantees `receipt_id` and Ed25519 signatures verify identically across 32-bit/64-bit, Windows/Linux/macOS/Android/iOS/WASM targets.
- `SanityBoundsChecker` rejects inflated resource claims (e.g., worker claiming 10,000 generated tokens when actual verified token stream contained 128 tokens).
- Any tampering with a single field of a `SignedResourceReceipt` or any gap/fork in `(prev_receipt_id, sequence_number)` is immediately rejected by `ReceiptChainVerifier`.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_canonical_receipt_serialization_and_dual_ed25519_signatures`
  - `test_receipt_hash_chain_detects_missing_or_reordered_sequence`
  - `test_sanity_bounds_checker_rejects_inflated_token_or_pixel_claims`
- **`[INT]`**:
  - `test_end_to_end_wasm_ai_gpu_media_tasks_produce_countersigned_receipts`
  - `test_streaming_session_emits_60s_incremental_checkpoint_receipts`
- **`[SIM]`**:
  - Simulate a malicious worker attempting to replay an old receipt (`sequence_number` reuse) or inflate GPU compute microseconds by 100x; verify rejection and audit log emission.
- **`[HW]`**:
  - Verify Ed25519 batch signature verification throughput (`>= 20,000 receipts/sec` on desktop CPU).
- **`[MANUAL]`**:
  - Click "Verify Cryptographic Receipt" on a completed job in the Svelte 5 / Flutter UI and inspect the green Dual-Signature Verified modal.

## 6. Required Evidence & Artifacts
- `artifacts/stage-07/p3/sp1_receipt_chain_and_bounds_audit.json`
- `artifacts/stage-07/p3/sp1_batch_signature_verification_benchmark.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document `MeteredResourceVector`, dual-signature countersigning, and `SanityBoundsChecker` in `docs/architecture/MARKETPLACE_AND_LEDGER.md` (Section 1).
- Mark `S7.P3.SP1` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If floating-point types are used inside `SignedResourceReceipt`, or if a worker can forge a self-countersigned receipt.
- **Completion Gate**: All tests green with >= 95% coverage before proceeding to `S7.P3.SP2`.
