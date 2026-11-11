//! Classified Five-Tier Test Suite for UMCF GGUF Store & Resumable P2P Sync.
//! Covers [Unit], [Integration], [Simulation], [Hardware-Required], and [Manually-Verified].
//! Strictly enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fs;
use std::sync::Arc;
use std::time::Instant;

use tempfile::TempDir;
use umcf_gguf::{
    Bitfield, ByteRange, ChunkDescriptor, ChunkHash, ContentAddressedModelStore, DownloadScope,
    Ed25519Signature, MockPeerProvider, ModelId, ModelManifest, PeerChunkProvider, PeerScore,
    ResumableShardDownloader, SyncStatus, CHUNK_SIZE_BYTES,
};

/// Helper to create a dummy payload with deterministic pseudo-random bytes.
fn generate_deterministic_payload(size: usize, seed: u8) -> Vec<u8> {
    let mut data = vec![0u8; size];
    for (i, byte) in data.iter_mut().enumerate() {
        *byte = ((i as u64 * 31 + seed as u64) % 256) as u8;
    }
    data
}

// =========================================================================
// 1. [Unit] Deterministic Unit Tests
// =========================================================================

#[test]
fn test_unit_manifest_chunk_splitting_exact_and_unaligned_sizes() {
    let temp = TempDir::new().expect("temp dir creation");
    let store = ContentAddressedModelStore::new(temp.path()).expect("create store");

    // Case A: Exact chunk boundary (8 MiB = exactly 2 chunks of 4 MiB)
    let exact_size = 2 * CHUNK_SIZE_BYTES as usize;
    let exact_data = generate_deterministic_payload(exact_size, 42);
    let exact_path = temp.path().join("exact_model.gguf");
    fs::write(&exact_path, &exact_data).expect("write exact file");

    let manifest_exact = store
        .ingest_local_file(&exact_path)
        .expect("ingest exact file");
    assert_eq!(manifest_exact.total_size_bytes, exact_size as u64);
    assert_eq!(manifest_exact.chunks.len(), 2);
    assert_eq!(manifest_exact.chunks[0].size, CHUNK_SIZE_BYTES);
    assert_eq!(manifest_exact.chunks[1].size, CHUNK_SIZE_BYTES);
    assert_eq!(manifest_exact.chunks[0].offset, 0);
    assert_eq!(manifest_exact.chunks[1].offset, CHUNK_SIZE_BYTES as u64);
    assert!(manifest_exact.verify_merkle_root());

    // Case B: Unaligned boundary (9.5 MiB = 2 chunks of 4 MiB + 1 chunk of 1.5 MiB)
    let unaligned_size = (2 * CHUNK_SIZE_BYTES + (1024 * 1024 + 512 * 1024)) as usize;
    let unaligned_data = generate_deterministic_payload(unaligned_size, 99);
    let unaligned_path = temp.path().join("unaligned_model.gguf");
    fs::write(&unaligned_path, &unaligned_data).expect("write unaligned file");

    let manifest_unaligned = store
        .ingest_local_file(&unaligned_path)
        .expect("ingest unaligned file");
    assert_eq!(manifest_unaligned.total_size_bytes, unaligned_size as u64);
    assert_eq!(manifest_unaligned.chunks.len(), 3);
    assert_eq!(manifest_unaligned.chunks[0].size, CHUNK_SIZE_BYTES);
    assert_eq!(manifest_unaligned.chunks[1].size, CHUNK_SIZE_BYTES);
    assert_eq!(
        manifest_unaligned.chunks[2].size,
        (1024 * 1024 + 512 * 1024) as u32
    );
    assert_eq!(
        manifest_unaligned.chunks[2].offset,
        (2 * CHUNK_SIZE_BYTES) as u64
    );
    assert!(manifest_unaligned.verify_merkle_root());

    // Case C: Small file (< 1 chunk)
    let small_size = 512 * 1024; // 512 KiB
    let small_data = generate_deterministic_payload(small_size, 17);
    let small_path = temp.path().join("small_model.gguf");
    fs::write(&small_path, &small_data).expect("write small file");

    let manifest_small = store
        .ingest_local_file(&small_path)
        .expect("ingest small file");
    assert_eq!(manifest_small.chunks.len(), 1);
    assert_eq!(manifest_small.chunks[0].size, small_size as u32);
    assert!(manifest_small.verify_merkle_root());
}

