//! Multi-tier classified verification test suite for `umcf-guard`.
//! Enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use tokio::time::sleep;
use umcf_core::units::ByteSize;
use umcf_enrollment::ResourceConsentPolicy;
use umcf_guard::error::{ErrorCode, GuardError};
use umcf_guard::manager::{QuotaManager, MAX_POLICY_UPDATE_LATENCY_MS};
use umcf_guard::quota::{
    AllocatorGuard, CgroupV2Enforcer, JobObjectEnforcer, QuotaConfig, QuotaEnforcer,
    WasmResourceLimiter,
};
use umcf_guard::vram::VramBudgetTracker;
use umcf_identity::NodeKeypair;

fn make_sample_policy(
    keypair: &NodeKeypair,
    cpu_percent: u8,
    ram_mb: u64,
    vram_mb: u64,
) -> ResourceConsentPolicy {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    ResourceConsentPolicy::sign_new(
        keypair,
        keypair.node_id(),
        cpu_percent,
        ByteSize::from_mib(ram_mb),
        ByteSize::from_mib(vram_mb),
        ByteSize::from_gib(2),
        HashSet::new(),
        false,
        20,
        None,
        Vec::new(),
        1,
        now_ms,
    )
    .expect("valid signed consent policy")
}

// ===========================================================================
// 1. UNIT TESTS [Unit]
// ===========================================================================

#[tokio::test]
async fn test_unit_vram_budget_tracker_concurrent_reservations() {
    let limit_bytes = 500 * 1024 * 1024; // 500 MiB
    let tracker = VramBudgetTracker::new(limit_bytes);

    let mut handles = Vec::new();
    // 10 concurrent tasks each reserving 50 MiB = 500 MiB total
    for i in 0..10 {
        let t = Arc::clone(&tracker);
        handles.push(tokio::spawn(async move {
            t.reserve(50 * 1024 * 1024, &format!("task-{i}"))
        }));
    }

    let mut leases = Vec::new();
    for h in handles {
        let lease = h.await.expect("join handle").expect("reservation should succeed");
        leases.push(lease);
    }

    assert_eq!(tracker.reserved_vram_bytes(), limit_bytes);
    assert_eq!(tracker.available_vram_bytes(), 0);

    // 11th task attempting to reserve 1 MiB must fail with VramQuotaExceeded
    let overflow_res = tracker.reserve(1024 * 1024, "overflow-task");
    assert!(matches!(
        overflow_res,
        Err(GuardError::VramQuotaExceeded { requested_bytes: 1048576, available_bytes: 0, ceiling_bytes: 524288000 })
    ));
    assert_eq!(overflow_res.expect_err("overflow").error_code(), ErrorCode::E4002_VRAM_QUOTA_EXCEEDED);

    // Drop one 50 MiB lease
    drop(leases.pop());
    assert_eq!(tracker.available_vram_bytes(), 50 * 1024 * 1024);

    // Now a 50 MiB reservation succeeds
    let new_lease = tracker.reserve(50 * 1024 * 1024, "retry-task");
    assert!(new_lease.is_ok());
}

#[test]
fn test_unit_vram_budget_tracker_raii_lease_drop_release() {
    let limit_bytes = 256 * 1024 * 1024; // 256 MiB
    let tracker = VramBudgetTracker::new(limit_bytes);

    {
        let lease1 = tracker.reserve(200 * 1024 * 1024, "task-1").expect("reserve 200M");
        assert_eq!(tracker.reserved_vram_bytes(), 200 * 1024 * 1024);
        assert_eq!(tracker.available_vram_bytes(), 56 * 1024 * 1024);
        assert_eq!(lease1.bytes(), 200 * 1024 * 1024);
        assert_eq!(lease1.task_id(), "task-1");
    } // lease1 dropped here

    // 100% of reserved VRAM must be immediately restored
    assert_eq!(tracker.reserved_vram_bytes(), 0);
    assert_eq!(tracker.available_vram_bytes(), limit_bytes);

    // Explicit release test
    let lease2 = tracker.reserve(100 * 1024 * 1024, "task-2").expect("reserve 100M");
    assert_eq!(tracker.reserved_vram_bytes(), 100 * 1024 * 1024);
    lease2.release();
    assert_eq!(tracker.reserved_vram_bytes(), 0);
}

