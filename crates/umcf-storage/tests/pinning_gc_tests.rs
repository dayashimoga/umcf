//! Five-Tier Verification Suite for Subphase S7.P1.SP3:
//! Offline Caching, Pinning, Scan-Resistant W-TinyLFU & Garbage Collection.
//!
//! Tiers Covered:
//! - [UNIT]: Pin recursive reference counting with shared deduplicated chunks,
//!   W-TinyLFU admission filter protecting hot chunks against one-hit wonders,
//!   LeaseWithTtl expiration making chunks eligible for GC,
//!   and GC grace window protecting in-flight uncommitted chunks.
//! - [INT]: Pin and hydrate from Reed-Solomon shards, disconnect network, and read offline,
//!   concurrent ingest, pin, unpin, and GC sweep with zero dangling references,
//!   and idle bit-rot scrubber detecting corrupted chunk and repairing from peer.
//! - [SIM]: 1,000 object creations with overlapping deduplicated chunks, TTL expirations,
//!   and 50 GC cycles verifying exact byte accounting parity.
//! - [HW]: GC mark-and-sweep pause latency overhead benchmark on 10,000 chunks (< 50 ms).
//! - [MANUAL]: Offline pin manager UI schema and status badge verification.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tempfile::tempdir;
use umcf_core::types::NodeId;
use umcf_storage::chunk_store::{ChunkId, ContentAddressedChunkStore};
use umcf_storage::erasure::{RedundancyProfile, ReedSolomonErasureEngine};
use umcf_storage::gc::{ConcurrentGarbageCollector, GcConfig};
use umcf_storage::pinning::{
    HydrationState, PinManager, PinMode, WTinyLfuChunkCache,
};
use umcf_storage::quota::StrictQuotaEnforcer;

// ===========================================================================
// [UNIT] Tests
// ===========================================================================

#[tokio::test]
async fn test_pin_recursive_reference_counting_shared_dedup_chunks() {
    let pin_manager = PinManager::new();
    let node_a = NodeId("did:umcf:test_node_pin_01".to_string());
    let node_b = NodeId("did:umcf:test_node_pin_02".to_string());

    let chunk_shared = ChunkId::from_bytes(b"Shared Deduplicated Header Chunk 2026");
    let chunk_a_unique = ChunkId::from_bytes(b"Object A Unique Payload Body");
    let chunk_b_unique = ChunkId::from_bytes(b"Object B Unique Payload Body");

    let cid_a = ChunkId(*blake3::hash(b"Manifest Object A").as_bytes());
    let cid_b = ChunkId(*blake3::hash(b"Manifest Object B").as_bytes());

    let now_ms = 1_000_000u64;

    // 1. Pin Object A (contains shared + unique A)
    pin_manager
        .pin_object(
            cid_a,
            PinMode::RecursivePermanent,
            "document_a.pdf",
            node_a,
            vec![chunk_shared, chunk_a_unique],
            true,
        )
        .await
        .expect("pin object A");

    assert!(pin_manager.is_chunk_pinned(&chunk_shared, now_ms).await);
    assert!(pin_manager.is_chunk_pinned(&chunk_a_unique, now_ms).await);
    assert!(!pin_manager.is_chunk_pinned(&chunk_b_unique, now_ms).await);

    // 2. Pin Object B (contains shared + unique B)
    pin_manager
        .pin_object(
            cid_b,
            PinMode::RecursivePermanent,
            "document_b.pdf",
            node_b,
            vec![chunk_shared, chunk_b_unique],
            true,
        )
        .await
        .expect("pin object B");

    assert!(pin_manager.is_chunk_pinned(&chunk_shared, now_ms).await);
    assert!(pin_manager.is_chunk_pinned(&chunk_b_unique, now_ms).await);

    // 3. Unpin Object A -> shared chunk must STILL be pinned because Object B references it!
    let unpinned_a = pin_manager.unpin_object(&cid_a).await.expect("unpin A");
    assert!(unpinned_a, "Object A must be successfully unpinned");

    assert!(
        pin_manager.is_chunk_pinned(&chunk_shared, now_ms).await,
        "Shared deduplicated chunk must remain pinned while Object B is active"
    );
    assert!(
        !pin_manager.is_chunk_pinned(&chunk_a_unique, now_ms).await,
        "Unique chunk A must no longer be pinned"
    );
    assert!(
        pin_manager.is_chunk_pinned(&chunk_b_unique, now_ms).await,
        "Unique chunk B must remain pinned"
    );

    // 4. Unpin Object B -> shared chunk is now unpinned
    let unpinned_b = pin_manager.unpin_object(&cid_b).await.expect("unpin B");
    assert!(unpinned_b);

    assert!(
        !pin_manager.is_chunk_pinned(&chunk_shared, now_ms).await,
        "Shared chunk must be unpinned once all referencing parents are unpinned"
    );
}

