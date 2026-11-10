# Phase S7.P1 Execution Prompt: Content-Addressed Distributed Storage & Erasure Coding

## 1. Role & Context
You are the Distributed Storage Systems Lead for UMCF. Phase `S7.P1` builds `crates/umcf-storage`, providing a content-addressed, end-to-end encrypted, erasure-coded distributed file and object store across mobile, desktop, and server nodes. Every participating device strictly enforces user-configured disk and flash wear quotas, while files are deduplicated via content-defined chunking (FastCDC + BLAKE3), protected against device churn via Reed-Solomon $RS(k, m)$ erasure coding, and available offline via explicit pinning and generational garbage collection.

This phase consists of three sequential subphases:
1. `S7_P1_SP1.md`: BLAKE3 Content-Addressed Chunk Store & Explicit Quota Enforcement
2. `S7_P1_SP2.md`: Reed-Solomon Erasure Coding, Replication & Encrypted Shard Placement
3. `S7_P1_SP3.md`: Offline Caching, Pinning & Garbage Collection

## 2. Prerequisites & Dependencies
- Stage 1–6 completion (`umcf-crypto`, `umcf-net` QUIC + DHT/gossip, `umcf-hw` disk space & wear telemetry).

## 3. Implementation Tasks
1. **Subphase S7.P1.SP1 (`crates/umcf-storage/src/chunk_store.rs`, `src/quota.rs`)**:
   - Implement gear-hash FastCDC (content-defined chunking, min 256 KiB, avg 1 MiB, max 4 MiB) + BLAKE3 Merkle DAG builder (`ObjectDagManifest`).
   - Implement `StrictQuotaEnforcer` with pre-write atomic byte reservation (`reserve_bytes` -> write -> commit/release) and mobile OS free-space floor protection (auto-shrinking dynamic quota if host OS free disk drops below 15%).
2. **Subphase S7.P1.SP2 (`crates/umcf-storage/src/erasure.rs`, `src/placement.rs`)**:
   - Implement client-side convergent XChaCha20-Poly1305 chunk encryption (keyed via HKDF from content hash + namespace secret) followed by configurable Reed-Solomon $RS(k, m)$ erasure coding (e.g., $RS(4, 2)$ or $RS(6, 3)$).
   - Implement `FailureDomainPlacementEngine` distributing the $k + m$ shards across distinct physical devices, power sources (battery vs AC), and network subnets, with background shard repair when node availability drops.
3. **Subphase S7.P1.SP3 (`crates/umcf-storage/src/pinning.rs`, `src/gc.rs`)**:
   - Implement local offline pinning (`PinMode::RecursivePermanent`, `PinMode::LeaseWithTtl`, `PinMode::OpportunisticCache`), W-TinyLFU cache admission/eviction for unpinned chunks, and concurrent mark-and-sweep garbage collection with bloom-filter reference tracking.

## 4. Acceptance Criteria
- >= 95% line/branch coverage across `crates/umcf-storage`; 100% test pass rate.
- Zero quota overrun under any concurrency pattern: `allocated_bytes + reserved_in_flight_bytes <= effective_quota_bytes` is a hard invariant checked on every write.
- Untrusted storage nodes holding erasure shards cannot read plaintext file contents or filenames (verified by entropy/ciphertext inspection and key separation).
- Any $k$ out of $k + m$ erasure shards reconstruct the exact original file with 100% BLAKE3 root hash verification.
- Garbage collection never deletes a chunk referenced by an active pin, an in-flight job input/output, or a local model in `umcf-gguf`.

## 5. Required Tests
- **`[UNIT]`**: FastCDC boundary shift resistance on 1-byte insertion, CAS quota reservation under contention, $RS(k, m)$ Vandermonde/Cauchy matrix encode/decode, W-TinyLFU eviction ordering, GC mark-and-sweep root reachability.
- **`[INT]`**: 6-node cluster $RS(4, 2)$ encrypted file upload, kill 2 nodes, download and reconstruct file on a 7th node, verify automatic background re-replication of the 2 lost shards onto surviving nodes.
- **`[SIM]`**: 64 concurrent writers hammering a store with 10 MiB quota; mobile node free disk dropping below 15% triggering emergency unpinned cache eviction.
- **`[HW]`**: Measure FastCDC + XChaCha20-Poly1305 + $RS(4, 2)$ throughput (MB/s) on desktop NVMe and mobile UFS storage.
- **`[MANUAL]`**: Configure storage quota slider, upload/pin files, inspect shard distribution map across nodes, and trigger "Run Garbage Collection" in Flutter & Svelte 5 Storage screens.

## 6. Required Evidence & Artifacts
- `artifacts/stage-07/p1/quota_concurrency_stress_proof.json`
- `artifacts/stage-07/p1/erasure_coding_kill2_reconstruction.json`
- `artifacts/stage-07/p1/gc_pinned_safety_audit.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Create `docs/architecture/DISTRIBUTED_STORAGE.md` covering FastCDC, quota invariants, convergent encryption + $RS(k,m)$, and GC.
- Mark `S7.P1.SP1`–`S7.P1.SP3` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: Any possibility of disk quota overrun, data loss of a pinned object during GC, or unencrypted chunk transmission to remote storage peers.
- **Gate**: All 3 subphases (`S7_P1_SP1`..`SP3`) pass with >= 95% coverage before starting Phase `S7.P2`.