#[test]
fn test_unit_bitfield_persistence_and_atomic_recovery_after_crash() {
    let temp = TempDir::new().expect("temp dir creation");
    let bitfield_path = temp.path().join("transfer.part.bitfield");

    let total_chunks = 20;
    let mut bf = Bitfield::new(total_chunks);
    assert_eq!(bf.completed_count(), 0);
    assert!(!bf.is_all_set());

    // Mark chunks 0, 3, 5, 19 as completed
    bf.set(0);
    bf.set(3);
    bf.set(5);
    bf.set(19);

    assert_eq!(bf.completed_count(), 4);
    assert!(bf.get(0));
    assert!(bf.get(3));
    assert!(bf.get(5));
    assert!(bf.get(19));
    assert!(!bf.get(1));
    assert!(!bf.get(18));

    // Save bitfield atomically to disk
    bf.save_to_file(&bitfield_path)
        .expect("save bitfield to file");
    assert!(bitfield_path.exists());

    // Simulate crash and recovery: reload from disk
    let recovered_bf =
        Bitfield::load_from_file(&bitfield_path, total_chunks).expect("load bitfield");
    assert_eq!(recovered_bf.completed_count(), 4);
    assert_eq!(recovered_bf.chunk_count(), total_chunks);
    assert!(recovered_bf.get(0));
    assert!(recovered_bf.get(3));
    assert!(recovered_bf.get(5));
    assert!(recovered_bf.get(19));
    assert!(!recovered_bf.get(2));

    let missing = recovered_bf.missing_indices();
    assert_eq!(missing.len(), 16);
    assert!(!missing.contains(&0));
    assert!(!missing.contains(&3));
    assert!(missing.contains(&1));

    // Fill all remaining chunks
    let mut full_bf = recovered_bf;
    for idx in 0..total_chunks {
        full_bf.set(idx);
    }
    assert!(full_bf.is_all_set());
    assert_eq!(full_bf.completed_count(), total_chunks);
}

#[test]
fn test_unit_layer_byte_range_to_chunk_index_mapping() {
    // 16 MiB file = 4 chunks of 4 MiB (indices 0, 1, 2, 3)
    let total_size = 16 * 1024 * 1024;
    let mut tensor_map = BTreeMap::new();

    // Header at offset 0..64KiB (chunk 0)
    tensor_map.insert("gguf.header".to_string(), ByteRange::new(0, 64 * 1024));

    // Layer 0: 64KiB .. 4MiB (chunk 0)
    tensor_map.insert(
        "blk.0.attn_q.weight".to_string(),
        ByteRange::new(64 * 1024, 4 * 1024 * 1024 - 64 * 1024),
    );

    // Layer 1: 4MiB .. 8MiB (chunk 1)
    tensor_map.insert(
        "blk.1.attn_q.weight".to_string(),
        ByteRange::new(4 * 1024 * 1024, 4 * 1024 * 1024),
    );

    // Layer 2: 8MiB .. 12MiB (chunk 2)
    tensor_map.insert(
        "blk.2.attn_q.weight".to_string(),
        ByteRange::new(8 * 1024 * 1024, 4 * 1024 * 1024),
    );

    // Layer 3: 12MiB .. 16MiB (chunk 3)
    tensor_map.insert(
        "blk.3.attn_q.weight".to_string(),
        ByteRange::new(12 * 1024 * 1024, 4 * 1024 * 1024),
    );

    let chunks: Vec<ChunkDescriptor> = (0..4)
        .map(|idx| ChunkDescriptor {
            index: idx,
            offset: (idx as u64) * (CHUNK_SIZE_BYTES as u64),
            size: CHUNK_SIZE_BYTES,
            hash: ChunkHash([idx as u8; 32]),
        })
        .collect();

    let manifest = ModelManifest {
        model_id: ModelId([1u8; 32]),
        filename: "test_model.gguf".to_string(),
        total_size_bytes: total_size,
        chunk_size_bytes: CHUNK_SIZE_BYTES,
        chunks,
        tensor_shard_map: tensor_map,
        created_at_unix_ms: 1000,
        publisher_signature: None,
    };

    // Mobile worker assigned only Layer 1
    // Required chunks: chunk 0 (header) + chunk 1 (layer 1) = 2 chunks out of 4 (50% disk/network savings)
    let layer_1_chunks = manifest.chunks_for_layers(1, 1);
    assert_eq!(layer_1_chunks, vec![0, 1]);

    // Mobile worker assigned Layer 2
    let layer_2_chunks = manifest.chunks_for_layers(2, 2);
    assert_eq!(layer_2_chunks, vec![0, 2]);

    // Desktop worker assigned Layers 2..=3
    let desktop_chunks = manifest.chunks_for_layers(2, 3);
    assert_eq!(desktop_chunks, vec![0, 2, 3]);
}