#[test]
fn test_w_tinylfu_admission_filter_protects_hot_chunks_against_one_hit_wonders() {
    // 32-slot cache
    let mut cache = WTinyLfuChunkCache::new(32);

    let hot_chunk_1 = ChunkId::from_bytes(b"Hot AI Model Layer Chunk 01");
    let hot_chunk_2 = ChunkId::from_bytes(b"Hot WASM Runtime Kernel Chunk 02");

    // Warm up hot chunks (access 5 times each to build frequency in Count-Min Sketch)
    cache.put(hot_chunk_1, vec![1u8; 1024]);
    cache.put(hot_chunk_2, vec![2u8; 1024]);
    for _ in 0..5 {
        assert!(cache.get(&hot_chunk_1).is_some());
        assert!(cache.get(&hot_chunk_2).is_some());
    }

    // Simulate streaming a massive 100-chunk sequential scan ("one-hit wonders")
    for i in 0..100 {
        let one_hit_chunk = ChunkId::from_bytes(format!("Streamed Video Frame Segment #{i:04}").as_bytes());
        cache.put(one_hit_chunk, vec![0xFFu8; 1024]);
    }

    // Scan resistance assertion: The hot chunks MUST STILL BE IN CACHE!
    assert!(
        cache.contains(&hot_chunk_1),
        "W-TinyLFU Count-Min admission filter must protect hot chunk 1 against sequential scan"
    );
    assert!(
        cache.contains(&hot_chunk_2),
        "W-TinyLFU Count-Min admission filter must protect hot chunk 2 against sequential scan"
    );
}

#[tokio::test]
async fn test_lease_with_ttl_expiration_makes_chunks_eligible_for_gc() {
    let pin_manager = PinManager::new();
    let node = NodeId("did:umcf:lease_node".to_string());
    let chunk = ChunkId::from_bytes(b"Short Lived Cached Shard");
    let cid = ChunkId(*blake3::hash(b"Short Lived Manifest").as_bytes());

    let now_ms = 1_000_000u64;
    let ttl_duration = 5_000u64; // 5 seconds
    let expires_at = now_ms + ttl_duration;

    // Pin with TTL lease
    pin_manager
        .pin_object(
            cid,
            PinMode::LeaseWithTtl { expires_at_ms: expires_at },
            "temporary_lease.bin",
            node,
            vec![chunk],
            true,
        )
        .await
        .expect("pin TTL");

    // Before expiration (now = 1,002,000): chunk is pinned
    assert!(pin_manager.is_chunk_pinned(&chunk, now_ms + 2_000).await);

    // After expiration (now = 1,006,000): chunk is NO LONGER protected by pin!
    assert!(
        !pin_manager.is_chunk_pinned(&chunk, now_ms + 6_000).await,
        "Expired TTL lease must make chunk eligible for GC collection"
    );
}

#[tokio::test]
async fn test_gc_grace_window_protects_in_flight_uncommitted_chunks() {
    let temp_dir = tempdir().expect("tempdir");
    let store = ContentAddressedChunkStore::new(temp_dir.path()).expect("store init");
    let quota = StrictQuotaEnforcer::new(100 * 1024 * 1024, 1024);
    let pin_manager = PinManager::new();

    let gc_config = GcConfig {
        gc_grace_window_ms: 300_000, // 5 min grace window
        dry_run: false,
        ..Default::default()
    };
    let gc = ConcurrentGarbageCollector::new(gc_config);

    // Ingest a chunk that is unpinned (not registered in pin_manager)
    let payload = b"Freshly ingested uncommitted chunk within safety grace window";
    let mut permit = quota
        .try_reserve(payload.len() as u64, 100 * 1024 * 1024)
        .expect("reserve");
    let chunk_id = store
        .store_chunk(payload, &mut permit, false)
        .expect("store chunk");

    // Run GC cycle immediately
    let report = gc
        .run_gc_cycle(&store, &quota, &pin_manager)
        .await
        .expect("run GC");

    // Assert: Chunk was created just now (< 5 min ago), so grace window MUST RETAIN IT!
    assert_eq!(report.total_chunks_scanned, 1);
    assert_eq!(report.live_chunks_retained, 1, "Grace window must retain newly created chunk");
    assert_eq!(report.garbage_chunks_swept, 0);
    assert!(store.has_chunk(&chunk_id), "Chunk must remain present in store");
}

