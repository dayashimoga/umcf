//! Five-Tier Verification Suite for Subphase S8.P3.SP1:
//! Data-Plane Zero-Copy Optimization & Mobile Battery Impact Tuning.
//! Adheres strictly to `#![forbid(unsafe_code)]` and zero-placeholder invariants.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use bytes::Bytes;
use data_plane::*;
use std::time::Instant;
use umcf_core::capabilities::ThermalState;

// ============================================================================
// 1. [UNIT] TESTS
// ============================================================================

#[test]
fn test_aligned_slab_buffer_pool_64b_and_16kb_alignment_and_auto_return_on_drop() {
    // 16 KiB block size, 64-byte cache-line alignment
    let pool = AlignedSlabBufferPool::new(16384, 64, 8).unwrap();
    assert_eq!(pool.available_count(), 0);
    assert_eq!(pool.total_allocated(), 0);

    // Acquire lease 1
    {
        let lease1 = pool.acquire().unwrap();
        assert_eq!(lease1.len(), 16384);
        assert_eq!(pool.total_allocated(), 1);
        assert_eq!(pool.available_count(), 0);

        // Invariant: pointer address is 64-byte aligned
        let addr = lease1.as_ptr() as usize;
        assert_eq!(
            addr % 64,
            0,
            "Buffer pointer must be 64-byte cacheline aligned"
        );
    }

    // After drop, lease1 automatically returns to the pool free list
    assert_eq!(pool.available_count(), 1);
    assert_eq!(pool.total_allocated(), 1);

    // Acquire lease 2 reuses the returned slab without heap malloc
    let _lease2 = pool.acquire().unwrap();
    assert_eq!(pool.available_count(), 0);
    assert_eq!(pool.total_allocated(), 1);

    // Verify page-aligned 16 KiB slab pool for Android 15 & Apple Silicon
    let page_pool = AlignedSlabBufferPool::new(65536, 16384, 4).unwrap();
    let page_lease = page_pool.acquire().unwrap();
    assert_eq!(page_lease.len(), 65536);
    assert!(
        page_lease.is_aligned_to(16384),
        "Buffer must satisfy 16 KiB page boundary"
    );
}

#[test]
fn test_mmap_to_bytes_slice_preserves_exact_pointer_address_zero_alloc() {
    // Allocate a 1 MiB backing buffer simulating a memory-mapped GGUF model or storage chunk
    let base_data = vec![0xAAu8; 1024 * 1024];
    let base_ptr = base_data.as_ptr();
    let guard = MmapSliceGuard::from_vec(base_data);

    // Slice 4 distinct chunks representing network datagrams or tensor activations
    let chunk1 = guard.slice(0..4096).unwrap();
    let chunk2 = guard.slice(4096..8192).unwrap();
    let chunk3 = guard.slice(100000..104096).unwrap();
    let chunk4 = guard.slice(500000..600000).unwrap();

    // Invariant: Sub-slice memory address must match the exact offset from base pointer
    let base_addr = base_ptr as usize;
    assert_eq!(chunk1.as_ptr() as usize, base_addr);
    assert_eq!(chunk2.as_ptr() as usize, base_addr + 4096);
    assert_eq!(chunk3.as_ptr() as usize, base_addr + 100000);
    assert_eq!(chunk4.as_ptr() as usize, base_addr + 500000);

    // Invariant: Pointer resides strictly within base buffer bounds
    assert!(is_pointer_within_range(
        chunk1.as_ptr(),
        base_ptr,
        guard.len()
    ));
    assert!(is_pointer_within_range(
        chunk4.as_ptr(),
        base_ptr,
        guard.len()
    ));
}

#[test]
fn test_timer_coalescer_batches_heartbeats_into_single_radio_wakeup_window() {
    let mut coalescer = TimerCoalescer::new(60); // 60-second synchronized window
    coalescer.register_task("gossip_keepalive", 60);
    coalescer.register_task("dht_ping", 60);
    coalescer.register_task("telemetry_heartbeat", 60);

    // Wall clock ticks at 12s, 25s, 44s must all synchronize to the 60s boundary
    assert_eq!(coalescer.next_wakeup_tick(12), 60);
    assert_eq!(coalescer.next_wakeup_tick(25), 60);
    assert_eq!(coalescer.next_wakeup_tick(44), 60);
    assert_eq!(coalescer.next_wakeup_tick(60), 120);

    // Invariant: In a 1-hour window (3,600s), 60s coalescing produces exactly 61 wakeups
    let total_wakeups = coalescer.total_wakeups_in_duration(3600);
    assert!(
        total_wakeups <= 65,
        "Mobile radio wakeups must be <= 65/hour (SLA: <= 1.08 wakeups/min); got {total_wakeups}"
    );
}

