# Subphase S7.P1.SP3 Execution Prompt: Offline Caching, Pinning & Garbage Collection

## 1. Role & Context
You are the Storage Lifecycle, Caching & Garbage Collection Engineer for UMCF. In Subphase `S7.P1.SP3`, you will implement `PinManager`, `WTinyLfuChunkCache`, and `ConcurrentGarbageCollector` in `crates/umcf-storage/src/pinning.rs` and `src/gc.rs`. Mobile devices frequently transition offline (airplane mode, commute, field work); users must be able to pin critical models, datasets, and job outputs for guaranteed offline availability, while unpinned transient cache chunks are automatically managed via scan-resistant W-TinyLFU eviction and safe mark-and-sweep garbage collection.

## 2. Prerequisites & Dependencies
- `S7.P1.SP1` (`ContentAddressedChunkStore`, `StrictQuotaEnforcer`) and `S7.P1.SP2` (`ReedSolomonErasureEngine`).

## 3. Implementation Tasks
1. **Hierarchical Pinning & Offline Availability Manager (`crates/umcf-storage/src/pinning.rs`)**:
   - Define `PinRecord`:
     ```rust
     pub struct PinRecord {
         pub target_cid: ObjectCid,
         pub mode: PinMode, // RecursivePermanent, LeaseWithTtl { expires_at_ms: u64 }, ActiveJobHold { job_id: JobId }
         pub label: String,
         pub pinned_by: NodeId,
         pub local_hydration_state: HydrationState, // Hydrating { progress_pct }, FullyAvailableOffline, Degraded
     }
     ```
   - When an `ObjectCid` is pinned locally with `RecursivePermanent` or `LeaseWithTtl`:
     - Immediately fetch and reassemble all constituent plaintext/encrypted chunks locally (hydrating from remote Reed-Solomon shards if only partial shards were present) so the object can be read at full local disk speed with **zero network connectivity**.
     - Increment persistent pin reference counts on the root manifest and all child `ChunkId`s in a single ACID transaction.
2. **Scan-Resistant W-TinyLFU Cache Policy (`WTinyLfuChunkCache`)**:
   - Manage unpinned chunks (e.g., opportunistically cached remote shards, intermediate map/reduce outputs, recently viewed media segments) using a Count-Min Sketch frequency estimator + Window/Probationary/Protected LRU segments so a one-time large sequential media scan never evicts frequently used AI model or WASM module chunks.
3. **Concurrent Mark-and-Sweep Garbage Collector (`crates/umcf-storage/src/gc.rs`)**:
   - Implement `ConcurrentGarbageCollector::run_gc_cycle(&GcConfig) -> Result<GcSummaryReport, StorageError>`:
     - **Root Set Enumeration**: Collect all `ObjectCid`s and `ChunkId`s reachable from:
       1. Active `RecursivePermanent` and non-expired `LeaseWithTtl` pins.
       2. Active or queued scheduler jobs (`ActiveJobHold`).
       3. Registered local GGUF models (`umcf-gguf` catalog).
       4. In-flight write reservations (`QuotaReservationPermit` staging table) and chunks created within the safety grace period (`gc_grace_window_ms`, default 300s).
     - **Mark & Sweep with Epoch Locking**: Mark all reachable chunks without blocking concurrent reads or new writes; sweep unreferenced expired chunks and reclaim bytes in `StrictQuotaEnforcer`.
     - **Bit-Rot Scrubbing**: During idle AC-powered maintenance windows, verify on-disk chunk BLAKE3 hashes and trigger self-healing fetch from peers if local flash bit-rot is detected.
4. **UI Integration**:
   - Flutter & Svelte 5: "Available Offline" pin toggle on every file/model/artifact with hydration progress spinner, Pin Manager list showing lease expirations, and GC Dashboard showing reclaimable space and last scrub status.

## 4. Acceptance Criteria
- >= 95% line/branch coverage in `pinning.rs` and `gc.rs`; 100% test pass rate.
- **100% Offline Read Guarantee**: Once a pinned object reaches `HydrationState::FullyAvailableOffline`, disabling all network interfaces (`OfflineMode`) still allows reading the complete object with 0 errors and < 5 ms first-byte latency.
- **Zero Premature Deletion**: Concurrent fuzz/stress testing of `pin` + `unpin` + `ingest` + `run_gc_cycle` across 16 threads never deletes a chunk reachable from any active root or in-flight write.
- **Scan Resistance**: Streaming a one-time unpinned 100 MiB file through a 32 MiB cache does not evict hot chunks accessed >= 3 times.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_pin_recursive_reference_counting_shared_dedup_chunks`
  - `test_w_tinylfu_admission_filter_protects_hot_chunks_against_one_hit_wonders`
  - `test_lease_with_ttl_expiration_makes_chunks_eligible_for_gc`
  - `test_gc_grace_window_protects_in_flight_uncommitted_chunks`
- **`[INT]`**:
  - `test_pin_hydrate_from_erasure_shards_disconnect_network_and_read_offline`
  - `test_concurrent_ingest_pin_unpin_and_gc_sweep_zero_dangling_refs`
  - `test_idle_bitrot_scrubber_detects_corrupted_local_chunk_and_repairs_from_peer`
- **`[SIM]`**:
  - Simulate 1,000 object creations, partial overlapping deduplicated chunks, TTL lease expirations, and 50 GC cycles verifying exact byte accounting parity (`used_bytes == sum(live_chunk_files)`).
- **`[HW]`**:
  - Measure GC mark-and-sweep pause/latency overhead on a store containing 10,000 chunks (< 50 ms target).
- **`[MANUAL]`**:
  - Pin a file/model in the Flutter mobile UI, enable Airplane Mode / disconnect network, open the file/model, and verify instant offline access.

## 6. Required Evidence & Artifacts
- `artifacts/stage-07/p1/sp3_offline_pin_hydration_proof.json`
- `artifacts/stage-07/p1/sp3_concurrent_gc_safety_and_accounting.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document Pinning state machine, W-TinyLFU cache eviction, and epoch-safe Mark-and-Sweep GC in `docs/architecture/DISTRIBUTED_STORAGE.md` (Section 3).
- Mark `S7.P1.SP3` and Phase `S7.P1` complete in `TODO.md`; update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If GC ever deletes a chunk belonging to a pinned object, registered GGUF model, or active job, or if `used_bytes` drifts from actual on-disk file sizes after GC.
- **Completion Gate**: Phase `S7.P1` completion gate passed (`S7_P1_SP1`..`SP3` green, >= 95% coverage). Proceed to Phase `S7.P2`.