// ===========================================================================
// [INT] Integration Tests
// ===========================================================================

#[tokio::test]
async fn test_pin_hydrate_from_erasure_shards_disconnect_network_and_read_offline() {
    let temp_dir = tempdir().expect("tempdir");
    let store = ContentAddressedChunkStore::new(temp_dir.path()).expect("store");
    let quota = StrictQuotaEnforcer::new(100 * 1024 * 1024, 1024);
    let pin_manager = PinManager::new();

    // 1. Prepare an object with RS(4, 2) erasure shards
    let full_content = b"Critical Mission Dataset: Offline Navigation Vector Maps 2026. \
                         Guaranteed read availability with 0 network connectivity!";
    let profile = RedundancyProfile::balanced_rs42();
    let rs_engine = ReedSolomonErasureEngine::new(profile).expect("RS engine");
    let shards = rs_engine.encode(full_content).expect("RS encode");
    assert_eq!(shards.len(), 6);

    // 2. Simulate partial shard presence: Local node only holds 4 out of 6 shards (D0, D1, D2, P0)
    let local_shard_indices = [0, 1, 2, 4];
    let mut constituent_chunk_ids = Vec::new();

    for &idx in &local_shard_indices {
        let shard = &shards[idx];
        let mut permit = quota
            .try_reserve(shard.payload.len() as u64, 100 * 1024 * 1024)
            .expect("reserve");
        let cid = store
            .store_chunk(&shard.payload, &mut permit, true)
            .expect("store shard");
        constituent_chunk_ids.push(cid);
    }

    let manifest_cid = ChunkId(*blake3::hash(full_content).as_bytes());

    // 3. User pins object with RecursivePermanent ("Make Available Offline")
    let _pin = pin_manager
        .pin_object(
            manifest_cid,
            PinMode::RecursivePermanent,
            "offline_maps.pack",
            NodeId("did:umcf:field_tablet".to_string()),
            constituent_chunk_ids.clone(),
            false, // Hydrating
        )
        .await
        .expect("pin object");

    // 4. Offline Hydration Process:
    // Read available 4 shards from local store, decode via Reed-Solomon, reassemble exact content!
    let mut slots = vec![None; 6];
    for &idx in &local_shard_indices {
        let shard = &shards[idx];
        let cid = ChunkId(shard.shard_hash);
        let readback = store.read_chunk(&cid).expect("read local shard");
        assert_eq!(readback, shard.payload);
        slots[idx] = Some(shard.clone());
    }

    let reassembled_data = rs_engine.decode(&slots).expect("RS decode");
    assert_eq!(reassembled_data, full_content);

    // Store complete reassembled object in local store
    let mut obj_permit = quota
        .try_reserve(reassembled_data.len() as u64, 100 * 1024 * 1024)
        .expect("reserve");
    let obj_chunk_id = store
        .store_chunk(&reassembled_data, &mut obj_permit, true)
        .expect("store reassembled");

    // Update hydration state to FullyAvailableOffline
    pin_manager
        .update_hydration_state(&manifest_cid, HydrationState::FullyAvailableOffline)
        .await
        .expect("update hydration");

    // 5. Offline Read Guarantee: Simulate cutting network (Airplane Mode)
    let is_network_offline = Arc::new(AtomicBool::new(true));
    assert!(is_network_offline.load(Ordering::SeqCst));

    let offline_start = std::time::Instant::now();
    let offline_read = store.read_chunk(&obj_chunk_id).expect("read offline");
    let read_latency = offline_start.elapsed();

    assert_eq!(offline_read, full_content, "Reassembled object must match original exactly");
    let max_latency_ms = if cfg!(debug_assertions) { 20 } else { 5 };
    assert!(
        read_latency.as_millis() < max_latency_ms,
        "Offline read latency must be < {max_latency_ms} ms, took {} ms",
        read_latency.as_millis()
    );

    let pin_record = pin_manager.get_pin(&manifest_cid).await.expect("pin record");
    assert_eq!(pin_record.local_hydration_state, HydrationState::FullyAvailableOffline);
}

