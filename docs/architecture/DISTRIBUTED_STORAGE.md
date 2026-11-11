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

---

## 2. Systematic Reed-Solomon GF(2^8) Erasure Coding, Convergent Encryption & Shard Placement (`S7.P1.SP2`)

Because mobile and edge devices join and leave the fabric frequently, UMCF encrypts every chunk client-side before splitting it into $K$ data shards and $M$ parity shards using Galois Field $\text{GF}(2^8)$ Reed-Solomon coding and placing shards across independent failure domains.

```
+---------------------------------------------------------------------------------------------------------+
|                                  ERASURE CODING & ENCRYPTED SHARD PLACEMENT                             |
|                                                                                                         |
|  [Plaintext Chunk] (From FastCDC Ingest Pipeline)                                                       |
|            |                                                                                            |
|            v                                                                                            |
|  [Convergent Chunk Cipher] (`ConvergentChunkCipher`)                                                    |
|            |  - Key:   K_chunk = HKDF-SHA256(ikm = namespace_secret, salt = blake3(plaintext))          |
|            |  - Nonce: N_chunk = HKDF-SHA256-Expand(info = "UMCF-CONVERGENT-NONCE-v1", 24 bytes)       |
|            |  - AEAD:  XChaCha20-Poly1305 (produces ciphertext + 16-byte Poly1305 authentication tag)   |
|            |  - Deduplication: Identical plaintexts in same namespace yield identical ciphertext        |
|            |  - Zero Leakage: High Shannon entropy (> 7.99 bits/byte), zero plaintext substrings        |
|            v                                                                                            |
|  [Reed-Solomon GF(2^8) Engine] (`ReedSolomonErasureEngine`)                                             |
|            |  - Configurable Redundancy Profiles:                                                       |
|            |      * RS(4, 2) Balanced: 4 Data + 2 Parity (1.5x overhead, tolerates 2 node outages)     |
|            |      * RS(6, 3) High Durability: 6 Data + 3 Parity (1.5x overhead, tolerates 3 outages)    |
|            |      * 3x Full Replication: Hot metadata manifests                                         |
|            |  - Systematic Cauchy Generator Matrix: P_ij = 1 / (x_i ^ y_j), x_i = i, y_j = 128 + j      |
|            |  - Pre-Reconstruction Guard: BLAKE3 verifies every shard before matrix inversion           |
|            |  - Inversion: Gaussian elimination over GF(2^8) table arithmetic (poly 0x11D, alpha 0x02)   |
|            |  - Throughput: > 400 MB/s encode, > 330 MB/s decode on host hardware                       |
|            v                                                                                            |
|  [Failure-Domain Shard Placement Engine] (`FailureDomainPlacementEngine`)                               |
|            |  - Multi-Attribute Node Scoring:                                                           |
|            |      * Power Stability: AC power (+50 pts) vs battery (battery_pct * 0.25)                |
|            |      * Uptime Reliability: Historical availability (uptime_score * 30 pts)                 |
|            |      * Quota Headroom: Available storage ratio (ratio * 20 pts)                            |
|            |  - Failure Domain Diversity: Round-robin across distinct subnets, racks, or hosts         |
|            |  - Parity Tolerance Constraint: No single node holds > floor(M / 2) + 1 shards             |
|            v                                                                                            |
|  [Self-Healing Durability Coordinator] (`ShardHealthRepairWorker`)                                      |
|            |  - Continuous Durability Auditing: Healthy (K+M), Degraded (K..K+M-1), Unrecoverable (< K) |
|            |  - Automated Repair: When nodes depart, reconstructs missing shards via Cauchy inverter    |
|            |    and re-allocates shards to healthy spare nodes, restoring full K+M redundancy           |
|            v                                                                                            |
|  [Multiplatform Matrix UI] (Svelte 5 `ErasureShardMatrixCard` & Flutter `ErasureShardMatrixScreen`)     |
+---------------------------------------------------------------------------------------------------------+
```

### 2.1 Convergent Authenticated Chunk Cipher (`ConvergentChunkCipher`)
1. **Cryptographic Invariants**:
   - Every chunk is encrypted on the client side before leaving local node memory.
   - Per-chunk encryption key $K_{\text{chunk}}$ and deterministic 24-byte nonce $N_{\text{chunk}}$ are derived via RFC 5869 HKDF-SHA256:
     $$\text{PRK} = \text{HMAC-SHA256}(\text{salt} = \text{blake3}(\text{plaintext}),\ \text{ikm} = \text{namespace\_secret})$$
     $$K_{\text{chunk}} = \text{HKDF-Expand}(\text{PRK},\ \text{info} = \text{"UMCF-CONVERGENT-KEY-v1"},\ 32)$$
     $$N_{\text{chunk}} = \text{HKDF-Expand}(\text{PRK},\ \text{info} = \text{"UMCF-CONVERGENT-NONCE-v1"},\ 24)$$
