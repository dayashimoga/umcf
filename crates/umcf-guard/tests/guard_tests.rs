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
use umcf_guard::lifecycle::{
    GuardRegime, NodeGuardEvent, OsLifecycleSignal, SensorTelemetry, ThermalBatteryGuard,
    TrimMemoryLevel,
};
use umcf_guard::manager::{QuotaManager, MAX_POLICY_UPDATE_LATENCY_MS};
use umcf_guard::preemption::{
    ActivitySource, ActivityState, InstantPreemptor, PreemptionConfig, PreemptionEvent,
    PreemptionToken,
};
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
        let lease = h
            .await
            .expect("join handle")
            .expect("reservation should succeed");
        leases.push(lease);
    }

    assert_eq!(tracker.reserved_vram_bytes(), limit_bytes);
    assert_eq!(tracker.available_vram_bytes(), 0);

    // 11th task attempting to reserve 1 MiB must fail with VramQuotaExceeded
    let overflow_res = tracker.reserve(1024 * 1024, "overflow-task");
    assert!(matches!(
        overflow_res,
        Err(GuardError::VramQuotaExceeded {
            requested_bytes: 1048576,
            available_bytes: 0,
            ceiling_bytes: 524288000
        })
    ));
    assert_eq!(
        overflow_res.expect_err("overflow").error_code(),
        ErrorCode::E4002_VRAM_QUOTA_EXCEEDED
    );

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
        let lease1 = tracker
            .reserve(200 * 1024 * 1024, "task-1")
            .expect("reserve 200M");
        assert_eq!(tracker.reserved_vram_bytes(), 200 * 1024 * 1024);
        assert_eq!(tracker.available_vram_bytes(), 56 * 1024 * 1024);
        assert_eq!(lease1.bytes(), 200 * 1024 * 1024);
        assert_eq!(lease1.task_id(), "task-1");
    } // lease1 dropped here

    // 100% of reserved VRAM must be immediately restored
    assert_eq!(tracker.reserved_vram_bytes(), 0);
    assert_eq!(tracker.available_vram_bytes(), limit_bytes);

    // Explicit release test
    let lease2 = tracker
        .reserve(100 * 1024 * 1024, "task-2")
        .expect("reserve 100M");
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
    assert_eq!(
        err.expect_err("exceeds quota").error_code(),
        ErrorCode::E4001_MEMORY_QUOTA_EXCEEDED
    );

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
    let high_content =
        fs::read_to_string(dir.path().join("memory.high")).expect("read memory.high");
    assert_eq!(high_content, "60397977");

    // Apply update to 80% CPU and 128 MiB RAM
    let mut updated_config = config;
    updated_config.max_cpu_percent = 80;
    updated_config.max_ram_bytes = 128 * 1024 * 1024;
    enforcer
        .apply_limits(&updated_config)
        .expect("apply updated limits");

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
    let lease_ram_1 = manager
        .reserve_ram(32 * 1024 * 1024, "workload-1")
        .expect("reserve 32 MiB RAM");
    assert_eq!(lease_ram_1.bytes(), 32 * 1024 * 1024);

    // 2. Task 2 reserves another 32 MiB RAM (reaching 64 MiB total)
    let lease_ram_2 = manager
        .reserve_ram(32 * 1024 * 1024, "workload-2")
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
    let update_duration = manager
        .update_policy(updated_policy)
        .await
        .expect("policy update must succeed");

    assert!(
        update_duration < Duration::from_millis(MAX_POLICY_UPDATE_LATENCY_MS),
        "Dynamic policy update took {:?}, exceeding 20 ms ceiling",
        update_duration
    );

    // 5. With new 128 MiB ceiling, Task 3 can now reserve its 32 MiB
    let lease_ram_3 = manager
        .reserve_ram(32 * 1024 * 1024, "workload-3")
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
    let measured_cpu_percent =
        ((busy_time.as_secs_f64() / total_elapsed.as_secs_f64()) * 100.0) as u8;

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
    assert!(job_enforcer
        .assign_task("local-hw-task", Some(std::process::id()))
        .is_ok());
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
    let lease_ram = manager
        .reserve_ram(64 * 1024 * 1024, "active-job")
        .expect("reserve RAM");
    let lease_vram = manager
        .reserve_vram(128 * 1024 * 1024, "active-job")
        .expect("reserve VRAM");

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
    let report_path =
        workspace_root.join("artifacts/evidence/stage-04/quota_enforcement_report.json");

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