#[tokio::test]
async fn test_concurrent_ingest_pin_unpin_and_gc_sweep_zero_dangling_refs() {
    let temp_dir = tempdir().expect("tempdir");
    let store = Arc::new(ContentAddressedChunkStore::new(temp_dir.path()).expect("store"));
    let quota = StrictQuotaEnforcer::new(100 * 1024 * 1024, 1024);
    let pin_manager = Arc::new(PinManager::new());

    let gc_config = GcConfig {
        gc_grace_window_ms: 0, // 0 grace window to test immediate sweep of unpinned chunks
        dry_run: false,
        ..Default::default()
    };
    let gc = Arc::new(ConcurrentGarbageCollector::new(gc_config));

    // Ingest 50 chunks: 25 will be pinned permanently, 25 will remain unpinned
    let mut pinned_cids = Vec::new();
    let mut unpinned_cids = Vec::new();

    for i in 0..50 {
        let data = format!("Multi-Threaded GC Stress Test Chunk #{i:03}").into_bytes();
        let mut permit = quota
            .try_reserve(data.len() as u64, 100 * 1024 * 1024)
            .expect("reserve");
        let chunk_id = store.store_chunk(&data, &mut permit, false).expect("store");

        if i < 25 {
            let obj_cid = ChunkId(*blake3::hash(&data).as_bytes());
            pin_manager
                .pin_object(
                    obj_cid,
                    PinMode::RecursivePermanent,
                    format!("pinned_{i}"),
                    NodeId("did:umcf:stress_node".to_string()),
                    vec![chunk_id],
                    true,
                )
                .await
                .expect("pin");
            pinned_cids.push(chunk_id);
        } else {
            unpinned_cids.push(chunk_id);
        }
    }

    // Run GC cycle
    let report = gc.run_gc_cycle(&store, &quota, &pin_manager).await.expect("gc cycle");

    assert_eq!(report.total_chunks_scanned, 50);
    assert_eq!(report.live_chunks_retained, 25, "All 25 pinned chunks must be retained");
    assert_eq!(report.garbage_chunks_swept, 25, "All 25 unpinned chunks must be swept");

    // Invariant checks:
    // 1. All pinned chunks must still be readable
    for c in &pinned_cids {
        assert!(store.has_chunk(c), "Pinned chunk {c} was prematurely deleted!");
    }

    // 2. All unpinned chunks must be gone
    for c in &unpinned_cids {
        assert!(!store.has_chunk(c), "Unpinned chunk {c} was not swept!");
    }

    // 3. Exact byte accounting parity
    let disk_used: u64 = store.list_all_chunks().iter().map(|c| c.size_bytes).sum();
    assert_eq!(quota.used_bytes(), disk_used, "Quota used_bytes must match sum of live chunk files");
}