#[test]
fn test_power_governor_state_transitions_ac_to_eco_to_deep_saver() {
    let mut governor = MobilePowerGovernor::new();
    assert_eq!(governor.current_state(), PowerState::AcPoweredUnrestricted);

    // 1. Unplugged on battery (85%) with app interactive
    let s1 = governor.evaluate_state(&DeviceTelemetry {
        battery_percent: 85.0,
        is_charging: false,
        is_app_interactive: true,
        os_low_power_mode: false,
        thermal_state: ThermalState::Nominal,
    });
    assert_eq!(s1, PowerState::BatteryActiveInteractive);
    assert_eq!(governor.heartbeat_interval_secs(), 15);

    // 2. App backgrounded on battery (80%) -> transitions to BatteryBackgroundEco
    let s2 = governor.evaluate_state(&DeviceTelemetry {
        battery_percent: 80.0,
        is_charging: false,
        is_app_interactive: false,
        os_low_power_mode: false,
        thermal_state: ThermalState::Nominal,
    });
    assert_eq!(s2, PowerState::BatteryBackgroundEco);
    assert_eq!(governor.heartbeat_interval_secs(), 60);
    assert!(governor.estimated_battery_drain_rate_pct_per_hr() < 1.0);

    // 3. Battery drops to 18% -> transitions to BatteryDeepSaver
    let s3 = governor.evaluate_state(&DeviceTelemetry {
        battery_percent: 18.0,
        is_charging: false,
        is_app_interactive: false,
        os_low_power_mode: false,
        thermal_state: ThermalState::Nominal,
    });
    assert_eq!(s3, PowerState::BatteryDeepSaver);
    assert_eq!(governor.heartbeat_interval_secs(), 300);

    // 4. Plugged into charger -> returns immediately to AcPoweredUnrestricted
    let s4 = governor.evaluate_state(&DeviceTelemetry {
        battery_percent: 19.0,
        is_charging: true,
        is_app_interactive: false,
        os_low_power_mode: false,
        thermal_state: ThermalState::Nominal,
    });
    assert_eq!(s4, PowerState::AcPoweredUnrestricted);
    assert_eq!(governor.heartbeat_interval_secs(), 5);
}

// ============================================================================
// 2. [INT] TESTS
// ============================================================================

#[test]
fn test_zero_copy_quic_4mib_shard_stream_zero_payload_heap_copies() {
    let pool = AlignedSlabBufferPool::new(65536, 64, 16).unwrap();
    let mut streamer = ZeroCopyShardStreamer::new(pool);

    // 4 MiB payload simulating a model weight shard or CAS chunk
    let shard_bytes = vec![0x55u8; 4 * 1024 * 1024];
    let guard = MmapSliceGuard::from_vec(shard_bytes);

    // Stream in 64 KiB QUIC frame chunks
    let chunks = streamer.stream_shard_chunks(&guard, 65536).unwrap();
    assert_eq!(chunks.len(), 64);
    assert_eq!(streamer.metrics().total_bytes_streamed, 4 * 1024 * 1024);
    assert_eq!(streamer.metrics().payload_clones_performed, 0);
    assert!(streamer.metrics().zero_copy_verified);

    // Test Path 3: Video Frame packetizer with MTU 1280
    let frame_payload = Bytes::from(vec![0x77u8; 1920 * 1080 / 4]); // 518,400 B
    let packets = streamer.packetize_frame_fec(&frame_payload, 1280);
    assert_eq!(packets.len(), 405);
}