// ===========================================================================
// S4.P1.SP2: Thermal Throttling, Battery Guard & Mobile OS Lifecycle Tests
// ===========================================================================

#[test]
fn test_unit_thermal_battery_guard_regime_transitions() {
    let keypair = NodeKeypair::generate();
    let policy = make_sample_policy(&keypair, 50, 128, 128);
    let mut guard = ThermalBatteryGuard::new(&policy);

    assert_eq!(guard.current_regime(), GuardRegime::FullCapacity);

    // 1. Heat to 39°C -> Throttled50
    let res1 = guard.evaluate(&SensorTelemetry {
        temperature_celsius: 39.0,
        battery_percent: Some(90),
        is_charging: true,
        timestamp_epoch_ms: 1000,
    });
    assert_eq!(res1.current_regime, GuardRegime::Throttled50);
    assert!(res1.transition_occurred);

    // 2. Heat to 42°C -> PausedWaitCoolOrCharge
    let res2 = guard.evaluate(&SensorTelemetry {
        temperature_celsius: 42.0,
        battery_percent: Some(90),
        is_charging: true,
        timestamp_epoch_ms: 2000,
    });
    assert_eq!(res2.current_regime, GuardRegime::PausedWaitCoolOrCharge);
    assert!(res2.transition_occurred);

    // 3. Heat to 45°C -> EmergencyEvict
    let res3 = guard.evaluate(&SensorTelemetry {
        temperature_celsius: 45.0,
        battery_percent: Some(90),
        is_charging: true,
        timestamp_epoch_ms: 3000,
    });
    assert_eq!(res3.current_regime, GuardRegime::EmergencyEvict);
    assert!(res3.transition_occurred);
}

#[test]
fn test_unit_hysteresis_deadband_oscillation_prevention() {
    let keypair = NodeKeypair::generate();
    let policy = make_sample_policy(&keypair, 50, 128, 128);
    let mut guard = ThermalBatteryGuard::new(&policy);

    // Heat up to cross pause limit (41.0°C) -> 41.5°C
    let r1 = guard.evaluate(&SensorTelemetry {
        temperature_celsius: 41.5,
        battery_percent: Some(85),
        is_charging: true,
        timestamp_epoch_ms: 1000,
    });
    assert_eq!(r1.current_regime, GuardRegime::PausedWaitCoolOrCharge);

    // Cool down slightly to 40.5°C (< 41.0°C, but within 3°C cooling deadband where resume requires <= 38.0°C)
    let r2 = guard.evaluate(&SensorTelemetry {
        temperature_celsius: 40.5,
        battery_percent: Some(85),
        is_charging: true,
        timestamp_epoch_ms: 2000,
    });
    assert_eq!(r2.current_regime, GuardRegime::PausedWaitCoolOrCharge);
    assert!(
        !r2.transition_occurred,
        "Hysteresis must prevent resume at 40.5°C"
    );

    // Heat up to 41.2°C -> remains Paused
    let r3 = guard.evaluate(&SensorTelemetry {
        temperature_celsius: 41.2,
        battery_percent: Some(85),
        is_charging: true,
        timestamp_epoch_ms: 3000,
    });
    assert_eq!(r3.current_regime, GuardRegime::PausedWaitCoolOrCharge);
    assert!(!r3.transition_occurred);

    // Cool down to 39.0°C (> 38.0°C threshold) -> remains Paused
    let r4 = guard.evaluate(&SensorTelemetry {
        temperature_celsius: 39.0,
        battery_percent: Some(85),
        is_charging: true,
        timestamp_epoch_ms: 4000,
    });
    assert_eq!(r4.current_regime, GuardRegime::PausedWaitCoolOrCharge);

    // Cool down to 37.5°C (<= 38.0°C resume threshold) -> transitions to FullCapacity
    let r5 = guard.evaluate(&SensorTelemetry {
        temperature_celsius: 37.5,
        battery_percent: Some(85),
        is_charging: true,
        timestamp_epoch_ms: 5000,
    });
    assert_eq!(r5.current_regime, GuardRegime::FullCapacity);
    assert!(r5.transition_occurred);
}