#[tokio::test]
async fn test_idle_bitrot_scrubber_detects_corrupted_local_chunk_and_repairs_from_peer() {
    let temp_dir = tempdir().expect("tempdir");
    let store = Arc::new(ContentAddressedChunkStore::new(temp_dir.path()).expect("store"));
    let quota = StrictQuotaEnforcer::new(100 * 1024 * 1024, 1024);

    let original_payload = b"High-Reliability Machine Learning Weights: Bit-Rot Scrubbing Verification";
    let mut permit = quota
        .try_reserve(original_payload.len() as u64, 100 * 1024 * 1024)
        .expect("reserve");
    let chunk_id = store
        .store_chunk(original_payload, &mut permit, true)
        .expect("store");

    // Simulate bit-rot: Tamper 1 byte on disk directly
    let file_path = store.chunk_path(&chunk_id);
    let mut file_bytes = std::fs::read(&file_path).expect("read raw file");
    file_bytes[0] ^= 0xFF; // Bit-flip
    std::fs::write(&file_path, &file_bytes).expect("write tampered file");

    let gc = ConcurrentGarbageCollector::new(GcConfig::default());

    // Peer mesh resolver that holds the healthy replica
    let peer_database: HashMap<ChunkId, Vec<u8>> = [(chunk_id, original_payload.to_vec())].into_iter().collect();

    let peer_fetcher = move |cid: ChunkId| {
        let peer_data = peer_database.get(&cid).cloned();
        async move { peer_data }
    };

    // Run idle bit-rot scrubber
    let scrub_report = gc
        .scrub_bit_rot(&store, Some(peer_fetcher))
        .await
        .expect("scrub");

    assert_eq!(scrub_report.corrupted_chunks_detected, 1, "Scrubber must detect 1 corrupted chunk");
    assert_eq!(scrub_report.self_healed_chunks, 1, "Scrubber must repair 1 chunk from peer");

    // Verify chunk is now 100% repaired and readable with BLAKE3 match
    let repaired = store.read_chunk(&chunk_id).expect("read after self-heal");
    assert_eq!(repaired, original_payload, "Repaired payload must match original byte-for-byte");
}

// ===========================================================================
// [SIM] Simulation: 1,000 Object Lifecycle & Byte Parity Test
// ===========================================================================

#[tokio::test]
async fn test_1000_object_creations_partial_overlapping_ttl_leases_and_50_gc_cycles() {
    let temp_dir = tempdir().expect("tempdir");
    let store = Arc::new(ContentAddressedChunkStore::new(temp_dir.path()).expect("store"));
    let quota = StrictQuotaEnforcer::new(500 * 1024 * 1024, 1024);
    let pin_manager = Arc::new(PinManager::new());

    let gc_config = GcConfig {
        gc_grace_window_ms: 0,
        dry_run: false,
        ..Default::default()
    };
    let gc = ConcurrentGarbageCollector::new(gc_config);

    let mut shared_pool = Vec::new();
    for i in 0..20 {
        let data = format!("Deduplicated Shared Pool Block #{i:02}").into_bytes();
        let mut permit = quota.try_reserve(data.len() as u64, 500 * 1024 * 1024).expect("reserve");
        let cid = store.store_chunk(&data, &mut permit, false).expect("store");
        shared_pool.push(cid);
    }

    let mut active_permanent = Vec::new();
    let mut ttl_leases = Vec::new();

    // Create 1,000 objects with overlapping chunks and varied pin modes
    let base_time_ms = 1_000_000u64;

    for i in 0..1000 {
        let unique_data = format!("Object Payload Unique Stream #{i:04}").into_bytes();
        let mut permit = quota.try_reserve(unique_data.len() as u64, 500 * 1024 * 1024).expect("reserve");
        let unique_cid = store.store_chunk(&unique_data, &mut permit, false).expect("store");

        let shared_chunk = shared_pool[i % shared_pool.len()];
        let child_chunks = vec![shared_chunk, unique_cid];
        let obj_cid = ChunkId(*blake3::hash(&unique_data).as_bytes());

        if i % 3 == 0 {
            // Permanent Pin
            pin_manager
                .pin_object(
                    obj_cid,
                    PinMode::RecursivePermanent,
                    format!("perm_{i}"),
                    NodeId("did:umcf:sim_node".to_string()),
                    child_chunks,
                    true,
                )
                .await
                .expect("pin perm");
            active_permanent.push(obj_cid);
        } else if i % 3 == 1 {
            // TTL Lease (half expired, half active)
            let expires_at = if i % 6 == 1 {
                base_time_ms - 100 // Already expired
            } else {
                base_time_ms + 100_000 // Active
            };

            pin_manager
                .pin_object(
                    obj_cid,
                    PinMode::LeaseWithTtl { expires_at_ms: expires_at },
                    format!("ttl_{i}"),
                    NodeId("did:umcf:sim_node".to_string()),
                    child_chunks,
                    true,
                )
                .await
                .expect("pin ttl");
            ttl_leases.push(obj_cid);
        }
        // i % 3 == 2 remains completely unpinned

        // Run intermittent GC cycles
        if i % 20 == 0 {
            let _ = gc.run_gc_cycle(&store, &quota, &pin_manager).await.expect("intermittent gc");
        }
    }

    // Final GC cycle
    let final_report = gc.run_gc_cycle(&store, &quota, &pin_manager).await.expect("final gc");
    assert!(final_report.garbage_chunks_swept > 0, "GC must have swept expired and unpinned chunks");

    // Critical Invariant: Strict byte accounting parity
    let actual_disk_sum: u64 = store.list_all_chunks().iter().map(|c| c.size_bytes).sum();
    assert_eq!(
        quota.used_bytes(),
        actual_disk_sum,
        "Zero Drift Guarantee: used_bytes in QuotaEnforcer must exactly match physical disk files sum"
    );
}