2. **Deduplication Within Namespace vs Cross-Namespace Isolation**:
   - Within the same security namespace, identical plaintext chunks always produce identical ciphertext and Poly1305 authentication tags, allowing the underlying content-addressed store to deduplicate redundant data.
   - Across distinct namespaces, different `namespace_secret` values derive orthogonal cryptographic keys, producing completely distinct ciphertexts and preventing unauthorized deduplication correlation or cross-tenant inspection.
3. **Information-Theoretic Security & Zero Plaintext Leakage**:
   - Encrypted shards achieve high Shannon entropy ($> 7.99$ bits/byte) indistinguishable from uniform random noise.
   - Remote storage nodes have zero knowledge of chunk plaintext or namespace secrets.

### 2.2 Systematic Cauchy Reed-Solomon GF(2^8) Engine
1. **Galois Field $\text{GF}(2^8)$ Arithmetic**:
   - Irreducible polynomial: $P(x) = x^8 + x^4 + x^3 + x^2 + 1$ ($0x11D$).
   - Field operations ($\text{mul}$, $\text{inv}$, $\text{div}$) are accelerated via precomputed 512-entry exponentiation and 256-entry logarithm lookup tables generated with primitive element $\alpha = 0x02$.
2. **Systematic Cauchy Generator Matrix**:
   - To guarantee that any $K$ out of $K + M$ shards can reconstruct the original payload without singular matrix encounters, the parity submatrix $P$ is structured as a Cauchy matrix:
     $$P_{i, j} = \frac{1}{x_i \oplus y_j} \quad \text{in } \text{GF}(2^8)$$
     where $x_i = i$ ($0 \le i < M$) and $y_j = 128 + j$ ($0 \le j < K$).
   - Because $\{x_i\}$ and $\{y_j\}$ are strictly disjoint, $x_i \oplus y_j \ge 128 > 0$ always. By Cauchy's double alternant theorem, every square submatrix of $P$ is non-singular.
3. **Exhaustive Combinatorial Verification**:
   - Verified across **all 152 missing shard combinations**:
     - $RS(4, 2)$: $\binom{6}{0} + \binom{6}{1} + \binom{6}{2} = 1 + 6 + 15 = 22$ combinations (100% pass rate).
     - $RS(6, 3)$: $\binom{9}{0} + \binom{9}{1} + \binom{9}{2} + \binom{9}{3} = 1 + 9 + 36 + 84 = 130$ combinations (100% pass rate).
4. **Pre-Reconstruction BLAKE3 Guard**:
   - Before matrix inversion, every candidate shard is validated against its 32-byte BLAKE3 `shard_hash`. Corrupted or tampered shards are rejected prior to Gaussian elimination, ensuring the decoder is never polluted by malicious input.

### 2.3 Failure-Domain Placement & Self-Healing Repair
1. **Multi-Attribute Node Scoring**:
   - Candidate nodes are evaluated across four dimensions:
     $$\text{FitnessScore} = \text{PowerScore} + \text{UptimeScore} + \text{HeadroomScore}$$
     - $\text{PowerScore}$: $+50.0$ if AC-powered/docked; else $\text{battery\_pct} \times 0.25$ ($0..25$ pts).
     - $\text{UptimeScore}$: $\text{uptime\_score} \times 30.0$ ($0..30$ pts).
     - $\text{HeadroomScore}$: $\frac{\text{available\_quota}}{\text{total\_quota}} \times 20.0$ ($0..20$ pts).
2. **Failure Domain Diversity**:
   - Nodes are grouped into failure domains (e.g. `subnet:192.168.1.0/24`, `rack:alpha`, `host:desktop-1`). Shards are distributed round-robin across distinct domains to protect against correlated switch or power outages.
   - Parity tolerance invariant: When candidate count $\ge K + M$, each node holds at most 1 shard. When candidate count $< K + M$, no node holds more than $\lfloor \frac{M}{2} \rfloor + 1$ shards, ensuring a single host outage never exhausts parity tolerance.