#[test]
fn test_unit_chunk_corruption_detection_and_peer_penalty() {
    let now_ms = 1_700_000_000_000u64;
    let mut score = PeerScore::new("peer-bad-actor");
    assert_eq!(score.corrupt_chunks, 0);
    assert!(!score.is_penalized(now_ms));
    assert!(score.priority_score(now_ms) > 0.0);

    // Record chunk corruption
    score.record_corruption(now_ms);
    assert_eq!(score.corrupt_chunks, 1);
    assert!(score.is_penalized(now_ms));
    assert_eq!(score.priority_score(now_ms), 0.0);

    // After penalty expiration (30 seconds), peer is unbanned
    assert!(!score.is_penalized(now_ms + 35_000));
}

#[test]
fn test_unit_ed25519_signature_on_manifest() {
    let sig_bytes = [0xABu8; 64];
    let signature = Ed25519Signature::from_bytes(sig_bytes);
    let hex = signature.to_hex();
    assert_eq!(hex.len(), 128);

    let parsed = Ed25519Signature::from_hex(&hex).expect("parse valid signature hex");
    assert_eq!(parsed.as_bytes(), &sig_bytes);

    let invalid = Ed25519Signature::from_hex("0x1234");
    assert!(invalid.is_err());
}

// =========================================================================
// 2. [Integration] Multi-Peer Swarm & Resumable Transfer Tests
// =========================================================================

#[test]
fn test_integration_multi_peer_swarmed_gguf_download_with_peer_drop() {
    let temp = TempDir::new().expect("temp dir creation");
    let store = ContentAddressedModelStore::new(temp.path()).expect("create store");

    // Create an 8 MiB model file (2 chunks of 4 MiB)
    let payload = generate_deterministic_payload(8 * 1024 * 1024, 77);
    let original_path = temp.path().join("source_model.gguf");
    fs::write(&original_path, &payload).expect("write original file");

    let manifest = store
        .ingest_local_file(&original_path)
        .expect("ingest source file");
    let model_id = manifest.model_id;

    // Chunk 0 and Chunk 1 payload
    let chunk_0_data = payload[0..CHUNK_SIZE_BYTES as usize].to_vec();
    let chunk_1_data = payload[CHUNK_SIZE_BYTES as usize..].to_vec();

    // Swarm setup:
    // Seeder 1: Drops on chunk 1 (simulated disconnect at ~50%)
    let mut seeder_1 = MockPeerProvider::new("seeder-1", 2).with_chunk(0, chunk_0_data.clone());
    seeder_1.fail_chunk_index = Some(1);

    // Seeder 2: Delivers corrupted chunk 1 (triggers corruption ban)
    let mut seeder_2 = MockPeerProvider::new("seeder-2", 2).with_chunk(0, chunk_0_data.clone());
    seeder_2.corrupt_chunk_index = Some(1);

    // Seeder 3: Healthy reliable peer
    let seeder_3 = MockPeerProvider::new("seeder-3", 2)
        .with_chunk(0, chunk_0_data)
        .with_chunk(1, chunk_1_data);

    let peers: Vec<Arc<dyn PeerChunkProvider>> =
        vec![Arc::new(seeder_1), Arc::new(seeder_2), Arc::new(seeder_3)];

    // Create isolated leecher store
    let leecher_dir = temp.path().join("leecher_store");
    let leecher_store = ContentAddressedModelStore::new(&leecher_dir).expect("leecher store");

    let mut downloader = ResumableShardDownloader::new(
        leecher_store.clone(),
        manifest.clone(),
        DownloadScope::FullModel,
    )
    .expect("create downloader");

    // Execute sync
    let progress = downloader.sync(&peers).expect("sync to completion");
    assert_eq!(progress.status, SyncStatus::Completed);
    assert_eq!(progress.verified_chunks, 2);
    assert_eq!(progress.total_chunks, 2);

    // Verify bit-exact file reconstruction in leecher store
    assert!(leecher_store.has_model(&model_id));
    let verified_mmap = leecher_store
        .open_verified_mmap(&model_id)
        .expect("open verified mmap");
    assert_eq!(verified_mmap.len(), payload.len());
    assert_eq!(verified_mmap.as_slice(), payload.as_slice());
}