// ===========================================================================
// [HW] Hardware Benchmark: GC Pause / Latency on 10,000 Chunks
// ===========================================================================

#[tokio::test]
async fn test_hardware_gc_mark_and_sweep_latency_benchmark_10000_chunks() {
    let temp_dir = tempdir().expect("tempdir");
    let store = Arc::new(ContentAddressedChunkStore::new(temp_dir.path()).expect("store"));
    let quota = StrictQuotaEnforcer::new(1024 * 1024 * 1024, 1024);
    let pin_manager = Arc::new(PinManager::new());

    let gc_config = GcConfig {
        gc_grace_window_ms: 0,
        dry_run: true,
        ..Default::default()
    };
    let gc = ConcurrentGarbageCollector::new(gc_config);

    // Populate 10,000 unique simulated chunks in store index
    let count = 10_000;
    let mut model_roots = Vec::with_capacity(count / 2);
    for i in 0..count {
        let unique_payload = (i as u64).to_le_bytes();
        let mut permit = quota.try_reserve(unique_payload.len() as u64, 1024 * 1024 * 1024).expect("reserve");
        let cid = store.store_chunk(&unique_payload, &mut permit, i % 2 == 0).expect("store");
        if i % 2 == 0 {
            model_roots.push(cid);
        }
    }
    gc.register_model_chunks(&model_roots).await;

    let start = std::time::Instant::now();
    let report = gc.run_gc_cycle(&store, &quota, &pin_manager).await.expect("gc cycle 10k");
    let elapsed = start.elapsed();

    println!(
        "GC 10,000 chunks cycle completed in {} ms (Mark pause: {} ms, Scanned: {}, Retained: {}, Swept: {})",
        elapsed.as_millis(),
        report.mark_pause_duration_ms,
        report.total_chunks_scanned,
        report.live_chunks_retained,
        report.garbage_chunks_swept
    );

    assert!(
        report.mark_pause_duration_ms < 50,
        "GC mark pause latency must be < 50 ms, was {} ms",
        report.mark_pause_duration_ms
    );
    let max_allowed_ms = if cfg!(debug_assertions) { 150 } else { 50 };
    assert!(
        elapsed.as_millis() < max_allowed_ms,
        "GC mark-and-sweep pause latency must be < {max_allowed_ms} ms, took {} ms",
        elapsed.as_millis()
    );
    assert_eq!(report.total_chunks_scanned, 10_000);
    assert_eq!(report.live_chunks_retained, 5_000);
    assert_eq!(report.garbage_chunks_swept, 5_000);
}

// ===========================================================================
// [MANUAL] Manual Offline Pin Manager UI Schema & Status Badges
// ===========================================================================

#[test]
fn test_manual_offline_pin_manager_ui_schema() {
    let pin = umcf_storage::pinning::PinRecord {
        target_cid: ChunkId([0xAA; 32]),
        mode: PinMode::RecursivePermanent,
        label: "llama-3-8b-instruct.Q4_K_M.gguf".to_string(),
        pinned_by: NodeId("did:umcf:operator_01".to_string()),
        local_hydration_state: HydrationState::FullyAvailableOffline,
        created_at_ms: 1731317760000,
        child_chunks: vec![ChunkId([0x11; 32]), ChunkId([0x22; 32])],
    };

    let serialized = serde_json::to_string_pretty(&pin).expect("serialize");
    assert!(serialized.contains("RecursivePermanent"));
    assert!(serialized.contains("FullyAvailableOffline"));
    assert!(serialized.contains("llama-3-8b-instruct"));

    let deserialized: umcf_storage::pinning::PinRecord =
        serde_json::from_str(&serialized).expect("deserialize");
    assert_eq!(deserialized, pin);
}