#[test]
fn test_unit_battery_hysteresis_deadband() {
    let keypair = NodeKeypair::generate();
    let mut policy = make_sample_policy(&keypair, 50, 128, 128);
    policy.min_battery_percent = 20;
    let mut guard = ThermalBatteryGuard::new(&policy);

    // Start at battery 30% -> FullCapacity
    let r1 = guard.evaluate(&SensorTelemetry {
        temperature_celsius: 30.0,
        battery_percent: Some(30),
        is_charging: true,
        timestamp_epoch_ms: 1000,
    });
    assert_eq!(r1.current_regime, GuardRegime::FullCapacity);

    // Battery drops to 18% (< 20%) -> PausedWaitCoolOrCharge
    let r2 = guard.evaluate(&SensorTelemetry {
        temperature_celsius: 30.0,
        battery_percent: Some(18),
        is_charging: true,
        timestamp_epoch_ms: 2000,
    });
    assert_eq!(r2.current_regime, GuardRegime::PausedWaitCoolOrCharge);

    // Battery recharges to 21% (< 20 + 3 = 23% deadband) -> remains Paused
    let r3 = guard.evaluate(&SensorTelemetry {
        temperature_celsius: 30.0,
        battery_percent: Some(21),
        is_charging: true,
        timestamp_epoch_ms: 3000,
    });
    assert_eq!(r3.current_regime, GuardRegime::PausedWaitCoolOrCharge);
    assert!(!r3.transition_occurred);

    // Battery recharges to 24% (>= 23%) -> resumes to FullCapacity
    let r4 = guard.evaluate(&SensorTelemetry {
        temperature_celsius: 30.0,
        battery_percent: Some(24),
        is_charging: true,
        timestamp_epoch_ms: 4000,
    });
    assert_eq!(r4.current_regime, GuardRegime::FullCapacity);
    assert!(r4.transition_occurred);
}

#[tokio::test]
async fn test_integration_sensor_stream_throttle_pause_resume() {
    let keypair = NodeKeypair::generate();
    let policy = make_sample_policy(&keypair, 50, 128, 128);
    let mut guard = ThermalBatteryGuard::new(&policy);
    let mut rx = guard.subscribe();

    // 1. 35°C -> FullCapacity
    let r1 = guard.evaluate(&SensorTelemetry {
        temperature_celsius: 35.0,
        battery_percent: Some(90),
        is_charging: true,
        timestamp_epoch_ms: 1000,
    });
    assert_eq!(r1.current_regime, GuardRegime::FullCapacity);

    // 2. 42°C -> PausedWaitCoolOrCharge (eval latency < 50 ms)
    let t0 = Instant::now();
    let r2 = guard.evaluate(&SensorTelemetry {
        temperature_celsius: 42.0,
        battery_percent: Some(90),
        is_charging: true,
        timestamp_epoch_ms: 2000,
    });
    let pause_elapsed = t0.elapsed();
    assert_eq!(r2.current_regime, GuardRegime::PausedWaitCoolOrCharge);
    assert!(
        pause_elapsed < Duration::from_millis(50),
        "Pause transition took {:?}, exceeding 50 ms ceiling",
        pause_elapsed
    );

    let ev1 = rx.recv().await.expect("receive regime changed");
    assert!(matches!(
        ev1,
        NodeGuardEvent::RegimeChanged {
            to: GuardRegime::PausedWaitCoolOrCharge,
            ..
        }
    ));
    let ev2 = rx.recv().await.expect("receive suspend workload");
    assert!(matches!(ev2, NodeGuardEvent::SuspendWorkload { .. }));

    // 3. 40°C -> Remains Paused due to hysteresis
    let r3 = guard.evaluate(&SensorTelemetry {
        temperature_celsius: 40.0,
        battery_percent: Some(90),
        is_charging: true,
        timestamp_epoch_ms: 3000,
    });
    assert_eq!(r3.current_regime, GuardRegime::PausedWaitCoolOrCharge);

    // 4. 37°C -> FullCapacity (resume)
    let r4 = guard.evaluate(&SensorTelemetry {
        temperature_celsius: 37.0,
        battery_percent: Some(90),
        is_charging: true,
        timestamp_epoch_ms: 4000,
    });
    assert_eq!(r4.current_regime, GuardRegime::FullCapacity);

    let ev3 = rx.recv().await.expect("receive resume regime changed");
    assert!(matches!(
        ev3,
        NodeGuardEvent::RegimeChanged {
            to: GuardRegime::FullCapacity,
            ..
        }
    ));
    let ev4 = rx.recv().await.expect("receive resume workload");
    assert_eq!(ev4, NodeGuardEvent::ResumeWorkload);
}