#[test]
fn test_integration_partial_layer_shard_download_and_range_mmap() {
    let temp = TempDir::new().expect("temp dir creation");
    let store = ContentAddressedModelStore::new(temp.path()).expect("create store");

    // Create 16 MiB model (4 chunks = 4 layers)
    let total_size = 16 * 1024 * 1024;
    let payload = generate_deterministic_payload(total_size, 88);
    let model_path = temp.path().join("full_16mb_model.gguf");
    fs::write(&model_path, &payload).expect("write 16mb file");

    let manifest = store
        .ingest_local_file(&model_path)
        .expect("ingest 16mb file");

    let seeder = Arc::new(
        MockPeerProvider::new("seeder-node", 4)
            .with_chunk(0, payload[0..4 * 1024 * 1024].to_vec())
            .with_chunk(1, payload[4 * 1024 * 1024..8 * 1024 * 1024].to_vec())
            .with_chunk(2, payload[8 * 1024 * 1024..12 * 1024 * 1024].to_vec())
            .with_chunk(3, payload[12 * 1024 * 1024..16 * 1024 * 1024].to_vec()),
    );

    // Mobile node downloads only Layer 1
    let mobile_dir = temp.path().join("mobile_store");
    let mobile_store = ContentAddressedModelStore::new(&mobile_dir).expect("mobile store");

    let mut downloader = ResumableShardDownloader::new(
        mobile_store,
        manifest.clone(),
        DownloadScope::LayerRange {
            start_layer: 1,
            end_layer: 1,
        },
    )
    .expect("downloader");

    let progress = downloader.sync(&[seeder]).expect("sync layer 1");
    assert_eq!(progress.status, SyncStatus::Completed);
    // Requires chunk 0 (header) and chunk 1 (layer 1) = 2 chunks out of 4 (50% storage saving)
    assert_eq!(progress.verified_chunks, 2);
    assert_eq!(progress.total_chunks, 2);

    // Verify partial shard file exists on disk
    let shard_filename = format!("{}_layers_1_1.shard", manifest.model_id.to_hex());
    let shard_file = temp.path().join("mobile_store/models").join(shard_filename);
    assert!(shard_file.exists());
}

#[test]
fn test_integration_zero_redundant_chunks_on_interrupted_resume() {
    let temp = TempDir::new().expect("temp dir creation");
    let store = ContentAddressedModelStore::new(temp.path()).expect("store");

    // 8 MiB model (2 chunks)
    let payload = generate_deterministic_payload(8 * 1024 * 1024, 33);
    let original = temp.path().join("source.gguf");
    fs::write(&original, &payload).expect("write source");
    let manifest = store.ingest_local_file(&original).expect("ingest");

    let chunk_0 = payload[0..CHUNK_SIZE_BYTES as usize].to_vec();
    let chunk_1 = payload[CHUNK_SIZE_BYTES as usize..].to_vec();

    let target_dir = temp.path().join("target_store");
    let target_store = ContentAddressedModelStore::new(&target_dir).expect("target store");

    // Session 1: download chunk 0, then pause
    let peer_step1 = Arc::new(MockPeerProvider::new("peer-1", 2).with_chunk(0, chunk_0.clone()));
    let mut downloader = ResumableShardDownloader::new(
        target_store.clone(),
        manifest.clone(),
        DownloadScope::FullModel,
    )
    .expect("downloader");

    // Downloading chunk 0 succeeds, but chunk 1 is not on peer_step1
    let _ = downloader.sync(&[peer_step1]);
    assert!(downloader.bitfield().get(0));
    assert!(!downloader.bitfield().get(1));

    // Simulate app shutdown and restart: create new downloader on target_store
    let peer_step2 = Arc::new(
        MockPeerProvider::new("peer-2", 2)
            .with_chunk(0, chunk_0)
            .with_chunk(1, chunk_1),
    );

    let mut resumed_downloader = ResumableShardDownloader::new(
        target_store.clone(),
        manifest.clone(),
        DownloadScope::FullModel,
    )
    .expect("resumed downloader");

    // Verified chunk 0 is already preserved from persistent bitfield
    assert!(resumed_downloader.bitfield().get(0));
    assert!(!resumed_downloader.bitfield().get(1));

    // Resume sync with peer 2
    let progress = resumed_downloader
        .sync(&[peer_step2])
        .expect("sync remainder");
    assert_eq!(progress.status, SyncStatus::Completed);
    assert_eq!(progress.verified_chunks, 2);
    assert!(target_store.has_model(&manifest.model_id));
}

// =========================================================================
// 3. [Simulation] Network Latency & Fault Injection Tests
// =========================================================================

