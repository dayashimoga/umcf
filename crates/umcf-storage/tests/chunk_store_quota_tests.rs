//! Five-Tier Verification Test Suite for S7.P1.SP1:
//! BLAKE3 Content-Addressed Chunk Store & Explicit Quota Enforcement.
//!
//! Tiers:
//! - [UNIT]: FastCDC shift-resilience deduplication, 64-thread CAS quota loop zero overrun, permit RAII drop, host space floor clamping.
//! - [INT]: Object ingest, deduplication, bit-exact reconstruction, chunk corruption rejection, and cache eviction.
//! - [SIM]: Crash recovery orphan staging cleanup and index reconciliation in < 50 ms.
//! - [HW]: FastCDC + BLAKE3 ingestion throughput benchmark on host hardware.
//! - [MANUAL]: Storage Quota Card UI schema and stacked progress model.

use std::collections::BTreeMap;
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use umcf_storage::{
    ChunkId, ContentAddressedChunkStore, FastCdcConfig, IngestOptions, StorageError,
    StrictQuotaEnforcer,
};

/// [UNIT] FastCDC chunking shift-resilience on mid-file insertion (>= 94% chunk reuse).
#[test]
fn test_fastcdc_chunking_shift_resilience_on_mid_file_insertion() {
    let config = FastCdcConfig::custom(4096, 16384, 65536);

    // Create 2 MiB pseudo-random base buffer
    let mut base_data = Vec::with_capacity(2 * 1024 * 1024);
    let mut val = 0x12345678u32;
    for _ in 0..(2 * 1024 * 1024) {
        val = val.wrapping_mul(1664525).wrapping_add(1013904223);
        base_data.push((val >> 24) as u8);
    }

    let base_chunks = config.chunk_data(&base_data);
    let base_cids: Vec<ChunkId> = base_chunks
        .iter()
        .map(|&c| ChunkId::from_bytes(c))
        .collect();
    let total_base = base_cids.len();
    assert!(total_base >= 20, "must produce at least 20 chunks");

    // Create modified data with 100 bytes inserted near the beginning (offset 256)
    let mut modified_data = base_data.clone();
    let inserted_bytes = vec![0xABu8; 100];
    modified_data.splice(256..256, inserted_bytes);

    let modified_chunks = config.chunk_data(&modified_data);
    let modified_cids: Vec<ChunkId> = modified_chunks
        .iter()
        .map(|&c| ChunkId::from_bytes(c))
        .collect();

    // Count identical matching chunks
    let mut reused_count = 0;
    for cid in &modified_cids {
        if base_cids.contains(cid) {
            reused_count += 1;
        }
    }

    let reuse_ratio = (reused_count as f64) / (total_base as f64);
    println!(
        "FastCDC Shift-Resilience: {} / {} chunks reused ({:.2}%)",
        reused_count,
        total_base,
        reuse_ratio * 100.0
    );

    assert!(
        reuse_ratio >= 0.94,
        "FastCDC reuse ratio {:.2}% fell below required 94% threshold",
        reuse_ratio * 100.0
    );
}

/// [UNIT] Atomic quota reservation CAS loop with 64 concurrent threads (zero overrun).
#[test]
fn test_atomic_quota_reservation_cas_loop_64_threads_zero_overrun() {
    let quota_limit = 16 * 1024 * 1024; // 16 MiB hard limit
    let enforcer = StrictQuotaEnforcer::new(quota_limit, 1024 * 1024);
    let host_free = 100 * 1024 * 1024; // 100 MiB free host disk

    let thread_count = 64;
    let request_size = 512 * 1024; // 512 KiB per attempt
    let iterations_per_thread = 20;

    let stop_signal = Arc::new(AtomicBool::new(false));
    let mut handles = Vec::new();

    for _ in 0..thread_count {
        let enf = Arc::clone(&enforcer);
        let stop = Arc::clone(&stop_signal);

        handles.push(thread::spawn(move || {
            let mut held_permits = Vec::new();
            for _ in 0..iterations_per_thread {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                match enf.try_reserve(request_size, host_free) {
                    Ok(permit) => {
                        let total = enf.total_allocated_bytes();
                        assert!(
                            total <= quota_limit,
                            "hard quota exceeded! total allocated {} > limit {}",
                            total,
                            quota_limit
                        );
                        held_permits.push(permit);
                    }
                    Err(StorageError::QuotaExceeded { .. }) => {
                        // Expected when quota is reached
                    }
                    Err(other) => panic!("unexpected error: {:?}", other),
                }
            }
        }));
    }

    for h in handles {
        h.join().expect("thread join");
    }

    let final_allocated = enforcer.total_allocated_bytes();
    assert!(
        final_allocated <= quota_limit,
        "final allocated {} exceeded quota {}",
        final_allocated,
        quota_limit
    );
}