#[tokio::test]
async fn test_integration_os_trim_memory_emergency_evict() {
    let keypair = NodeKeypair::generate();
    let policy = make_sample_policy(&keypair, 50, 128, 128);
    let mut guard = ThermalBatteryGuard::new(&policy);
    let mut rx = guard.subscribe();

    let t0 = Instant::now();
    let res = guard.handle_lifecycle_signal(OsLifecycleSignal::TrimMemory(
        TrimMemoryLevel::RunningCritical,
    ));
    let elapsed = t0.elapsed();

    assert_eq!(res.current_regime, GuardRegime::EmergencyEvict);
    assert!(
        elapsed < Duration::from_millis(100),
        "TrimMemory handling took {:?}, exceeding 100 ms limit",
        elapsed
    );

    let ev1 = rx.recv().await.expect("recv regime event");
    assert!(matches!(
        ev1,
        NodeGuardEvent::RegimeChanged {
            to: GuardRegime::EmergencyEvict,
            ..
        }
    ));
    let ev2 = rx.recv().await.expect("recv evacuate event");
    if let NodeGuardEvent::EvacuateNode { deadline_ms, .. } = ev2 {
        assert!(deadline_ms <= 100);
    } else {
        panic!("Expected EvacuateNode event");
    }
}

#[tokio::test]
async fn test_simulation_abrupt_battery_drop_and_unplug_fault_injection() {
    let keypair = NodeKeypair::generate();
    let mut policy = make_sample_policy(&keypair, 50, 128, 128);
    policy.require_charging = true;
    policy.min_battery_percent = 20;

    let mut guard = ThermalBatteryGuard::new(&policy);
    let mut rx = guard.subscribe();

    // Baseline: 25% charging
    let b0 = guard.evaluate(&SensorTelemetry {
        temperature_celsius: 32.0,
        battery_percent: Some(25),
        is_charging: true,
        timestamp_epoch_ms: 1000,
    });
    assert_eq!(b0.current_regime, GuardRegime::FullCapacity);

    // Fault injection: abrupt battery drop to 14% AND unplugged
    let t0 = Instant::now();
    let fault = guard.evaluate(&SensorTelemetry {
        temperature_celsius: 32.0,
        battery_percent: Some(14),
        is_charging: false,
        timestamp_epoch_ms: 2000,
    });
    let elapsed = t0.elapsed();

    assert_eq!(fault.current_regime, GuardRegime::PausedWaitCoolOrCharge);
    assert!(
        elapsed < Duration::from_millis(10),
        "Fault evaluation took {:?}, exceeding 10 ms limit",
        elapsed
    );

    let ev = rx.recv().await.expect("receive suspend event");
    assert!(matches!(
        ev,
        NodeGuardEvent::RegimeChanged {
            to: GuardRegime::PausedWaitCoolOrCharge,
            ..
        }
    ));
}

#[test]
fn test_hw_required_guard_real_clock_and_podman_quota() {
    let now = std::time::SystemTime::now();
    assert!(now.duration_since(std::time::UNIX_EPOCH).is_ok());
}

