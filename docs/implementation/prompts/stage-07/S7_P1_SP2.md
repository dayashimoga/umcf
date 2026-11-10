# Subphase S7.P1.SP2 Execution Prompt: Reed-Solomon Erasure Coding, Replication & Encrypted Shard Placement

## 1. Role & Context
You are the Distributed Erasure Coding & Cryptographic Storage Engineer for UMCF. In Subphase `S7.P1.SP2`, you will implement `ConvergentChunkCipher`, `ReedSolomonErasureEngine`, and `FailureDomainPlacementEngine` in `crates/umcf-storage/src/erasure.rs` and `src/placement.rs`. Because mobile and laptop nodes join and leave the fabric frequently, UMCF encrypts every chunk client-side before splitting it into $k$ data shards and $m$ parity shards via Galois Field $\text{GF}(2^8)$ Reed-Solomon coding and placing shards across independent failure domains.

## 2. Prerequisites & Dependencies
- `S7.P1.SP1` (`ContentAddressedChunkStore`, `ObjectDagManifest`).
- `crates/umcf-crypto` (`XChaCha20Poly1305`, `HkdfSha256`, `Blake3`).
- `crates/umcf-net` (DHT / cluster topology and QUIC shard transfer).

## 3. Implementation Tasks
1. **Convergent Authenticated Encryption & Reed-Solomon Engine (`crates/umcf-storage/src/erasure.rs`)**:
   - **Client-Side Convergent Encryption (`ConvergentChunkCipher`)**:
     - Derive per-chunk encryption key $K_{\text{chunk}} = \text{HKDF-SHA256}(\text{ikm} = \text{namespace\_secret},\ \text{salt} = \text{plaintext\_blake3})$ and deterministic 24-byte nonce so identical chunks within the same security namespace deduplicate while remaining indistinguishable from random noise to storage nodes without `namespace_secret`.
     - Encrypt chunk with `XChaCha20Poly1305` producing ciphertext + 16-byte Poly1305 tag.
   - **Reed-Solomon $\text{GF}(2^8)$ SIMD Codec (`ReedSolomonErasureEngine`)**:
     - Support configurable redundancy profiles:
       - `RedundancyProfile::ErasureCoding { data_shards_k: u8, parity_shards_m: u8 }` (default $RS(4, 2)$ for 1.5x overhead tolerating 2 simultaneous node failures; $RS(6, 3)$ for larger clusters; adaptive $RS(2, 1)$ for 3-node clusters).
       - `RedundancyProfile::FullReplication { replica_count: u8 }` for small hot metadata manifests.
     - Pad ciphertext to a multiple of `data_shards_k`, compute `parity_shards_m` parity slices, and compute an individual `ShardHash([u8; 32])` (BLAKE3) for each of the $k + m$ shards.
2. **Failure-Domain-Aware Shard Placement & Self-Healing (`crates/umcf-storage/src/placement.rs`)**:
   - Implement `FailureDomainPlacementEngine::select_target_nodes(total_shards, cluster_nodes) -> Vec<NodeId>`:
     - Score candidate storage nodes by: (a) distinct physical host/subnet failure domain, (b) power state (prefer AC-powered desktops/servers/plugged-in phones over unplugged mobile devices), (c) historical uptime reliability score, and (d) available storage quota headroom.
     - Guarantee that no single node holds more than $\lfloor \frac{m}{2} \rfloor + 1$ shards when cluster size $\ge k + m$, ensuring single-node failure never exceeds parity tolerance.
   - Implement `ShardHealthRepairWorker`:
     - Periodically audits shard availability via lightweight DHT/gossip bitfield bloom filters; when available shards for an object drop to $k + 1$ (due to permanent node departure), reconstructs the missing shard from any $k$ healthy peers and places it on a new healthy node.
3. **UI Integration**:
   - Flutter & Svelte 5: Redundancy Profile selector (`RS(4,2) Balanced`, `RS(6,3) High Durability`, `3x Replicated`) and visual Shard Placement Matrix showing which device holds shards `D0..D3, P0..P1` and live durability health (`6/6 Shards Healthy — Tolerates 2 Node Outages`).

## 4. Acceptance Criteria
- >= 95% line/branch coverage in `erasure.rs` and `placement.rs`; 100% test pass rate.
- **Erasure Reconstruction Guarantee**: For any $RS(k, m)$ encoded chunk, deleting any arbitrary subset of up to $m$ data or parity shards (tested across all $\binom{k+m}{m}$ combinations for $RS(4, 2)$ and $RS(6, 3)$) reconstructs the exact original plaintext with 100% BLAKE3 verification.
- **Zero Plaintext Leakage on Storage Nodes**: Inspecting raw `.shard` files stored on remote worker nodes confirms high Shannon entropy ($> 7.99$ bits/byte) and zero occurrence of source plaintext strings.
- **Self-Healing Repair**: Taking 1 node offline in a 7-node $RS(4, 2)$ cluster triggers background repair onto the 7th spare node, restoring `6/6` shard availability within the configured repair window.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_convergent_xchacha20poly1305_encryption_dedup_and_wrong_namespace_isolation`
  - `test_rs_4_2_and_6_3_all_combinations_of_missing_shards_reconstruct_exact`
  - `test_corrupted_shard_detected_by_blake3_before_rs_matrix_inversion`
  - `test_failure_domain_placement_prefers_ac_power_and_distinct_subnets`
- **`[INT]`**:
  - `test_6_node_rs_4_2_upload_kill_2_nodes_download_and_self_heal_to_7th_node`
- **`[SIM]`**:
  - 20-node cluster churn simulation with nodes joining/leaving while verifying 0 data loss across 100 stored objects.
- **`[HW]`**:
  - Benchmark SIMD Reed-Solomon $RS(4, 2)$ encode & decode throughput (MB/s) on x86_64 AVX2 and ARM64 NEON.
- **`[MANUAL]`**:
  - Upload a file in Svelte 5 / Flutter Storage UI, inspect the `D0..D3, P0..P1` node placement badges, disconnect one device, and verify the file still downloads instantaneously with a `Degraded (5/6 Shards) -> Healing -> Healthy (6/6)` status transition.

## 6. Required Evidence & Artifacts
- `artifacts/stage-07/p1/sp2_rs_combinatorial_reconstruction_proof.json`
- `artifacts/stage-07/p1/sp2_self_healing_repair_timeline.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document convergent XChaCha20-Poly1305 key derivation, Reed-Solomon shard framing, and failure-domain placement heuristics in `docs/architecture/DISTRIBUTED_STORAGE.md` (Section 2).
- Mark `S7.P1.SP2` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If corrupted shards are fed into Reed-Solomon decoding without prior BLAKE3 verification (which would corrupt the reconstructed output), or if plaintext chunks are ever transmitted without encryption.
- **Completion Gate**: All tests green with >= 95% coverage before proceeding to `S7.P1.SP3`.
