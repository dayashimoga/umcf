# Phase S5.P1 Execution Prompt: Model Registry, Content-Addressed GGUF Management & Quantization Verification

## 1. Role & Context
You are the Distributed Storage & AI Model Registry Lead for UMCF. Phase `S5.P1` builds the content-addressed GGUF model registry, resumable P2P shard distribution engine, zero-copy GGUF v2/v3 metadata inspector, exact RAM/VRAM + KV-cache footprint estimator, and cryptographic model integrity/licensing/RBAC enforcement in `crates/umcf-gguf`.

This phase consists of three sequential subphases:
1. `S5_P1_SP1.md`: Content-Addressed Model Store & Resumable P2P Shard Distribution
2. `S5_P1_SP2.md`: GGUF Metadata Inspector, Memory Footprint Estimator & Compatibility Validator
3. `S5_P1_SP3.md`: Model Integrity Verification, Licensing & Access Control

## 2. Prerequisites & Dependencies
- Completion of Stage 4 (`umcf-scheduler`, `umcf-net` QUIC streams, `umcf-crypto` Ed25519/BLAKE3 primitives, `umcf-hw` device capability descriptors).
- Access to reference small GGUF test fixtures generated deterministically during test bootstrap (`crates/umcf-gguf/tests/fixtures/`).

## 3. Implementation Tasks
1. **Subphase S5.P1.SP1 (`crates/umcf-gguf/src/store.rs`, `src/p2p_sync.rs`)**:
   - Implement `ContentAddressedModelStore` with BLAKE3 Merkle tree manifests (`ModelManifest`, `ShardDescriptor`) split into 4 MiB chunks.
   - Implement `ResumableShardDownloader` over `umcf-net` QUIC streams with HTTP Range / Bitfield resumption, multi-peer swarm pulling, and atomic rename on verified completion.
2. **Subphase S5.P1.SP2 (`crates/umcf-gguf/src/parser.rs`, `src/estimator.rs`)**:
   - Implement zero-copy memory-mapped GGUF v2/v3 parser (`GgufHeader`, `GgufMetadataValue`, `GgufTensorInfo`, `GgufQuantType`).
   - Implement `ModelFootprintEstimator::estimate_memory(&GgufFile, ContextConfig, HardwareCapabilities) -> MemoryEstimate` calculating static weights, KV-cache bytes per layer, compute buffer overhead, and hardware quantization compatibility.
3. **Subphase S5.P1.SP3 (`crates/umcf-gguf/src/security.rs`, `src/license.rs`)**:
   - Implement Ed25519 manifest signature validation, continuous chunk-level BLAKE3 verification before mmap, SPDX/custom AI model license policy enforcement (`LicensePolicyEngine`), and cluster RBAC access control (`ModelAccessPolicy`).
4. **UI & RPC Integration**:
   - Expose gRPC `ModelRegistryService` (`ListModels`, `PullModel`, `InspectGguf`, `VerifyModelIntegrity`, `SetModelAccessPolicy`).
   - Wire Flutter `ModelRegistryScreen` and Svelte 5 `/ai` model management table.

## 4. Acceptance Criteria
- >= 95% line/branch coverage across `crates/umcf-gguf`; 100% test pass rate.
- Interrupted P2P transfers resume from the exact last verified 4 MiB chunk with zero redundant network bytes for completed chunks.
- Memory footprint estimation error is within <= 2.5% of actual `llama.cpp` runtime allocation across Q4_0, Q4_K_M, Q5_K_M, Q8_0, and F16 quantizations.
- Corrupted GGUF headers, out-of-bounds tensor offsets, or tampered shard bytes are rejected in < 5 ms before any FFI or mmap load occurs.

## 5. Required Tests
- **`[UNIT]`**: GGUF v2/v3 binary parser edge cases, quantization block size tables, KV-cache calculation math, Merkle proof validation, license policy evaluation.
- **`[INT]`**: 3-peer QUIC swarm download with injected stream resets at 35% and 80% progress; RBAC denial for unauthorized peer pull requests.
- **`[SIM]`**: Bit-flip fault injection across 500 randomized GGUF headers and tensor offset tables.
- **`[HW]`**: Memory-mapped BLAKE3 verification throughput benchmark on NVMe desktop vs UFS 3.1/4.0 Android storage.
- **`[MANUAL]`**: Model import, progress bar resumption, memory badge color-coding (Fits in VRAM / Split Required / Insufficient RAM), and license acceptance modal in Flutter & Svelte 5 UIs.

## 6. Required Evidence & Artifacts
- `artifacts/stage-05/p1/gguf_parser_fuzz_summary.json`
- `artifacts/stage-05/p1/memory_estimator_accuracy.json`
- `artifacts/stage-05/p1/p2p_resume_transfer_log.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document GGUF manifest schema and memory estimation formulas in `docs/architecture/MODEL_REGISTRY.md`.
- Update `TODO.md` (`S5.P1.SP1`–`S5.P1.SP3`) and `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If any crafted GGUF file can trigger a Rust panic (`unwrap`/`slice::from_raw_parts` out of bounds) or if an unverified shard is exposed to `umcf-llama-sys`.
- **Gate**: All 3 subphases (`S5_P1_SP1`..`SP3`) complete and verified before starting `S5.P2`.