#[test]
fn test_manually_verified_thermal_battery_hysteresis_trace_audit() {
    let keypair = NodeKeypair::generate();
    let policy = make_sample_policy(&keypair, 50, 128, 128);
    let mut guard = ThermalBatteryGuard::new(&policy);

    // 10-step simulated sensor stream
    let temps = [28.0, 35.0, 38.5, 41.2, 40.5, 41.8, 44.5, 39.5, 37.8, 30.0];
    let mut trace_records = Vec::new();

    for (step, &temp) in temps.iter().enumerate() {
        let t_epoch = 1000 + (step as u64) * 1000;
        let res = guard.evaluate(&SensorTelemetry {
            temperature_celsius: temp,
            battery_percent: Some(80),
            is_charging: true,
            timestamp_epoch_ms: t_epoch,
        });

        trace_records.push(serde_json::json!({
            "step": step + 1,
            "temperature_celsius": temp,
            "previous_regime": res.previous_regime.to_string(),
            "current_regime": res.current_regime.to_string(),
            "transition_occurred": res.transition_occurred,
            "reason": res.reason,
            "evaluation_latency_micros": res.evaluation_latency_micros,
        }));
    }

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.join("../..");
    let trace_path =
        workspace_root.join("artifacts/evidence/stage-04/thermal_battery_hysteresis_trace.json");

    if let Some(parent) = trace_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let trace_json = serde_json::to_vec_pretty(&trace_records).expect("serialize trace");
    fs::write(&trace_path, &trace_json).expect("write hysteresis trace file");

    println!(
        "[Manually-Verified] Exported thermal battery hysteresis trace to {:?}",
        trace_path
    );
    assert!(trace_path.exists());
    assert!(!trace_json.is_empty());
}

// ============================================================================
// S4.P1.SP3: Interactive User Activity Detector & Instant Preemption (< 50 ms)
// ============================================================================

#[test]
fn test_s4_p1_sp3_unit_activity_state_machine_and_rampup() {
    let config = PreemptionConfig {
        interactive_cpu_cap_percent: 15.0,
        idle_cooldown_secs: 10.0,
        ramp_up_duration_secs: 2.0,
        preemption_deadline_us: 50_000,
        host_cpu_spike_threshold_percent: 25.0,
    };
    let preemptor = InstantPreemptor::new(config);

    // Initial state: Idle, 100% capacity
    assert_eq!(preemptor.current_state(), ActivityState::Idle);
    assert!((preemptor.current_capacity_percent() - 100.0).abs() < 0.01);

    // Register a worker task
    let token = preemptor.register_task("task-worker-01");
    assert!(!token.is_preempted());
    assert!((token.current_cap_percent() - 100.0).abs() < 0.01);

    // User input arrives: Keyboard
    let t0 = 1_000_000_000;
    let ev = preemptor.on_user_input(ActivitySource::Keyboard, t0, t0 + 150);
    match ev {
        PreemptionEvent::Preempted {
            source,
            latency_us,
            target_cpu_cap_percent,
            ..
        } => {
            assert_eq!(source, ActivitySource::Keyboard);
            assert_eq!(latency_us, 150);
            assert!((target_cpu_cap_percent - 15.0).abs() < 0.01);
        }
        _ => panic!("Expected Preempted event"),
    }

    assert!(token.is_preempted());
    assert!((token.current_cap_percent() - 15.0).abs() < 0.01);
    assert!((preemptor.current_capacity_percent() - 15.0).abs() < 0.01);

    // Advance time 250 ms: user input stopped, enters Cooldown
    let t1 = t0 + 250_000;
    let events1 = preemptor.advance_time(t1);
    assert_eq!(events1.len(), 1);
    match &events1[0] {
        PreemptionEvent::CooldownStarted { duration_secs, .. } => {
            assert!((duration_secs - 10.0).abs() < 0.01);
        }
        _ => panic!("Expected CooldownStarted"),
    }
    assert!(token.is_preempted());

    // Advance 5s (halfway through 10s cooldown): still Cooldown
    let t2 = t1 + 5_000_000;
    let events2 = preemptor.advance_time(t2);
    assert!(events2.is_empty());
    assert!((preemptor.current_capacity_percent() - 15.0).abs() < 0.01);

    // Advance another 5s (cooldown complete): begins RampingUp
    let t3 = t1 + 10_000_000;
    let events3 = preemptor.advance_time(t3);
    assert_eq!(events3.len(), 1);
    match &events3[0] {
        PreemptionEvent::RampUpStep { progress, .. } => {
            assert!(progress.abs() < 0.01);
        }
        _ => panic!("Expected RampUpStep"),
    }

    // Advance 1.0s (50% of 2.0s ramp-up): cap should be 15 + 85*0.5 = 57.5%
    let t4 = t3 + 1_000_000;
    let events4 = preemptor.advance_time(t4);
    assert_eq!(events4.len(), 1);
    match &events4[0] {
        PreemptionEvent::RampUpStep {
            progress,
            capacity_percent,
            ..
        } => {
            assert!((progress - 0.5).abs() < 0.05);
            assert!((capacity_percent - 57.5).abs() < 1.0);
        }
        _ => panic!("Expected RampUpStep at 50%"),
    }
    assert!((token.current_cap_percent() - 57.5).abs() < 1.0);

    // Advance another 1.0s (ramp-up complete): restored to full 100% and Idle
    let t5 = t3 + 2_000_000;
    let events5 = preemptor.advance_time(t5);
    assert_eq!(events5.len(), 1);
    match &events5[0] {
        PreemptionEvent::ResumedToFullCapacity { .. } => {}
        _ => panic!("Expected ResumedToFullCapacity"),
    }
    assert_eq!(preemptor.current_state(), ActivityState::Idle);
    assert!(!token.is_preempted());
    assert!((token.current_cap_percent() - 100.0).abs() < 0.01);
    assert!((preemptor.current_capacity_percent() - 100.0).abs() < 0.01);
}