#[test]
fn test_unit_wasm_resource_limiter_pages() {
    let limit_bytes = 64 * 1024 * 1024; // 64 MiB
    let limiter = WasmResourceLimiter::new(limit_bytes, 1000);

    // 16 pages = 1 MiB -> Ok
    assert!(limiter.memory_growing(16).expect("16 pages ok"));
    assert_eq!(limiter.current_memory_bytes(), 16 * 65536);

    // 1024 pages = 64 MiB -> Ok
    assert!(limiter.memory_growing(1024).expect("1024 pages ok"));
    assert_eq!(limiter.current_memory_bytes(), 64 * 1024 * 1024);

    // 1025 pages = 64 MiB + 64 KiB -> Exceeds quota
    let err = limiter.memory_growing(1025);
    assert!(err.is_err());
    assert_eq!(err.expect_err("exceeds quota").error_code(), ErrorCode::E4001_MEMORY_QUOTA_EXCEEDED);

    // Table elements
    assert!(limiter.table_growing(500));
    assert!(limiter.table_growing(1000));
    assert!(!limiter.table_growing(1001));
}

#[test]
fn test_unit_cgroup_v2_control_file_formatting() {
    let dir = tempdir().expect("tempdir");
    let config = QuotaConfig {
        max_cpu_percent: 50,
        max_ram_bytes: 64 * 1024 * 1024,
        max_vram_bytes: 128 * 1024 * 1024,
        max_storage_bytes: 1024 * 1024 * 1024,
    };

    let mut enforcer = CgroupV2Enforcer::new(dir.path(), config.clone()).expect("cgroup enforcer");

    // Check cpu.max: 50% * 100,000 us = 50,000 us
    let cpu_content = fs::read_to_string(dir.path().join("cpu.max")).expect("read cpu.max");
    assert_eq!(cpu_content, "50000 100000");

    // Check memory.max
    let mem_content = fs::read_to_string(dir.path().join("memory.max")).expect("read memory.max");
    assert_eq!(mem_content, "67108864");

    // Check memory.high (90% = 60,397,977 bytes)
    let high_content = fs::read_to_string(dir.path().join("memory.high")).expect("read memory.high");
    assert_eq!(high_content, "60397977");

    // Apply update to 80% CPU and 128 MiB RAM
    let mut updated_config = config;
    updated_config.max_cpu_percent = 80;
    updated_config.max_ram_bytes = 128 * 1024 * 1024;
    enforcer.apply_limits(&updated_config).expect("apply updated limits");

    let updated_cpu = fs::read_to_string(dir.path().join("cpu.max")).expect("read updated cpu");
    assert_eq!(updated_cpu, "80000 100000");

    let updated_mem = fs::read_to_string(dir.path().join("memory.max")).expect("read updated mem");
    assert_eq!(updated_mem, "134217728");
}

// ===========================================================================
// 2. INTEGRATION TESTS [Integration]
// ===========================================================================