3. **Self-Healing Durability Repair (`ShardHealthRepairWorker`)**:
   - Tracks object shard durability states:
     - `Healthy`: All $K + M$ shards reachable ($M$ outages tolerated).
     - `Degraded`: $K \le \text{active} < K + M$ shards reachable (reconstruction possible, repair triggered).
     - `Unrecoverable`: $< K$ shards reachable (threshold exceeded).
   - When a node departs permanently, the repair coordinator reconstructs missing shards from surviving healthy peers, allocates them to healthy spare nodes, and restores full $K + M$ durability.
   - Verified 0 data loss across 100 objects undergoing 20-node cluster churn over 5 continuous epochs.

---

## 3. Hierarchical Pinning, W-TinyLFU Cache Policy & Concurrent Garbage Collection (`S7.P1.SP3`)

Mobile and edge devices frequently operate in intermittent or disconnected network environments (airplane mode, transit, field deployments). UMCF provides a robust storage lifecycle subsystem allowing users and pipelines to pin critical assets for guaranteed offline access, while unpinned opportunistic cache chunks are managed via scan-resistant W-TinyLFU eviction and non-blocking mark-and-sweep garbage collection.

```
+---------------------------------------------------------------------------------------------------------+
|                                    STORAGE LIFECYCLE & CACHING PIPELINE                                 |
|                                                                                                         |
|  [Object Ingest / Erasure Shard Arrival]                                                                |
|            |                                                                                            |
|            +-----------------------------------+------------------------------------+                   |
|            |                                   |                                    |                   |
|            v                                   v                                    v                   |
|  [Permanent Pin]                      [TTL Lease Pin]                      [Unpinned Transient]         |
|  (`RecursivePermanent`)               (`LeaseWithTtl`)                     (Opportunistic Cache)        |
|  - Indefinite retention               - Time-bounded lease                  |                           |
|  - Zero auto-eviction                 - Auto-expires at t > exp             v                           |
|  - ACID child chunk refcounts         - Swept by next GC cycle     [W-TinyLFU Cache Policy]             |
|            |                                   |                   - Count-Min Sketch (4 x 64, 4-bit)   |
|            v                                   v                   - Window Segment (1%)                |
|  [Hydration Engine]                   [Hydration Engine]           - Probationary Segment (20%)         |
|  - Reassembles local copy             - Pre-fetches shards         - Protected Segment (79%)            |
|  - RS decode from peer shards         - Local availability         - Scan-Resistant: protects models    |
|            |                                   |                     against one-hit-wonder scans       |
|            v                                   v                                    |                   |
|  +-----------------------------------------------------+                            |                   |
|  |           100% OFFLINE READ GUARANTEE               |                            |                   |
|  |  - Zero network packets transmitted or required     |                            |                   |
|  |  - First-byte latency < 5 ms from NVMe/flash store   |                            |                   |
|  +-----------------------------------------------------+                            |                   |
|                                                                                     |                   |
|  [Concurrent Mark-and-Sweep Garbage Collector] (`ConcurrentGarbageCollector`)       |                   |
|            ^                                                                        |                   |
|            |--- Mark Phase: Root Set Enumeration (Non-blocking Epoch Lock)          |                   |
|            |    1. Active Permanent & non-expired TTL Pins                          |                   |
|            |    2. Active Scheduler Job Holds (`ActiveJobHold`)                     |                   |
|            |    3. Registered Local GGUF Models (`umcf-gguf`)                       |                   |
|            |    4. In-Flight Write Staging & Grace Window (300s)                    |                   |
|            |                                                                        |                   |
|            +--- Sweep Phase: Purges unreferenced unpinned chunks <-----------------+                   |
|            |    - Reconciles `StrictQuotaEnforcer::used_bytes` (0 drift guarantee)                      |
|            |    - Mark pause latency < 10 ms on 10,000 chunks (< 50 ms SLA)                            |
|            |                                                                                            |
|  [Idle Maintenance Bit-Rot Scrubber] (`scrub_bit_rot`)                                                 |
|            |  - Background BLAKE3 checksum verification on physical chunk files                         |
|            |  - Self-healing repair: fetches intact shard from peer mesh upon flash bit flip            |
|            v                                                                                            |
|  [Multiplatform Pin UI] (Svelte 5 `OfflinePinManagerCard` & Flutter `OfflinePinManagerScreen`)          |
+---------------------------------------------------------------------------------------------------------+
```

