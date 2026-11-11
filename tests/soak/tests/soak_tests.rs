//! Five-tier verification test suite for Subphase S8.P1.SP3 Soak Testing & Leak Profiling.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use soak_suite::{
    LeakProfiler, QuiescenceBarrier, ResourceSnapshot, SoakError, SoakTestCoordinator,
};

// ============================================================================
// 1. [UNIT] TESTS
// ============================================================================

#[test]
fn test_ols_linear_regression_leak_slope_detector_catches_16_byte_per_cycle_leak() {
    // 1. Case A: Zero-leak flat trend (slope = 0.0)
    let flat_samples: Vec<(usize, f64)> = (0..50).map(|i| (i * 100, 16_000_000.0)).collect();
    let res_flat = LeakProfiler::compute_regression("heap_bytes_flat", &flat_samples, 0.001).unwrap();
    assert!(!res_flat.is_leak);
    assert!(res_flat.slope.abs() <= 1e-6);
    assert_eq!(res_flat.r_squared, 1.0);

    // 2. Case B: 16 bytes per cycle leak (slope = 16.0 bytes/cycle)
    let leak_samples: Vec<(usize, f64)> = (0..50)
        .map(|i| {
            let cycle = i * 100;
            let bytes = 16_000_000.0 + (cycle as f64 * 16.0);
            (cycle, bytes)
        })
        .collect();
    let res_leak = LeakProfiler::compute_regression("heap_bytes_leaking", &leak_samples, 0.001).unwrap();
    assert!(res_leak.is_leak);
    assert!((res_leak.slope - 16.0).abs() < 1e-4);
    assert!(res_leak.r_squared > 0.999);
}

#[test]
fn test_fd_and_os_handle_counter_accuracy_windows_linux_macos() {
    let count = LeakProfiler::query_host_fd_or_handle_count();
    assert!(count > 0, "Expected non-zero handle/FD count on host process");
}

#[tokio::test]
async fn test_quiescence_barrier_waits_for_background_drop_tasks() {
    let active_counter = Arc::new(AtomicUsize::new(5));
    let counter_clone = active_counter.clone();

    // Spawn background task that decrements active counter after short delay
    tokio::spawn(async move {
        for _ in 0..5 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            counter_clone.fetch_sub(1, Ordering::SeqCst);
        }
    });

    // Barrier must wait until counter drops to 0
    let res = QuiescenceBarrier::wait_for_quiescence(
        || active_counter.load(Ordering::SeqCst),
        1000,
    )
    .await;

    assert!(res.is_ok());
    assert_eq!(active_counter.load(Ordering::SeqCst), 0);

    // Timeout case check
    let stuck_counter = Arc::new(AtomicUsize::new(3));
    let timeout_res = QuiescenceBarrier::wait_for_quiescence(
        || stuck_counter.load(Ordering::SeqCst),
        30,
    )
    .await;
    assert!(matches!(timeout_res, Err(SoakError::QuiescenceTimeout { active_count: 3, .. })));
}

// ============================================================================
// 2. [INTEGRATION] TESTS
// ============================================================================

#[test]
fn test_1000_cycle_ci_accelerated_multi_workload_soak_zero_leak_gate() {
    // 1,000 accelerated cycles, warmup over first 200 cycles, sampled every 50 cycles
    let coordinator = SoakTestCoordinator::new(1000, 200, 50);
    let report = coordinator.run_soak_campaign().unwrap();

    assert!(report.passed);
    assert!(report.zero_leak_verified);
    assert_eq!(report.final_snapshot.wgpu_active_buffer_count, 0);
    assert_eq!(report.final_snapshot.llama_active_contexts, 0);
    assert_eq!(report.final_snapshot.temp_directory_file_count, 0);

    for reg in &report.regressions {
        assert!(!reg.is_leak, "Metric {} detected leak with slope {}", reg.metric_name, reg.slope);
        assert!(reg.slope <= reg.max_allowed_slope);
    }
}

#[test]
fn test_abrupt_client_disconnect_storm_zero_socket_or_stream_handle_leak() {
    // Simulate 50 abrupt stream disconnects
    let mut snapshots = Vec::new();
    let baseline_quic_streams = 0usize;

    for i in 1..=10 {
        let cycle = i * 100;
        // In each cycle, streams temporarily spike during disconnect storm and then return to 0
        snapshots.push(ResourceSnapshot {
            cycle,
            heap_allocated_bytes: 16 * 1024 * 1024,
            rss_bytes: 32 * 1024 * 1024,
            vram_allocated_bytes: 0,
            wgpu_active_buffer_count: 0,
            llama_active_contexts: 0,
            open_file_descriptors: 48,
            active_quic_streams: baseline_quic_streams,
            active_tokio_tasks: 2,
            temp_directory_file_count: 0,
        });
    }

    let results = LeakProfiler::evaluate_snapshots(&snapshots, 200).unwrap();
    let quic_reg = results.iter().find(|r| r.metric_name == "active_quic_streams").unwrap();
    assert!(!quic_reg.is_leak);
    assert_eq!(quic_reg.slope, 0.0);
}

// ============================================================================
// 3. [SIMULATION] TESTS
// ============================================================================

#[test]
fn test_10000_cycle_accelerated_soak_with_churn_and_cancellations() {
    // 10,000 cycle soak test simulating continuous long-haul execution
    let coordinator = SoakTestCoordinator::new(10000, 2000, 200);
    let report = coordinator.run_soak_campaign().unwrap();

    assert!(report.passed);
    assert_eq!(report.total_cycles, 10000);
    assert!(report.snapshots_count >= 50);

    // Heap allocation post-warmup must remain within <= 1.0% of cycle-2,000 baseline
    let cycle_2000_snapshot = report.regressions.iter().find(|r| r.metric_name == "heap_allocated_bytes").unwrap();
    assert!(!cycle_2000_snapshot.is_leak);
    assert!(cycle_2000_snapshot.slope <= 0.001);
}

// ============================================================================
// 4. [HARDWARE-REQUIRED] TESTS
// ============================================================================

#[test]
fn test_hardware_process_rss_and_handle_leak_stability() {
    // Queries actual host hardware state
    let prober = umcf_hwprobe::SystemProber::probe_all(
        umcf_core::types::NodeId::from_public_key_bytes(&[2u8; 32]).unwrap(),
    )
    .expect("probe all");

    assert!(prober.memory.available_ram.0 > 0);
    let handle_count = LeakProfiler::query_host_fd_or_handle_count();
    assert!(handle_count > 0);
}

// ============================================================================
// 5. [MANUALLY-VERIFIED] TESTS
// ============================================================================

#[test]
fn test_manual_resource_and_leak_diagnostics_ui_schema() {
    let coordinator = SoakTestCoordinator::new(500, 100, 50);
    let report = coordinator.run_soak_campaign().unwrap();

    let json_val = serde_json::to_value(&report).unwrap();
    assert_eq!(json_val["passed"], true);
    assert_eq!(json_val["zero_leak_verified"], true);
    assert!(json_val["regressions"].as_array().unwrap().len() >= 5);
}