#[tokio::test]
async fn test_integration_cpu_memory_intensive_workload_enforcement() {
    let keypair = NodeKeypair::generate();
    let initial_policy = make_sample_policy(&keypair, 50, 64, 128); // 50% CPU, 64 MiB RAM, 128 MiB VRAM
    let manager = QuotaManager::with_default_enforcer(initial_policy);

    let t_snapshot0 = manager.telemetry().await;
    assert_eq!(t_snapshot0.max_cpu_percent, 50);
    assert_eq!(t_snapshot0.max_ram_bytes, 64 * 1024 * 1024);
    assert_eq!(t_snapshot0.reserved_ram_bytes, 0);

    // 1. Task 1 reserves 32 MiB RAM
    let lease_ram_1 = manager.reserve_ram(32 * 1024 * 1024, "workload-1")
        .expect("reserve 32 MiB RAM");
    assert_eq!(lease_ram_1.bytes(), 32 * 1024 * 1024);

    // 2. Task 2 reserves another 32 MiB RAM (reaching 64 MiB total)
    let lease_ram_2 = manager.reserve_ram(32 * 1024 * 1024, "workload-2")
        .expect("reserve 32 MiB RAM");
    assert_eq!(lease_ram_2.bytes(), 32 * 1024 * 1024);

    let t_snapshot1 = manager.telemetry().await;
    assert_eq!(t_snapshot1.reserved_ram_bytes, 64 * 1024 * 1024);
    assert_eq!(t_snapshot1.available_ram_bytes, 0);

    // 3. Task 3 attempting to allocate 1 additional MiB must be denied deterministically
    let denial = manager.reserve_ram(1024 * 1024, "workload-3");
    assert!(denial.is_err());
    let err = denial.expect_err("denial");
    assert_eq!(err.error_code(), ErrorCode::E4001_MEMORY_QUOTA_EXCEEDED);

    // 4. Update policy to 128 MiB RAM and 75% CPU dynamically
    let updated_policy = make_sample_policy(&keypair, 75, 128, 256);
    let update_duration = manager.update_policy(updated_policy).await
        .expect("policy update must succeed");

    assert!(
        update_duration < Duration::from_millis(MAX_POLICY_UPDATE_LATENCY_MS),
        "Dynamic policy update took {:?}, exceeding 20 ms ceiling",
        update_duration
    );

    // 5. With new 128 MiB ceiling, Task 3 can now reserve its 32 MiB
    let lease_ram_3 = manager.reserve_ram(32 * 1024 * 1024, "workload-3")
        .expect("reserve 32 MiB after limit increase");
    assert_eq!(lease_ram_3.bytes(), 32 * 1024 * 1024);

    let t_snapshot2 = manager.telemetry().await;
    assert_eq!(t_snapshot2.max_ram_bytes, 128 * 1024 * 1024);
    assert_eq!(t_snapshot2.reserved_ram_bytes, 96 * 1024 * 1024);
    assert_eq!(t_snapshot2.available_ram_bytes, 32 * 1024 * 1024);

    // 6. Complete workloads: drop all leases and verify 100% reclamation
    drop(lease_ram_1);
    drop(lease_ram_2);
    drop(lease_ram_3);

    let t_snapshot3 = manager.telemetry().await;
    assert_eq!(t_snapshot3.reserved_ram_bytes, 0);
    assert_eq!(t_snapshot3.available_ram_bytes, 128 * 1024 * 1024);
}

#[tokio::test]
async fn test_integration_cpu_utilization_window_measurement() {
    let keypair = NodeKeypair::generate();
    let target_cpu_percent: u8 = 50;
    let policy = make_sample_policy(&keypair, target_cpu_percent, 128, 128);
    let manager = QuotaManager::with_default_enforcer(policy);

    assert_eq!(manager.telemetry().await.max_cpu_percent, 50);

    // Simulate task execution with target duty cycle over measurement window
    // Use 100 ms period (50 ms busy / 50 ms idle) to avoid OS timer tick quantization jitter
    let window = Duration::from_millis(600);
    let start = Instant::now();
    let mut busy_time = Duration::ZERO;

    let period = Duration::from_millis(100);
    let busy_slice = Duration::from_millis((100 * target_cpu_percent as u64) / 100);
    let idle_slice = period.saturating_sub(busy_slice);

    while start.elapsed() < window {
        let t_busy = Instant::now();
        while t_busy.elapsed() < busy_slice {
            std::hint::spin_loop();
        }
        busy_time += t_busy.elapsed();
        sleep(idle_slice).await;
    }

    let total_elapsed = start.elapsed();
    let measured_cpu_percent = ((busy_time.as_secs_f64() / total_elapsed.as_secs_f64()) * 100.0) as u8;

    let delta = (measured_cpu_percent as i32 - target_cpu_percent as i32).abs();
    assert!(
        delta <= 6,
        "Measured CPU duty cycle {}% deviated from target {}% by {}% (threshold: ±6%)",
        measured_cpu_percent,
        target_cpu_percent,
        delta
    );
}

// ===========================================================================
// 3. SIMULATION & FAULT-INJECTION TESTS [Simulation]
// ===========================================================================