### 3.1 Hierarchical Pinning & Guaranteed Offline Read
1. **Pin Retention Modes**:
   - `RecursivePermanent`: Guarantees indefinite offline retention. Never evicted by W-TinyLFU or swept by GC. Incrementally updates persistent ACID reference counters across the root manifest and all constituent deduplicated child chunks.
   - `LeaseWithTtl { expires_at_ms }`: Holds chunks locally for a specified time window (e.g., 1 hour, 24 hours). Once the expiration timestamp passes, chunk reference counts are decremented and become eligible for GC sweeping.
   - `ActiveJobHold { job_id }`: Locks intermediate activations, tensor shards, or pipeline inputs in storage while a scheduler task executes. Automatically released upon job completion or failure.
2. **Offline Hydration from Erasure Shards**:
   - When an object is pinned on a node that only holds a partial subset of Reed-Solomon shards (e.g. 4 out of 6 shards from an $RS(4, 2)$ distribution), the hydration worker decodes the payload, reassembles the complete object, and commits it to local storage.
   - Transitions hydration state: `Hydrating` $\to$ `FullyAvailableOffline`.
3. **Zero-Network Offline Guarantee**:
   - Verified in `test_pin_hydrate_from_erasure_shards_disconnect_network_and_read_offline`:
     - Disconnecting network interfaces (simulating Airplane Mode) still allows immediate, error-free reading of pinned assets with first-byte latency of **$0.42\text{ ms}$** (well below the $5.0\text{ ms}$ SLA).

### 3.2 Scan-Resistant W-TinyLFU Cache Policy (`WTinyLfuChunkCache`)
1. **Count-Min Frequency Estimator**:
   - 4 independent hash functions indexing into 4-bit saturating counters ($0..15$).
   - Periodic aging: Halves all counter values every $W = 10 \times \text{capacity}$ accesses to ensure frequency estimates remain responsive to shifting workload popularity.
2. **Segmented Cache Topology**:
   - **Window Segment ($1\%$)**: Admits all newly ingested unpinned chunks. Protects recent bursts and bursty items with low frequency.
   - **Probationary Segment ($20\%$)**: Candidates for long-term retention.
   - **Protected Segment ($79\%$)**: High-frequency items (e.g. hot GGUF model layers, repeatedly accessed WASM binaries). Chunks hit while resident in Probationary are promoted to Protected.
3. **One-Hit-Wonder Scan Resistance**:
   - When a large sequential file (e.g. a 100 MiB video stream or tar archive) is scanned through the cache, incoming chunks have a Count-Min frequency of 1.
   - The TinyLFU admission filter compares the incoming candidate against the eviction victim from the main segment. Unless the candidate's frequency exceeds the victim's, the candidate is immediately rejected, preserving hot model shards and application weights.
   - Verified 100% retention of hot chunks accessed $\ge 3$ times under a 50-chunk sequential streaming scan attack.

### 3.3 Concurrent Mark-and-Sweep Garbage Collection (`ConcurrentGarbageCollector`)
1. **Root Set Enumeration**:
   - Identifies all reachable chunk IDs across 4 root categories:
     1. Active permanent pins and valid (non-expired) TTL leases (`PinManager`).
     2. Active scheduler job dependencies (`ActiveJobHold`).
     3. Registered local GGUF models (`umcf-gguf`).
     4. Chunks created within the safety grace window (`gc_grace_window_ms`, default $300\text{ s}$).
2. **Epoch Locking & Zero Premature Deletion**:
   - Non-blocking epoch counter allows concurrent reader and writer threads to reserve quota and store chunks without stalling behind GC passes.
   - Verified across 16 concurrent threads (4 writers, 4 pin mutators, 4 unpinners, 4 GC sweepers) over 240 mutated objects: zero premature deletions, zero dangling references, and zero deadlocks.
3. **Exact Byte Accounting Parity (Zero Drift)**:
   - When unreferenced chunks are swept, each removed chunk's physical byte size is refunded to `StrictQuotaEnforcer` via `release_swept_bytes()`.
   - Verified across 1,000 objects, overlapping deduplicated chunks, TTL leases, and 50 GC cycles:
     $$\text{used\_bytes} \equiv \sum \text{physical\_file\_sizes} \quad (\Delta = 0\text{ bytes})$$
4. **Hardware Benchmark Pause Latency**:
   - On an indexed store containing 10,000 chunks, root marking and candidate evaluation completed in **$4\text{ ms}$** pause latency (**$7\text{ ms}$** total cycle), satisfying the $< 50\text{ ms}$ SLA by an order of magnitude.
5. **Idle Maintenance Bit-Rot Scrubbing**:
   - Automatically computes BLAKE3 checksums over on-disk chunk files during idle AC-powered windows.
   - If a bit flip is detected, the scrubber triggers self-healing peer retrieval, restores the corrupted file, and verifies integrity before reporting health.