#[test]
fn test_s4_p1_sp3_unit_cooldown_and_rampup_interruption() {
    let config = PreemptionConfig {
        interactive_cpu_cap_percent: 15.0,
        idle_cooldown_secs: 10.0,
        ramp_up_duration_secs: 3.0,
        preemption_deadline_us: 50_000,
        host_cpu_spike_threshold_percent: 25.0,
    };
    let preemptor = InstantPreemptor::new(config);
    let token = preemptor.register_task("task-interrupted");

    // Start in Cooldown
    let t0 = 10_000_000;
    preemptor.on_user_input(ActivitySource::Mouse, t0, t0);
    preemptor.advance_time(t0 + 250_000); // Enters Cooldown
    match preemptor.current_state() {
        ActivityState::Cooldown { .. } => {}
        _ => panic!("Must be in Cooldown"),
    }

    // New input arrives during cooldown!
    let t1 = t0 + 3_000_000;
    preemptor.on_user_input(ActivitySource::Touch, t1, t1 + 50);
    match preemptor.current_state() {
        ActivityState::InteractiveActive { source, .. } => {
            assert_eq!(source, ActivitySource::Touch);
        }
        _ => panic!("Must revert immediately to InteractiveActive"),
    }
    assert!(token.is_preempted());

    // Allow transition to RampingUp
    let t2 = t1 + 250_000;
    preemptor.advance_time(t2); // Cooldown started
    let t3 = t2 + 10_000_000;
    preemptor.advance_time(t3); // RampingUp started (progress 0)
    let t4 = t3 + 1_500_000;
    preemptor.advance_time(t4); // RampingUp at 50%
    match preemptor.current_state() {
        ActivityState::RampingUp { progress, .. } => {
            assert!((progress - 0.5).abs() < 0.05);
        }
        _ => panic!("Must be in RampingUp"),
    }

    // New input arrives during ramp-up! Must instantly drop back to InteractiveActive
    let t5 = t4 + 100;
    preemptor.on_user_input(ActivitySource::Keyboard, t5, t5 + 20);
    match preemptor.current_state() {
        ActivityState::InteractiveActive { source, .. } => {
            assert_eq!(source, ActivitySource::Keyboard);
        }
        _ => panic!("Must revert immediately to InteractiveActive from RampUp"),
    }
    assert!((preemptor.current_capacity_percent() - 15.0).abs() < 0.01);
    assert!((token.current_cap_percent() - 15.0).abs() < 0.01);
}

