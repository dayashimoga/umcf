# Subphase S5.P1.SP1 Execution Prompt: Content-Addressed Model Store & Resumable P2P Shard Distribution

## 1. Role & Context
You are the Storage & P2P Content Distribution Engineer for UMCF. In Subphase `S5.P1.SP1`, you will implement the content-addressed GGUF model store and resumable multi-peer QUIC shard synchronization engine inside `crates/umcf-gguf`. Models in UMCF are addressed by the BLAKE3 Merkle root of their 4 MiB chunks, enabling deduplicated storage, partial layer caching for pipeline parallelism, and resilient resumable transfers over flaky mobile connections.

## 2. Prerequisites & Dependencies
- `crates/umcf-crypto`: BLAKE3 streaming hasher & Merkle tree builder.
- `crates/umcf-net`: Multi-stream QUIC transport and peer DHT/gossip announcement service.
- SQLite/Redb embedded metadata catalog (`crates/umcf-storage-meta`).

## 3. Implementation Tasks
1. **Create Crate & Core Data Structures (`crates/umcf-gguf/src/store.rs`)**:
   - Define `ModelId([u8; 32])` (BLAKE3 root hash of the complete GGUF file).
   - Define `ChunkHash([u8; 32])` for fixed `4_194_304` byte (4 MiB) chunks.
   - Define `ModelManifest`:
     ```rust
     pub struct ModelManifest {
         pub model_id: ModelId,
         pub filename: String,
         pub total_size_bytes: u64,
         pub chunk_size_bytes: u32,
         pub chunks: Vec<ChunkDescriptor>,
         pub tensor_shard_map: BTreeMap<String, ByteRange>,
         pub created_at_unix_ms: u64,
         pub publisher_signature: Option<Ed25519Signature>,
     }
     ```
   - Implement `ContentAddressedModelStore` with methods:
     - `ingest_local_file(path: &Path) -> Result<ModelManifest, StoreError>`
     - `open_verified_mmap(model_id: &ModelId) -> Result<VerifiedModelMmap, StoreError>`
     - `open_layer_range_reader(model_id: &ModelId, byte_range: ByteRange) -> Result<LayerRangeHandle, StoreError>`
     - `evict_model(model_id: &ModelId, force: bool) -> Result<u64, StoreError>`
2. **Implement Resumable P2P Shard Transfer (`crates/umcf-gguf/src/p2p_sync.rs`)**:
   - Implement `ResumableShardDownloader` tracking chunk completion in a persistent `.part.bitfield` file alongside `.part.data`.
   - Implement work-stealing multi-peer chunk scheduler:
     - Queries peer swarm for `BitfieldHave(ModelId)` via QUIC control stream.
     - Requests rarest/sequential chunks in parallel (configurable concurrency 4..16 streams) with per-peer RTT/bandwidth scoring.
     - Verifies each 4 MiB chunk's `ChunkHash` in memory *before* pwrite-ing to `.part.data` and setting the bit in `.part.bitfield`.
   - Support selective byte-range / tensor-subset download (`DownloadScope::FullModel` vs `DownloadScope::LayerRange { start_layer, end_layer }`) so memory-constrained mobile nodes only download the GGUF header + their assigned layer shards.
3. **Expose gRPC/HTTP Endpoints (`crates/umcf-daemon/src/rpc/model_registry.rs`)**:
   - `ImportLocalModel`, `GetModelManifest`, `StartModelSync`, `PauseModelSync`, `CancelModelSync`, `SubscribeSyncProgress` (streaming progress events: bytes/sec, ETA, active peers, verified chunks).
4. **UI Integration**:
   - Flutter: `ui/flutter_app/lib/features/ai/widgets/model_download_card.dart` showing chunk bitfield heatmap, pause/resume buttons, and peer swarm count.
   - Svelte 5: `ui/admin_web/src/lib/components/ai/ShardDistributionTable.svelte`.

## 4. Acceptance Criteria
- >= 95% line and branch coverage in `crates/umcf-gguf/src/store.rs` and `src/p2p_sync.rs`; 100% test pass rate.
- Ingesting a 1 GiB GGUF file computes the BLAKE3 manifest using multi-threaded `rayon`/SIMD hashing at >= 1.5 GB/s on desktop and >= 400 MB/s on mobile ARM64.
- Killing a download at any arbitrary millisecond and restarting it resumes from the persistent bitfield with 0 re-transferred completed chunks and produces an identical final `ModelId`.
- Selective `LayerRange` download fetches only the GGUF metadata header chunks plus chunks overlapping layers `[L_start..=L_end]`, saving >= 60% disk/network on a 4-node split.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_manifest_chunk_splitting_exact_and_unaligned_sizes`
  - `test_bitfield_persistence_and_atomic_recovery_after_crash`
  - `test_layer_byte_range_to_chunk_index_mapping`
- **`[INT]`**:
  - `test_multi_peer_swarmed_gguf_download_with_peer_drop`: 3 seeders + 1 leecher over local QUIC; kill seeder #1 at 40%, corrupt chunk #7 from seeder #2 (verify peer ban + retry on seeder #3), verify bit-exact file reconstruction.
  - `test_partial_layer_shard_download_and_range_mmap`
- **`[SIM]`**:
  - Packet loss (5%) and bandwidth throttling simulation verifying sliding-window chunk timeout adjustment.
- **`[HW]`**:
  - Real Wi-Fi 6 / LAN transfer of a 2.5 GiB GGUF between desktop and Android device with airplane-mode toggle mid-transfer.
- **`[MANUAL]`**:
  - Verify Pause/Resume/Cancel buttons and live chunk bitfield visualization in Flutter and Svelte 5 UIs.

## 6. Required Evidence & Artifacts
- `artifacts/stage-05/p1/sp1_unit_int_test_report.json`
- `artifacts/stage-05/p1/sp1_swarm_resume_trace.log` (showing chunk corruption detection, peer penalty, and zero-redundant-chunk resume).

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Create `docs/architecture/MODEL_REGISTRY.md` (Section 1: Content-Addressed Store & Partial Layer Sync).
- Mark `S5.P1.SP1` complete in `TODO.md` and log in `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If partial files are ever promoted to `.gguf` without 100% chunk + root BLAKE3 verification, or if file descriptors leak on cancelled sync streams.
- **Completion Gate**: All `[UNIT]`, `[INT]`, and `[SIM]` tests green with >= 95% coverage before proceeding to `S5.P1.SP2`.
