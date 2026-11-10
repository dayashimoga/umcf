# Subphase S7.P1.SP1 Execution Prompt: BLAKE3 Content-Addressed Chunk Store & Explicit Quota Enforcement

## 1. Role & Context
You are the Storage Engine & Resource Isolation Engineer for UMCF. In Subphase `S7.P1.SP1`, you will implement `ContentAddressedChunkStore` and `StrictQuotaEnforcer` in `crates/umcf-storage/src/chunk_store.rs` and `src/quota.rs`. Because UMCF runs on personal mobile phones and laptops where user storage is precious, the storage engine must provide high-speed content-defined deduplication while enforcing an ironclad, byte-exact storage ceiling that never starves the host OS.

## 2. Prerequisites & Dependencies
- `crates/umcf-crypto` (BLAKE3 hashing).
- `crates/umcf-hw` (filesystem statvfs / available disk telemetry).

## 3. Implementation Tasks
1. **Gear-Hash FastCDC & BLAKE3 Merkle DAG (`crates/umcf-storage/src/chunk_store.rs`)**:
   - Implement deterministic Gear-based Content-Defined Chunking (`FastCdcChunker`) with configurable parameters (`min_size = 262_144` (256 KiB), `avg_size = 1_048_576` (1 MiB), `max_size = 4_194_304` (4 MiB)) plus fixed-size chunking mode for pre-aligned assets.
   - Compute `ChunkId([u8; 32])` (BLAKE3 hash of chunk bytes) and assemble hierarchical `ObjectDagManifest`:
     ```rust
     pub struct ObjectDagManifest {
         pub root_cid: ObjectCid, // BLAKE3 hash of canonical CBOR/bincode manifest
         pub total_size_bytes: u64,
         pub chunk_refs: Vec<ChunkRef>, // { chunk_id, offset, length }
         pub content_type: String,
         pub metadata: BTreeMap<String, String>,
         pub created_at_ms: u64,
     }
     ```
   - Store verified chunks on disk in a 2-level fanout directory (`chunks/{hex[0..2]}/{hex[2..4]}/{full_hex}.chk`) using atomic write-to-staging + `fsync` + rename, paired with an embedded `redb`/`sqlite` index tracking reference counts, last-access timestamps, and byte sizes.
2. **Byte-Exact Atomic Quota Enforcer (`crates/umcf-storage/src/quota.rs`)**:
   - Implement `StrictQuotaEnforcer`:
     - Tracks `used_bytes: AtomicU64`, `reserved_in_flight_bytes: AtomicU64`, `configured_max_bytes: AtomicU64`, and `host_free_floor_bytes: u64` (default 15% of total partition size or 2 GiB on mobile).
     - Effective ceiling:
       $$\text{EffectiveLimit} = \min\left(\text{configured\_max\_bytes},\ \text{used\_bytes} + \max(0, \text{host\_free\_bytes} - \text{host\_free\_floor\_bytes})\right)$$
     - Before any chunk or staging file is written to disk, the caller must acquire an RAII `QuotaReservationPermit` via lock-free compare-and-swap (`try_reserve(bytes) -> Result<QuotaReservationPermit, QuotaExceededError>`).
     - If `used_bytes + reserved_in_flight_bytes + bytes > EffectiveLimit`, `try_reserve` first triggers synchronous eviction of unpinned opportunistic cache entries (`evict_unpinned_up_to(bytes)`); if still insufficient, it immediately rejects the write with `QuotaExceededError` before 1 byte touches the filesystem.
     - Dropping an uncommitted `QuotaReservationPermit` (e.g., on network abort or hash mismatch) atomically decrements `reserved_in_flight_bytes` and deletes the staging file.
3. **UI Integration**:
   - Flutter & Svelte 5 Storage Quota Card: Interactive slider to set max contributed storage (e.g., `512 MB` to `500 GB`), stacked progress bar showing `Pinned Data`, `Model Shards`, `Cached Chunks`, `In-Flight Reserved`, and `OS Safety Floor`, plus a "One-Click Purge Cache" button.

## 4. Acceptance Criteria
- >= 95% line/branch coverage in `chunk_store.rs` and `quota.rs`; 100% test pass rate.
- **Deduplication Efficiency**: Inserting a 50 MiB file and then inserting a modified version with 100 bytes inserted near the beginning reuses >= 94% of existing FastCDC chunks without duplicating disk usage.
- **Hard Quota Invariant**: Under a 64-thread concurrent write storm targeting a 16 MiB quota limit, total bytes on disk (`used_bytes + staging_files_bytes`) never exceeds `16_777_216` bytes at any microsecond.
- **Host OS Floor Protection**: When simulated host free disk space drops below the 15% safety floor, new remote cache writes are immediately blocked even if `used_bytes < configured_max_bytes`.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_fastcdc_chunking_shift_resilience_on_mid_file_insertion`
  - `test_atomic_quota_reservation_cas_loop_64_threads_zero_overrun`
  - `test_uncommitted_quota_permit_drop_releases_in_flight_bytes`
  - `test_host_os_free_space_floor_clamps_effective_quota`
- **`[INT]`**:
  - `test_ingest_deduplicate_and_verify_readback_corrupted_chunk_rejected`
  - `test_quota_evicts_unpinned_cache_to_admit_new_pinned_write`
- **`[SIM]`**:
  - Crash recovery simulation leaving 20 orphan `.staging` files in the store directory; verify startup recovery cleans staging files and reconciles `used_bytes` against the index in < 50 ms.
- **`[HW]`**:
  - Benchmark FastCDC + BLAKE3 ingestion speed (MB/s) on local storage.
- **`[MANUAL]`**:
  - Lower the Storage Quota slider in Flutter/Svelte below current cache usage and verify immediate eviction down to the new cap.

## 6. Required Evidence & Artifacts
- `artifacts/stage-07/p1/sp1_fastcdc_dedup_ratio.json`
- `artifacts/stage-07/p1/sp1_quota_64thread_invariant_audit.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document FastCDC parameters and `QuotaReservationPermit` RAII state machine in `docs/architecture/DISTRIBUTED_STORAGE.md` (Section 1).
- Mark `S7.P1.SP1` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If any race condition allows concurrent streams to write past `EffectiveLimit`, or if corrupted chunks can be read back without BLAKE3 verification.
- **Completion Gate**: All tests green with >= 95% coverage before proceeding to `S7.P1.SP2`.