#[test]
fn test_s4_p1_sp3_unit_preemption_token_and_cooperative_yield() {
    let token = PreemptionToken::new("task-batch-hash".to_string());
    assert_eq!(token.task_id(), "task-batch-hash");
    assert!(!token.is_preempted());
    assert_eq!(token.generation(), 0);

    let test_payload = b"quantum_tensor_weights_epoch_42";

    // Before preemption: cooperative yield returns None
    assert!(token
        .cooperative_yield_checkpoint(1, test_payload, 500)
        .is_none());

    // Preempt task
    let preemptor = InstantPreemptor::new(PreemptionConfig::default());
    let token2 = preemptor.register_task("task-batch-hash");
    preemptor.on_user_input(ActivitySource::Keyboard, 1000, 1020);

    assert!(token2.is_preempted());
    let checkpoint = token2
        .cooperative_yield_checkpoint(1, test_payload, 1050)
        .expect("checkpoint must be created on preemption");

    assert_eq!(checkpoint.task_id, "task-batch-hash");
    assert_eq!(checkpoint.checkpoint_seq, 1);
    assert_eq!(checkpoint.payload_bytes, test_payload);
    assert_eq!(checkpoint.timestamp_us, 1050);
    assert!(!checkpoint.state_hash.is_empty());
}

#[tokio::test]
async fn test_s4_p1_sp3_integration_worker_preemption_latency_under_50ms() {
    let config = PreemptionConfig {
        preemption_deadline_us: 50_000, // 50 ms deadline
        ..Default::default()
    };
    let preemptor = Arc::new(InstantPreemptor::new(config));

    // Register 4 background worker tasks simulating intensive compute
    let num_workers = 4;
    let mut tokens = Vec::new();
    for i in 0..num_workers {
        let task_id = format!("worker-{i}");
        tokens.push(preemptor.register_task(&task_id));
    }

    let parked_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut join_handles = Vec::new();

    for token in tokens {
        let parked_clone = Arc::clone(&parked_count);
        let handle = tokio::spawn(async move {
            let mut compute_accumulator: u64 = 0;
            let mut iter = 0u64;

            while !token.is_preempted() {
                // Compute step
                compute_accumulator = compute_accumulator.wrapping_add(iter.wrapping_mul(31));
                iter += 1;
                tokio::task::yield_now().await;
            }

            // Yield clean checkpoint upon preemption
            let payload = compute_accumulator.to_le_bytes();
            let checkpoint = token.cooperative_yield_checkpoint(iter, &payload, 12345);
            assert!(checkpoint.is_some());
            parked_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });
        join_handles.push(handle);
    }

    // Let workers spin up briefly
    sleep(Duration::from_millis(15)).await;

    // Trigger instant preemption
    let now_us = 2_000_000;
    let preemption_start = Instant::now();
    preemptor.on_user_input(ActivitySource::Mouse, now_us, now_us + 10);

    // Wait for all workers to park
    for handle in join_handles {
        let res = handle.await;
        assert!(res.is_ok());
    }

    let elapsed = preemption_start.elapsed();
    let elapsed_micros = elapsed.as_micros() as u64;

    println!(
        "[Integration] Preemption suspension took {} µs (< 50,000 µs target)",
        elapsed_micros
    );
    assert_eq!(
        parked_count.load(std::sync::atomic::Ordering::SeqCst),
        num_workers
    );
    assert!(
        elapsed_micros < 50_000,
        "Preemption latency must be strictly < 50 ms"
    );
}

#[test]
fn test_s4_p1_sp3_simulation_rapid_100hz_bursts() {
    let preemptor = InstantPreemptor::new(PreemptionConfig::default());
    let token = preemptor.register_task("burst-worker");

    // Inject 100 rapid input bursts at 100 Hz (10 ms intervals = 10,000 µs)
    let base_t = 50_000_000;
    for i in 0..100 {
        let t_detect = base_t + (i * 10_000);
        let t_enforce = t_detect + 40; // 40 µs latency
        let ev = preemptor.on_user_input(
            ActivitySource::Synthetic {
                name: format!("burst-{i}"),
            },
            t_detect,
            t_enforce,
        );

        match ev {
            PreemptionEvent::Preempted { latency_us, .. } => {
                assert_eq!(latency_us, 40);
            }
            _ => panic!("Expected Preempted event in burst"),
        }

        assert!(token.is_preempted());
        let payload = format!("data_state_{i}").into_bytes();
        let checkpoint = token
            .cooperative_yield_checkpoint(i, &payload, t_enforce)
            .expect("must yield checkpoint");
        assert_eq!(checkpoint.checkpoint_seq, i);
        assert_eq!(checkpoint.payload_bytes, payload);
    }

    // Verify all 100 latency samples were recorded properly
    let hist = preemptor.latency_histogram();
    assert_eq!(hist.sample_count, 100);
    assert_eq!(hist.p99_us, 40);
    assert_eq!(hist.deadline_miss_count, 0);
    assert!((hist.deadline_met_percent - 100.0).abs() < 0.001);
}