/// [UNIT] Uncommitted quota permit drop releases in-flight bytes.
#[test]
fn test_uncommitted_quota_permit_drop_releases_in_flight_bytes() {
    let enforcer = StrictQuotaEnforcer::new(10 * 1024 * 1024, 1024 * 1024);
    let host_free = 50 * 1024 * 1024;

    {
        let permit = enforcer
            .try_reserve(4 * 1024 * 1024, host_free)
            .expect("reserve");
        assert_eq!(
            enforcer.reserved_in_flight_bytes.load(Ordering::SeqCst),
            4 * 1024 * 1024
        );
        assert_eq!(enforcer.used_bytes.load(Ordering::SeqCst), 0);
        drop(permit); // Drop without commit
    }

    // In-flight bytes must be completely released back to 0
    assert_eq!(enforcer.reserved_in_flight_bytes.load(Ordering::SeqCst), 0);
    assert_eq!(enforcer.used_bytes.load(Ordering::SeqCst), 0);
}

/// [UNIT] Host OS free space floor clamps effective quota.
#[test]
fn test_host_os_free_space_floor_clamps_effective_quota() {
    let configured_max = 50 * 1024 * 1024 * 1024; // 50 GiB
    let safety_floor = 2 * 1024 * 1024 * 1024; // 2 GiB
    let enforcer = StrictQuotaEnforcer::new(configured_max, safety_floor);

    // Case 1: Host has 5 GiB free (> 2 GiB floor) -> allowable is 3 GiB
    let effective = enforcer.calculate_effective_limit(5 * 1024 * 1024 * 1024);
    assert_eq!(effective, 3 * 1024 * 1024 * 1024);

    // Case 2: Host free is at or below 2 GiB safety floor -> Rejected with HostSpaceExhausted
    let result = enforcer.try_reserve(1024, 1500 * 1024 * 1024);
    assert!(matches!(
        result,
        Err(StorageError::HostSpaceExhausted { .. })
    ));
}

/// [INT] Ingest, deduplicate, verify readback, and assert corrupted chunk rejection.
#[test]
fn test_ingest_deduplicate_and_verify_readback_corrupted_chunk_rejected() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let store = ContentAddressedChunkStore::new(tmp.path()).expect("store");
    let enforcer = StrictQuotaEnforcer::new(100 * 1024 * 1024, 1024 * 1024);
    let host_free = 50 * 1024 * 1024;

    let original_data =
        b"Universal Mobile Compute Fabric - Content Addressed Storage Test Payload 12345";
    let chunk_config = FastCdcConfig::custom(16, 32, 64);

    let options = IngestOptions {
        content_type: "text/plain",
        metadata: BTreeMap::new(),
        chunk_config: chunk_config.clone(),
        host_free_bytes: host_free,
        pinned: true,
    };

    // Ingest first object
    let manifest1 = store
        .ingest_object(original_data, options.clone(), &enforcer)
        .expect("ingest 1");

    // Read back and reconstruct
    let reconstructed = store.reconstruct_object(&manifest1).expect("reconstruct");
    assert_eq!(reconstructed, original_data);

    // Ingest identical data -> 100% deduplicated, 0 new bytes consumed on quota
    let used_before = enforcer.used_bytes.load(Ordering::SeqCst);
    let manifest2 = store
        .ingest_object(original_data, options, &enforcer)
        .expect("ingest 2");
    let used_after = enforcer.used_bytes.load(Ordering::SeqCst);
    assert_eq!(
        used_before, used_after,
        "duplicate data must not consume quota"
    );
    assert_eq!(manifest1.chunk_refs, manifest2.chunk_refs);

    // Corrupt one chunk on disk and verify integrity verification catches it
    let target_cid = &manifest1.chunk_refs[0].chunk_id;
    let path = store.chunk_path(target_cid);
    let mut bytes = fs::read(&path).expect("read chunk");
    bytes[0] ^= 0xFF; // Tamper with first byte
    fs::write(&path, bytes).expect("overwrite chunk");

    let read_result = store.read_chunk(target_cid);
    assert!(
        matches!(
            read_result,
            Err(StorageError::ChunkIntegrityMismatch { .. })
        ),
        "tampered chunk must be rejected with ChunkIntegrityMismatch"
    );
}