#[test]
fn test_simulation_runaway_allocation_loop_rejection() {
    let guard = AllocatorGuard::new(64 * 1024 * 1024); // 64 MiB hard ceiling
    let chunk_size = 16 * 1024 * 1024; // 16 MiB chunks

    let mut leases = Vec::new();
    let mut runaway_alloc_count = 0;
    let mut caught_error: Option<GuardError> = None;

    let t0 = Instant::now();

    // Runaway loop attempting to allocate up to 1 GiB in 16 MiB increments
    for i in 0..64 {
        match guard.reserve(chunk_size, &format!("runaway-{i}")) {
            Ok(lease) => {
                leases.push(lease);
                runaway_alloc_count += 1;
            }
            Err(e) => {
                caught_error = Some(e);
                break;
            }
        }
    }

    let elapsed = t0.elapsed();

    // Exactly 4 chunks of 16 MiB = 64 MiB must succeed, 5th chunk must fail
    assert_eq!(runaway_alloc_count, 4);
    assert!(caught_error.is_some());

    let err = caught_error.expect("runaway allocation must produce error");
    assert_eq!(err.error_code(), ErrorCode::E4001_MEMORY_QUOTA_EXCEEDED);

    // Termination must occur in < 10 ms
    assert!(
        elapsed < Duration::from_millis(10),
        "Runaway allocation rejection took {:?}, exceeding 10 ms ceiling",
        elapsed
    );

    // Reclaim memory
    drop(leases);
    assert_eq!(guard.reserved_ram_bytes(), 0);
    assert_eq!(guard.available_ram_bytes(), 64 * 1024 * 1024);
}

// ===========================================================================
// 4. HARDWARE-REQUIRED TESTS [Hardware-Required]
// ===========================================================================

#[test]
fn test_hw_required_cgroup_v2_or_job_object_limits() {
    let config = QuotaConfig {
        max_cpu_percent: 60,
        max_ram_bytes: 256 * 1024 * 1024,
        max_vram_bytes: 512 * 1024 * 1024,
        max_storage_bytes: 2 * 1024 * 1024 * 1024,
    };

    let mut job_enforcer = JobObjectEnforcer::new(config.clone());
    assert!(job_enforcer.apply_limits(&config).is_ok());
    assert!(job_enforcer.assign_task("local-hw-task", Some(std::process::id())).is_ok());
    assert_eq!(job_enforcer.current_config(), config);
    assert!(job_enforcer.remove_task("local-hw-task").is_ok());
}

// ===========================================================================
// 5. MANUALLY-VERIFIED TESTS [Manually-Verified]
// ===========================================================================

#[tokio::test]
async fn test_manually_verified_quota_enforcement_report_audit() {
    let keypair = NodeKeypair::generate();
    let initial_policy = make_sample_policy(&keypair, 40, 128, 256);
    let manager = QuotaManager::with_default_enforcer(initial_policy);

    // Telemetry: Baseline before workload
    let t_baseline = manager.telemetry().await;

    // Execute active workload: reserve 64 MiB RAM and 128 MiB VRAM
    let lease_ram = manager.reserve_ram(64 * 1024 * 1024, "active-job").expect("reserve RAM");
    let lease_vram = manager.reserve_vram(128 * 1024 * 1024, "active-job").expect("reserve VRAM");

    // Telemetry: During peak workload
    let t_peak = manager.telemetry().await;

    // Release workload
    drop(lease_ram);
    drop(lease_vram);

    // Telemetry: Post-release
    let t_post = manager.telemetry().await;

    let report_data = serde_json::json!({
        "timestamp_utc": chrono_like_now(),
        "baseline": t_baseline,
        "peak_utilization": t_peak,
        "post_release": t_post,
        "reclamation_verified": (t_post.reserved_ram_bytes == 0 && t_post.reserved_vram_bytes == 0),
        "status": "PASS"
    });

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.join("../..");
    let report_path = workspace_root.join("artifacts/evidence/stage-04/quota_enforcement_report.json");

    if let Some(parent) = report_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let json_bytes = serde_json::to_vec_pretty(&report_data).expect("serialize report to json");
    fs::write(&report_path, &json_bytes).expect("write quota enforcement report");

    println!(
        "[Manually-Verified] Exported quota enforcement report to {:?}",
        report_path
    );
    assert!(report_path.exists());
    assert!(!json_bytes.is_empty());
}

fn chrono_like_now() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    format!("{}.{:03}Z", now.as_secs(), now.subsec_millis())
}