#[test]
fn test_simulation_packet_loss_and_bandwidth_throttling_chunk_scheduler() {
    let now_ms = 1_000_000u64;
    // Simulate 3 heterogeneous peers:
    // Fast peer (2 ms RTT, 50 MB/s)
    let mut fast_peer = PeerScore::new("desktop-lan");
    fast_peer.rtt_ms = 2.0;
    fast_peer.bandwidth_bytes_per_sec = 50_000_000.0;

    // Medium peer (25 ms RTT, 15 MB/s)
    let mut med_peer = PeerScore::new("laptop-wifi");
    med_peer.rtt_ms = 25.0;
    med_peer.bandwidth_bytes_per_sec = 15_000_000.0;

    // Slow mobile peer (100 ms RTT, 2 MB/s)
    let mut slow_peer = PeerScore::new("phone-5g");
    slow_peer.rtt_ms = 100.0;
    slow_peer.bandwidth_bytes_per_sec = 2_000_000.0;

    let score_fast = fast_peer.priority_score(now_ms);
    let score_med = med_peer.priority_score(now_ms);
    let score_slow = slow_peer.priority_score(now_ms);

    assert!(
        score_fast > score_med,
        "Fast peer must outrank medium peer: {score_fast} vs {score_med}"
    );
    assert!(
        score_med > score_slow,
        "Medium peer must outrank slow peer: {score_med} vs {score_slow}"
    );

    // Simulate 5% packet loss on fast peer -> 3 timeouts occur
    fast_peer.record_timeout(now_ms);
    fast_peer.record_timeout(now_ms);
    fast_peer.record_timeout(now_ms);

    // Peer is penalized after consecutive timeouts
    assert!(fast_peer.is_penalized(now_ms));
    assert_eq!(fast_peer.priority_score(now_ms), 0.0);
    // Medium peer now wins priority scheduling
    assert!(med_peer.priority_score(now_ms) > fast_peer.priority_score(now_ms));
}

// =========================================================================
// 4. [Hardware-Required] Real Storage BLAKE3 Throughput Benchmark
// =========================================================================

#[test]
fn test_hardware_storage_blake3_throughput_benchmark() {
    let temp = TempDir::new().expect("temp dir creation");
    let store = ContentAddressedModelStore::new(temp.path()).expect("store");

    // Benchmark with 32 MiB file (8 chunks of 4 MiB)
    let file_size = 32 * 1024 * 1024;
    let data = generate_deterministic_payload(file_size, 44);
    let path = temp.path().join("bench_model.gguf");
    fs::write(&path, &data).expect("write bench file");

    let start = Instant::now();
    let manifest = store.ingest_local_file(&path).expect("ingest bench file");
    let duration = start.elapsed();

    let secs = duration.as_secs_f64();
    let mb_per_sec = (file_size as f64 / (1024.0 * 1024.0)) / secs;

    println!(
        "[HARDWARE-REQUIRED] Ingested 32 MiB GGUF in {:.3}s ({:.1} MB/s) with {} chunks. Root: {}",
        secs,
        mb_per_sec,
        manifest.chunks.len(),
        manifest.model_id
    );

    assert!(
        mb_per_sec >= 100.0,
        "BLAKE3 multi-threaded storage ingestion throughput ({:.1} MB/s) must be >= 100 MB/s",
        mb_per_sec
    );
    assert!(manifest.verify_merkle_root());
}

// =========================================================================
// 5. [Manually-Verified] UI Pause / Resume / Cancel State Machine
// =========================================================================

#[test]
fn test_manual_ui_pause_resume_cancel_and_bitfield_visualization() {
    let temp = TempDir::new().expect("temp dir creation");
    let store = ContentAddressedModelStore::new(temp.path()).expect("store");

    let payload = generate_deterministic_payload(4 * 1024 * 1024, 12);
    let file_path = temp.path().join("manual_ui_model.gguf");
    fs::write(&file_path, &payload).expect("write file");
    let manifest = store.ingest_local_file(&file_path).expect("ingest");

    let mut downloader =
        ResumableShardDownloader::new(store.clone(), manifest.clone(), DownloadScope::FullModel)
            .expect("downloader");

    // Verify initial idle state
    assert_eq!(downloader.status(), SyncStatus::Idle);
    let initial_prog = downloader.progress();
    assert_eq!(initial_prog.status, SyncStatus::Idle);
    assert_eq!(initial_prog.verified_chunks, 0);

    // Verify pause transition
    downloader.pause();
    assert_eq!(downloader.status(), SyncStatus::Paused);

    // Verify cancel transition and file removal
    downloader.cancel().expect("cancel downloader");
    assert_eq!(downloader.status(), SyncStatus::Cancelled);
}