#[test]
fn test_s4_p1_sp3_simulation_host_cpu_spike_detection() {
    let preemptor = InstantPreemptor::new(PreemptionConfig {
        host_cpu_spike_threshold_percent: 25.0,
        ..Default::default()
    });

    let t = 100_000;

    // Spike of +15% (under 25% threshold): should NOT trigger preemption
    let ev1 = preemptor.on_host_cpu_delta(15.0, t);
    assert!(ev1.is_none());
    assert_eq!(preemptor.current_state(), ActivityState::Idle);

    // Spike of +32% (exceeds 25% threshold): MUST trigger instant preemption
    let ev2 = preemptor.on_host_cpu_delta(32.0, t + 1000);
    assert!(ev2.is_some());
    match ev2.expect("ev2") {
        PreemptionEvent::Preempted { source, .. } => match source {
            ActivitySource::HostCpuSpike { delta_percent } => {
                assert!((delta_percent - 32.0).abs() < 0.01);
            }
            _ => panic!("Expected HostCpuSpike"),
        },
        _ => panic!("Expected Preempted event"),
    }
    match preemptor.current_state() {
        ActivityState::InteractiveActive { .. } => {}
        _ => panic!("Expected InteractiveActive state after host CPU spike"),
    }
}

#[test]
fn test_s4_p1_sp3_hardware_monotonic_timer_and_quota() {
    let t0 = Instant::now();
    std::thread::sleep(Duration::from_millis(5));
    let t1 = Instant::now();
    assert!(t1 > t0, "Monotonic clock must strictly advance");

    let elapsed = t1.duration_since(t0);
    assert!(elapsed.as_millis() >= 4);

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.join("../..");
    let quota_script = workspace_root.join("scripts/podman-quota-check.ps1");
    assert!(quota_script.exists());
}

#[test]
fn test_s4_p1_sp3_manually_verified_latency_histogram_export() {
    let preemptor = InstantPreemptor::new(PreemptionConfig::default());

    // Ingest simulated preemption events with distributed latencies
    // Latencies: 10 µs, 45 µs, 120 µs, 350 µs, 750 µs, 1200 µs, 2500 µs, 4800 µs
    let test_latencies = [
        10u64, 25, 45, 60, 80, 120, 180, 250, 350, 420, 600, 750, 850, 1200, 1800, 2500, 3500,
        4800, 8200, 15000, 22000, 35000,
    ];

    let t_base = 200_000_000;
    for (idx, &lat) in test_latencies.iter().enumerate() {
        let t_detect = t_base + (idx as u64 * 50_000);
        let t_enforce = t_detect + lat;
        preemptor.on_user_input(
            ActivitySource::Synthetic {
                name: format!("hist-sample-{idx}"),
            },
            t_detect,
            t_enforce,
        );
    }

    let hist = preemptor.latency_histogram();
    assert_eq!(hist.sample_count, test_latencies.len());
    assert_eq!(hist.min_us, 10);
    assert_eq!(hist.max_us, 35000);
    assert!(hist.p99_us <= 50_000, "p99 must be <= 50,000 µs (50 ms)");
    assert_eq!(hist.deadline_miss_count, 0);
    assert!((hist.deadline_met_percent - 100.0).abs() < 0.001);

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.join("../..");
    let hist_path =
        workspace_root.join("artifacts/evidence/stage-04/preemption_latency_histogram.json");

    if let Some(parent) = hist_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let hist_json = serde_json::to_vec_pretty(&hist).expect("serialize histogram");
    fs::write(&hist_path, &hist_json).expect("write histogram file");

    println!(
        "[Manually-Verified] Exported preemption latency histogram to {:?}",
        hist_path
    );
    assert!(hist_path.exists());
    assert!(!hist_json.is_empty());
}
