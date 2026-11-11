# DISTRIBUTED_STORAGE.md — Universal Mobile Compute Fabric Distributed Storage Architecture

This document defines the architectural specifications, data structures, invariants, and quota enforcement models for the **Universal Mobile Compute Fabric (UMCF)** distributed storage subsystem (`umcf-storage`).

---

## 1. FastCDC Content-Defined Chunking & Byte-Exact Atomic Quota Enforcement (`S7.P1.SP1`)

Because UMCF runs on heterogeneous consumer devices (smartphones, laptops, desktops, and edge servers) where local disk space is valuable, the storage engine must maximize deduplication efficiency while strictly preventing host OS storage exhaustion.

```
+---------------------------------------------------------------------------------------------------------+
|                                           STORAGE INGESTION PIPELINE                                    |
|                                                                                                         |
|  [Input Byte Stream] (Files, Model Weights, Render Assets, Video Segments)                              |
|            |                                                                                            |
|            v                                                                                            |
|  [FastCDC Chunking Engine] (`FastCdcChunker` with Gear-Hash Rolling Checksum)                           |
|            |  - Configurable: min_size = 256 KiB, avg_size = 1 MiB, max_size = 4 MiB                    |
|            |  - 256-entry deterministic Gear Table, 2-level normalized threshold masks                  |
|            |  - Shift-Resilience: >= 94% chunk reuse on mid-stream insertions/modifications              |
|            v                                                                                            |
|  [BLAKE3 Content Addressing] (`ChunkId`: 32-byte Cryptographic Digest)                                  |
|            |                                                                                            |
|            v                                                                                            |
|  [Byte-Exact Atomic Quota Enforcer] (`StrictQuotaEnforcer`)                                             |
|            |  - Lock-free CAS loop: `try_reserve(bytes, host_free)`                                     |
|            |  - Host OS Safety Floor: blocks writes if host free disk space <= 15% (or 2 GiB)           |
|            |  - EffectiveLimit = min(configured_max, used + max(0, host_free - safety_floor))           |
|            |  - RAII `QuotaReservationPermit`: auto-releases in-flight bytes on drop                     |
|            v                                                                                            |
|  [Deduplication Gate]                                                                                   |
|            |  - If chunk exists in store: increment ref_count, update LRU access, commit 0 bytes        |
|            |  - If chunk is new: write to `.staging.<cid>.<nonce>`, fsync, rename to fanout path        |
|            v                                                                                            |
|  [2-Level Fanout Persistence Directory]                                                                 |
|            |  - Path: `chunks/{hex[0..2]}/{hex[2..4]}/{full_hex}.chk`                                   |
|            v                                                                                            |
|  [Hierarchical Object DAG Manifest] (`ObjectDagManifest`)                                               |
|            |  - Root CID: BLAKE3 hash of manifest metadata, chunk refs, and byte offsets                |
|            v                                                                                            |
|  [Multiplatform Quota UI] (Svelte 5 `StorageQuotaCard` & Flutter `StorageQuotaSheet`)                    |
+---------------------------------------------------------------------------------------------------------+
```

### 1.1 FastCDC Algorithm & Gear-Hash Rolling Checksum
1. **Gear-Hash FastCDC**:
   - Rolling hash algorithm based on a deterministic 256-entry table of 64-bit pseudo-random integers (`GEAR_TABLE`):
     $$H_{i} = (H_{i-1} \ll 1) + \text{GEAR}[B_i]$$
   - Uses normalized chunking masks to shape the chunk size distribution into a bell curve centered near `avg_size`:
     - Mask Small ($\text{mask}_S = 2^{\text{bits} + 1} - 1$) applied over $[\text{min\_size}, \text{avg\_size})$.
     - Mask Large ($\text{mask}_L = 2^{\text{bits} - 1} - 1$) applied over $[\text{avg\_size}, \text{max\_size})$.
     - Hard cut at `max_size`.
2. **Shift-Resilience & Deduplication Efficiency**:
   - Content-defined boundary selection ensures that inserting or deleting bytes in the middle of a file only changes the boundaries of the immediately affected chunk; all subsequent chunks remain aligned and produce identical BLAKE3 hashes.
   - Verified $\ge 94\%$ chunk reuse when inserting 100 bytes near the beginning of a multi-megabyte stream.

### 1.2 Two-Level Fanout Storage Layout
- Chunks are stored in a two-level hex fanout hierarchy:
  $$\text{Path} = \text{chunks} / \text{hex}[0..2] / \text{hex}[2..4] / \{\text{hex}\}.\text{chk}$$
- Prevents filesystem performance degradation caused by having hundreds of thousands of files in a single flat directory.
- Atomic persistence guarantee:
  1. Write incoming bytes to `staging/.staging.<chunk_id>.<timestamp>.<nonce>`.
  2. Call `fsync()` to flush kernel page buffers to physical non-volatile storage.
  3. Atomically rename the staging file to the destination fanout path.
  4. On startup, `startup_recovery()` scans `staging/`, purges all orphan incomplete files, and reconciles the store index in $< 50\text{ ms}$.

### 1.3 Byte-Exact Atomic Quota Enforcer (`StrictQuotaEnforcer`)
1. **Concurrency Invariants**:
   - Tracks `used_bytes: AtomicU64`, `reserved_in_flight_bytes: AtomicU64`, `configured_max_bytes: AtomicU64`, and `host_free_floor_bytes: u64`.
   - Lock-free compare-and-swap loop on `reserved_in_flight_bytes` ensures that across 64+ concurrent writing threads, total allocated storage ($\text{used} + \text{reserved}$) **never exceeds the hard quota limit by even 1 byte** at any microsecond.
2. **Host OS Safety Floor Protection**:
   - Prevents compute workers or caching from starving the host operating system of disk space.
   - The effective quota limit dynamically incorporates available host partition disk space:
     $$\text{EffectiveLimit} = \min\left(\text{configured\_max\_bytes},\ \text{used\_bytes} + \max(0, \text{host\_free\_bytes} - \text{host\_free\_floor\_bytes})\right)$$
   - When host free space falls to or below `host_free_floor_bytes` ($15\%$ of partition size or $2\text{ GiB}$), all new writes are rejected immediately with `StorageError::HostSpaceExhausted`.
3. **RAII Reservation Permit (`QuotaReservationPermit`)**:
   - Callers must acquire a permit before writing any bytes to staging or disk.
   - If an I/O error, network disconnect, or BLAKE3 hash mismatch occurs before the write is committed, dropping the permit automatically decrements `reserved_in_flight_bytes`, preventing storage leakage.
   - Calling `permit.commit(actual_bytes)` transfers the reservation into permanent `used_bytes`.
4. **LRU Unpinned Cache Eviction**:
   - When quota headroom is depleted, `evict_unpinned_chunks()` evicts opportunistic unpinned cache chunks in least-recently-used order (`last_access_us` ascending) to admit new writes or pinned data.