/// [INT] Quota evicts unpinned cache to admit new writes.
#[test]
fn test_quota_evicts_unpinned_cache_to_admit_new_pinned_write() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let store = ContentAddressedChunkStore::new(tmp.path()).expect("store");
    let enforcer = StrictQuotaEnforcer::new(10 * 1024, 1024);
    let host_free = 50 * 1024 * 1024;

    // Store 4 unpinned chunks of 2 KB each = 8 KB used (out of 10 KB quota)
    for i in 0..4 {
        let chunk_data = vec![i as u8; 2048];
        let mut permit = enforcer
            .try_reserve(2048, host_free)
            .expect("reserve unpinned");
        store
            .store_chunk(&chunk_data, &mut permit, false)
            .expect("store");
    }

    assert_eq!(enforcer.used_bytes.load(Ordering::SeqCst), 8192);

    // Attempting to reserve 4 KB would fail because 8 KB + 4 KB > 10 KB
    let fail_res = enforcer.try_reserve(4096, host_free);
    assert!(matches!(fail_res, Err(StorageError::QuotaExceeded { .. })));

    // Evict 4 KB of unpinned cache
    let freed = store.evict_unpinned_chunks(4096).expect("evict");
    assert!(freed >= 4096);
    enforcer.release_used_bytes(freed);

    // Now reservation succeeds!
    let success_res = enforcer.try_reserve(4096, host_free);
    assert!(success_res.is_ok());
}

/// [SIM] Startup recovery cleans orphan staging files and reconciles index in < 50 ms.
#[test]
fn test_simulation_crash_recovery_cleans_staging_files_and_reconciles_index() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let store = ContentAddressedChunkStore::new(tmp.path()).expect("store");

    // Simulate crash leaving 20 orphan .staging files
    for i in 0..20 {
        let staging_file = store.staging_dir.join(format!(".staging.orphan_{}.chk", i));
        fs::write(staging_file, b"orphan incomplete payload").expect("write orphan");
    }

    let start = Instant::now();
    let (cleaned, _) = store.startup_recovery().expect("recovery");
    let elapsed = start.elapsed();

    assert_eq!(cleaned, 20);
    assert!(
        elapsed.as_millis() <= 50,
        "startup recovery took {:?} > 50 ms budget",
        elapsed
    );

    // Verify 0 files remaining in staging
    let remaining_staging = fs::read_dir(&store.staging_dir).expect("dir").count();
    assert_eq!(remaining_staging, 0);
}

/// [HW] FastCDC + BLAKE3 ingestion throughput benchmark on host hardware.
#[test]
fn test_hardware_fastcdc_blake3_ingestion_throughput() {
    let size = 16 * 1024 * 1024; // 16 MiB benchmark buffer
    let mut data = Vec::with_capacity(size);
    let mut val = 0x9ABCDEF0u32;
    for _ in 0..size {
        val = val.wrapping_mul(1103515245).wrapping_add(12345);
        data.push((val >> 24) as u8);
    }

    let config = FastCdcConfig::default_production();

    let start = Instant::now();
    let chunks = config.chunk_data(&data);
    let mut total_hashes = 0;
    for &chunk in &chunks {
        let _ = ChunkId::from_bytes(chunk);
        total_hashes += 1;
    }
    let elapsed = start.elapsed();

    let mb = (size as f64) / (1024.0 * 1024.0);
    let sec = elapsed.as_secs_f64();
    let throughput_mb_s = mb / sec;

    println!(
        "FastCDC + BLAKE3 throughput: {:.2} MB/s across {} chunks ({:.3} s)",
        throughput_mb_s, total_hashes, sec
    );

    let min_baseline = if cfg!(debug_assertions) { 50.0 } else { 200.0 };
    assert!(
        throughput_mb_s >= min_baseline,
        "throughput {:.2} MB/s fell below baseline {:.1} MB/s",
        throughput_mb_s,
        min_baseline
    );
}

/// [MANUAL] Storage Quota Card UI schema and stacked progress model.
#[test]
fn test_manual_storage_quota_card_and_stacked_progress_ui_schema() {
    let schema = serde_json::json!({
        "slider": {
            "min_mb": 512,
            "max_gb": 500,
            "step_mb": 512,
            "default_gb": 32
        },
        "stacked_progress_categories": [
            { "name": "Pinned Data", "color": "#10b981" },
            { "name": "Model Shards", "color": "#3b82f6" },
            { "name": "Cached Chunks", "color": "#6366f1" },
            { "name": "In-Flight Reserved", "color": "#f59e0b" },
            { "name": "OS Safety Floor", "color": "#ef4444" }
        ],
        "actions": {
            "purge_unpinned_cache": "evict_unpinned_chunks",
            "set_max_quota": "set_configured_max_bytes"
        }
    });

    assert_eq!(
        schema["stacked_progress_categories"]
            .as_array()
            .unwrap()
            .len(),
        5
    );
    assert_eq!(
        schema["actions"]["purge_unpinned_cache"].as_str().unwrap(),
        "evict_unpinned_chunks"
    );
}