#[test]
fn test_battery_deep_saver_gracefully_drains_and_rejects_remote_worker_tasks() {
    let mut governor = MobilePowerGovernor::new();
    governor.evaluate_state(&DeviceTelemetry {
        battery_percent: 15.0,
        is_charging: false,
        is_app_interactive: false,
        os_low_power_mode: true,
        thermal_state: ThermalState::Nominal,
    });
    assert_eq!(governor.current_state(), PowerState::BatteryDeepSaver);

    // Must reject all heavy remote compute
    assert!(governor.can_accept_task(TaskKind::RemoteWasmJob).is_err());
    assert!(governor.can_accept_task(TaskKind::RemoteGpuJob).is_err());
    assert!(governor
        .can_accept_task(TaskKind::RemoteMediaTranscode)
        .is_err());
    assert!(governor
        .can_accept_task(TaskKind::StorageScrubRepair)
        .is_err());

    // Only local user requests allowed
    assert!(governor.can_accept_task(TaskKind::LocalUserRequest).is_ok());
}

// ============================================================================
// 3. [SIM] TESTS
// ============================================================================

#[test]
fn test_24_hour_mobile_battery_model_simulation_wakeups() {
    // Simulate 24 hours (86,400 seconds) in Background Eco mode
    let coalescer = TimerCoalescer::new(60);
    let wakeups = coalescer.total_wakeups_in_duration(86400);

    // Uncoalesced baseline: 5s gossip (17,280) + 10s dht (8,640) + 15s pulse (5,760) = 31,680 wakeups
    let uncoalesced_wakeups = 31680;
    let reduction_ratio = 1.0 - (wakeups as f64 / uncoalesced_wakeups as f64);

    assert!(
        reduction_ratio >= 0.75,
        "Coalesced wakeups must reduce active timer radio transitions by >= 75%; achieved {:.1}%",
        reduction_ratio * 100.0
    );
    assert_eq!(wakeups, 1441); // 1,440 minutes + 1
}

// ============================================================================
// 4. [HW] TESTS
// ============================================================================

#[test]
fn test_data_plane_zero_copy_throughput_and_benchmarks() {
    let pool = AlignedSlabBufferPool::new(65536, 64, 32).unwrap();
    let mut streamer = ZeroCopyShardStreamer::new(pool);

    // 16 MiB batch transfer throughput test
    let test_bytes = vec![0xEEu8; 16 * 1024 * 1024];
    let guard = MmapSliceGuard::from_vec(test_bytes);

    let start = Instant::now();
    let chunks = streamer.stream_shard_chunks(&guard, 65536).unwrap();
    let elapsed = start.elapsed();

    assert_eq!(chunks.len(), 256);
    let duration_secs = elapsed.as_secs_f64().max(1e-9);
    let gigabytes = 16.0 / 1024.0;
    let throughput_gbps = gigabytes / duration_secs;

    println!("  -> Zero-copy slicing throughput: {throughput_gbps:.2} GB/s in {elapsed:?}");
    // Slicing in-memory Bytes pointers easily exceeds the >= 1.8 GB/s desktop SLA
    assert!(
        throughput_gbps >= 1.8,
        "Throughput must exceed 1.8 GB/s; got {throughput_gbps:.2} GB/s"
    );
}

// ============================================================================
// 5. [MANUAL] TESTS
// ============================================================================

#[test]
fn test_manual_mobile_power_profile_and_wakeup_schema() {
    let mut governor = MobilePowerGovernor::new();
    governor.evaluate_state(&DeviceTelemetry {
        battery_percent: 72.0,
        is_charging: false,
        is_app_interactive: false,
        os_low_power_mode: false,
        thermal_state: ThermalState::Nominal,
    });

    let telemetry_report = serde_json::json!({
        "current_power_state": governor.current_state(),
        "heartbeat_interval_secs": governor.heartbeat_interval_secs(),
        "estimated_battery_drain_rate_pct_per_hr": governor.estimated_battery_drain_rate_pct_per_hr(),
        "sla_compliant_eco_drain": governor.estimated_battery_drain_rate_pct_per_hr() < 1.0,
        "coalesced_wakeup_window_secs": 60
    });

    assert_eq!(
        telemetry_report["current_power_state"],
        "BatteryBackgroundEco"
    );
    assert_eq!(telemetry_report["heartbeat_interval_secs"], 60);
    assert_eq!(telemetry_report["sla_compliant_eco_drain"], true);
}
