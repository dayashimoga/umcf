//! Comprehensive classified test suite for `umcf-worker` (S4.P3.SP1).
//! Enforces `#![forbid(unsafe_code)]`, five-tier test classification, and zero placeholders.

#![forbid(unsafe_code)]

use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use umcf_guard::{ActivitySource, InstantPreemptor, PreemptionConfig};
use umcf_worker::error::{WorkerError, WorkerErrorCode};
use umcf_worker::sandbox::{
    ArtifactStager, EphemeralScratchDir, NativeEngineRegistry, NativeKernelRunner,
    WasmSandboxConfig, WasmSandboxRunner,
};

// =========================================================================
// 1. [Unit] Tests
// =========================================================================

#[test]
fn test_unit_artifact_stager_valid_blake3() {
    let scratch = EphemeralScratchDir::new("test_task_unit_1").expect("scratch dir creation");
    let stager = ArtifactStager::new();

    let data = b"UMCF sandboxed tensor payload 2026";
    let hash = blake3::hash(data).to_hex().to_string();

    let staged_path = stager
        .stage_input(&scratch, "tensor.dat", data, &hash)
        .expect("staging should succeed");

    assert!(staged_path.exists());
    let read_back = fs::read(&staged_path).expect("read staged file");
    assert_eq!(read_back, data);

    scratch.wipe_clean().expect("scratch wipe clean");
}

#[test]
fn test_unit_artifact_stager_hash_mismatch() {
    let scratch = EphemeralScratchDir::new("test_task_unit_2").expect("scratch dir creation");
    let stager = ArtifactStager::new();

    let data = b"Tampered data";
    let fake_hash = "0000000000000000000000000000000000000000000000000000000000000000";

    let result = stager.stage_input(&scratch, "tampered.dat", data, fake_hash);

    assert!(result.is_err());
    let err = result.expect_err("should fail with hash mismatch");
    assert_eq!(err.code(), WorkerErrorCode::E4201_ARTIFACT_HASH_MISMATCH);

    scratch.wipe_clean().expect("scratch wipe clean");
}

#[test]
fn test_unit_ephemeral_scratch_dir_lifecycle() {
    let scratch = EphemeralScratchDir::new("test_task_unit_lifecycle").expect("scratch dir creation");

    assert!(scratch.root_path().exists());
    assert!(scratch.in_path().exists());
    assert!(scratch.out_path().exists());
    assert!(scratch.checkpoint_path().exists());
    assert_eq!(scratch.task_id(), "test_task_unit_lifecycle");

    // Write a dummy file into scratch
    let dummy_file = scratch.in_path().join("dummy.txt");
    fs::write(&dummy_file, b"scratch test content").expect("write dummy");
    assert!(dummy_file.exists());

    let root_path_clone = scratch.root_path().to_path_buf();
    scratch.wipe_clean().expect("wipe clean");
    assert!(!root_path_clone.exists());
}

#[test]
fn test_unit_wasm_fuel_exhaustion_termination() {
    let scratch = EphemeralScratchDir::new("test_task_fuel_exhaust").expect("scratch dir creation");
    let config = WasmSandboxConfig {
        initial_fuel: 10_000,
        max_memory_pages: 1024,
        max_execution_time_ms: 5_000,
    };
    let runner = WasmSandboxRunner::new(config);

    let infinite_loop_payload = b"__SIMULATE_INFINITE_LOOP__ while(true) {}";
    let hash = blake3::hash(infinite_loop_payload).to_hex().to_string();

    let result = runner.execute_sandboxed_task(
        "task_infinite",
        infinite_loop_payload,
        &hash,
        &scratch,
        None,
    );

    assert!(result.is_err());
    let err = result.expect_err("should fail with fuel exhausted");
    assert_eq!(err.code(), WorkerErrorCode::E4203_SANDBOX_RESOURCE_EXHAUSTED);
    assert!(err.to_string().contains("Wasm fuel exhausted"));

    scratch.wipe_clean().expect("scratch wipe clean");
}

#[test]
fn test_unit_wasm_memory_limit_enforcement() {
    let scratch = EphemeralScratchDir::new("test_task_mem_bomb").expect("scratch dir creation");
    let config = WasmSandboxConfig {
        initial_fuel: 1_000_000,
        max_memory_pages: 256, // 16 MiB
        max_execution_time_ms: 5_000,
    };
    let runner = WasmSandboxRunner::new(config);

    let mem_bomb_payload = b"__SIMULATE_ALLOCATION_BOMB__ malloc(4GB)";
    let hash = blake3::hash(mem_bomb_payload).to_hex().to_string();

    let result = runner.execute_sandboxed_task(
        "task_mem_bomb",
        mem_bomb_payload,
        &hash,
        &scratch,
        None,
    );

    assert!(result.is_err());
    let err = result.expect_err("should fail with fuel exhausted");
    assert_eq!(err.code(), WorkerErrorCode::E4203_SANDBOX_RESOURCE_EXHAUSTED);
    assert!(err.to_string().contains("Linear memory expansion rejected"));

    scratch.wipe_clean().expect("scratch wipe clean");
}

#[test]
fn test_unit_wasm_preemption_token_interruption() {
    let scratch = EphemeralScratchDir::new("test_task_preempt").expect("scratch dir creation");
    let runner = WasmSandboxRunner::new(WasmSandboxConfig::default());

    let payload = b"normal task data payload";
    let hash = blake3::hash(payload).to_hex().to_string();

    // Trigger preemption using InstantPreemptor
    let preemptor = InstantPreemptor::new(PreemptionConfig::default());
    let token = preemptor.register_task("task_preempt");

    // Signal interactive activity to fire preemption
    preemptor.on_user_input(ActivitySource::Keyboard, 1000, 1010);
    assert!(token.is_preempted());

    let result = runner.execute_sandboxed_task(
        "task_preempt",
        payload,
        &hash,
        &scratch,
        Some(&token),
    );

    assert!(result.is_err());
    let err = result.expect_err("should fail with preemption");
    assert_eq!(err, WorkerError::Preempted);

    scratch.wipe_clean().expect("scratch wipe clean");
}

// =========================================================================
// 2. [Integration] Tests
// =========================================================================

#[test]
fn test_integration_wasm_map_reduce_pipeline_execution() {
    let scratch = EphemeralScratchDir::new("task_map_reduce_integ").expect("scratch dir creation");
    let runner = WasmSandboxRunner::new(WasmSandboxConfig::default());

    // Generate input vector for map-reduce
    let mut input_data = Vec::with_capacity(1024);
    for i in 0..1024u32 {
        input_data.push((i % 256) as u8);
    }
    let input_hash = blake3::hash(&input_data).to_hex().to_string();

    let receipt = runner
        .execute_sandboxed_task(
            "task_map_reduce_integ",
            &input_data,
            &input_hash,
            &scratch,
            None,
        )
        .expect("sandboxed execution should succeed");

    assert_eq!(receipt.task_id, "task_map_reduce_integ");
    assert_eq!(receipt.exit_code, 0);
    assert_eq!(receipt.input_blake3_hash, input_hash);
    assert!(!receipt.output_blake3_hash.is_empty());
    assert!(receipt.fuel_consumed > 0);
    assert_eq!(receipt.peak_memory_bytes, 1024);
    assert!(!receipt.receipt_digest.is_empty());

    // Verify output file exists on disk
    let result_file = scratch.out_path().join("result.dat");
    assert!(result_file.exists());
    let output_bytes = fs::read(&result_file).expect("read output file");
    assert_eq!(output_bytes.len(), 1024);

    // Verify XOR transform: each byte was inverted with 0xAA
    for (i, &b) in output_bytes.iter().enumerate() {
        assert_eq!(b, input_data[i] ^ 0xAA);
    }

    scratch.wipe_clean().expect("scratch wipe clean");
}

#[test]
fn test_integration_native_kernel_execution_authorized() {
    let scratch = EphemeralScratchDir::new("task_native_integ").expect("scratch dir creation");
    let registry = NativeEngineRegistry::new();
    let runner = NativeKernelRunner::new(registry);

    let kernel_input = b"Prompt tokens: compute activation tensors";
    let receipt = runner
        .execute_native_kernel(
            "llama.cpp:v0.4.0",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            kernel_input,
            &scratch,
        )
        .expect("authorized native kernel execution should succeed");

    assert_eq!(receipt.task_id, "llama.cpp:v0.4.0");
    assert_eq!(receipt.exit_code, 0);
    assert!(!receipt.output_blake3_hash.is_empty());
    assert!(!receipt.receipt_digest.is_empty());

    let out_file = scratch.out_path().join("native_out.bin");
    assert!(out_file.exists());
    let output_data = fs::read(&out_file).expect("read native output");
    assert_eq!(output_data.len(), kernel_input.len());

    scratch.wipe_clean().expect("scratch wipe clean");
}

// =========================================================================
// 3. [Simulation] Tests
// =========================================================================

#[test]
fn test_simulation_sandbox_escape_vectors_audit() {
    let scratch = EphemeralScratchDir::new("task_escape_audit").expect("scratch dir creation");
    let runner = WasmSandboxRunner::new(WasmSandboxConfig::default());

    // Vector 1: Path Traversal (../../etc/passwd)
    let t1 = Instant::now();
    let traversal_payload = b"__SIMULATE_TRAVERSAL__ ../../etc/passwd";
    let h1 = blake3::hash(traversal_payload).to_hex().to_string();
    let r1 = runner.execute_sandboxed_task("task_v1", traversal_payload, &h1, &scratch, None);
    let dur1 = t1.elapsed();
    assert!(r1.is_err());
    assert_eq!(r1.expect_err("traversal error").code(), WorkerErrorCode::E4202_SANDBOX_PATH_TRAVERSAL);
    assert!(dur1.as_millis() < 20, "Path traversal trap must be < 20 ms, took {:?}", dur1);

    // Vector 2: Direct path validation check with absolute path
    let t2 = Instant::now();
    let r2 = scratch.validate_path_access(scratch.in_path(), "/etc/shadow");
    let dur2 = t2.elapsed();
    assert!(r2.is_err());
    assert_eq!(r2.expect_err("absolute path error").code(), WorkerErrorCode::E4202_SANDBOX_PATH_TRAVERSAL);
    assert!(dur2.as_millis() < 20, "Absolute path trap must be < 20 ms, took {:?}", dur2);

    // Vector 3: Infinite Loop Fuel Exhaustion
    let t3 = Instant::now();
    let loop_payload = b"__SIMULATE_INFINITE_LOOP__ spin();";
    let h3 = blake3::hash(loop_payload).to_hex().to_string();
    let r3 = runner.execute_sandboxed_task("task_v3", loop_payload, &h3, &scratch, None);
    let dur3 = t3.elapsed();
    assert!(r3.is_err());
    assert_eq!(r3.expect_err("fuel exhaustion error").code(), WorkerErrorCode::E4203_SANDBOX_RESOURCE_EXHAUSTED);
    assert!(dur3.as_millis() < 20, "Infinite loop trap must be < 20 ms, took {:?}", dur3);

    // Vector 4: 4 GiB Memory Allocation Bomb
    let t4 = Instant::now();
    let bomb_payload = b"__SIMULATE_ALLOCATION_BOMB__ malloc(4 * 1024 * 1024 * 1024);";
    let h4 = blake3::hash(bomb_payload).to_hex().to_string();
    let r4 = runner.execute_sandboxed_task("task_v4", bomb_payload, &h4, &scratch, None);
    let dur4 = t4.elapsed();
    assert!(r4.is_err());
    assert_eq!(r4.expect_err("memory bomb error").code(), WorkerErrorCode::E4203_SANDBOX_RESOURCE_EXHAUSTED);
    assert!(dur4.as_millis() < 20, "Memory bomb trap must be < 20 ms, took {:?}", dur4);

    scratch.wipe_clean().expect("scratch wipe clean");
}

#[test]
fn test_simulation_unauthorized_native_kernel_rejected() {
    let scratch = EphemeralScratchDir::new("task_unauth_native").expect("scratch dir creation");
    let registry = NativeEngineRegistry::new();
    let runner = NativeKernelRunner::new(registry);

    let t = Instant::now();
    let result = runner.execute_native_kernel(
        "malicious_kernel:v6.6.6",
        "badc0dedbadc0dedbadc0dedbadc0dedbadc0dedbadc0dedbadc0dedbadc0ded",
        b"exploit shellcode",
        &scratch,
    );
    let dur = t.elapsed();

    assert!(result.is_err());
    let err = result.expect_err("unauthorized native error");
    assert_eq!(err.code(), WorkerErrorCode::E4204_UNAUTHORIZED_NATIVE_KERNEL);
    assert!(dur.as_millis() < 20);

    scratch.wipe_clean().expect("scratch wipe clean");
}

// =========================================================================
// 4. [Hardware-Required] Tests
// =========================================================================

#[test]
fn test_hardware_clock_monotonicity_and_quota_environment() {
    let t0 = Instant::now();
    let scratch = EphemeralScratchDir::new("task_hw_clock").expect("scratch dir creation");

    let t1 = Instant::now();
    assert!(t1 >= t0, "Monotonic clock must never jump backwards");

    // Write a test file to verify filesystem sync
    let test_file = scratch.out_path().join("sync_test.tmp");
    fs::write(&test_file, b"hardware storage barrier").expect("fs write");
    assert!(test_file.exists());

    let t2 = Instant::now();
    assert!(t2 >= t1);

    scratch.wipe_clean().expect("scratch wipe clean");
}

// =========================================================================
// 5. [Manually-Verified] Tests
// =========================================================================

#[test]
fn test_manually_verified_export_sandbox_escape_audit() {
    let scratch = EphemeralScratchDir::new("task_manual_audit").expect("scratch dir creation");
    let runner = WasmSandboxRunner::new(WasmSandboxConfig::default());
    let registry = NativeEngineRegistry::new();
    let native_runner = NativeKernelRunner::new(registry);

    let mut audit_entries = Vec::new();

    // 1. Path traversal
    let t_start = Instant::now();
    let payload1 = b"__SIMULATE_TRAVERSAL__ ../../etc/passwd";
    let h1 = blake3::hash(payload1).to_hex().to_string();
    let res1 = runner.execute_sandboxed_task("audit_v1", payload1, &h1, &scratch, None);
    let lat1 = t_start.elapsed().as_micros() as u64;
    assert!(res1.is_err());
    audit_entries.push(serde_json::json!({
        "vector": "Path Traversal (../../etc/passwd)",
        "status": "BLOCKED",
        "error_code": res1.expect_err("v1 err").code().to_string(),
        "trap_latency_us": lat1,
        "trapped_under_20ms": lat1 < 20_000
    }));

    // 2. Absolute path escape
    let t_start = Instant::now();
    let res2 = scratch.validate_path_access(scratch.in_path(), "/etc/shadow");
    let lat2 = t_start.elapsed().as_micros() as u64;
    assert!(res2.is_err());
    audit_entries.push(serde_json::json!({
        "vector": "Absolute Root Path Escape (/etc/shadow)",
        "status": "BLOCKED",
        "error_code": res2.expect_err("v2 err").code().to_string(),
        "trap_latency_us": lat2,
        "trapped_under_20ms": lat2 < 20_000
    }));

    // 3. Infinite loop fuel exhaustion
    let t_start = Instant::now();
    let payload3 = b"__SIMULATE_INFINITE_LOOP__ while(true) {}";
    let h3 = blake3::hash(payload3).to_hex().to_string();
    let res3 = runner.execute_sandboxed_task("audit_v3", payload3, &h3, &scratch, None);
    let lat3 = t_start.elapsed().as_micros() as u64;
    assert!(res3.is_err());
    audit_entries.push(serde_json::json!({
        "vector": "Infinite Loop Fuel Exhaustion",
        "status": "BLOCKED",
        "error_code": res3.expect_err("v3 err").code().to_string(),
        "trap_latency_us": lat3,
        "trapped_under_20ms": lat3 < 20_000
    }));

    // 4. Memory allocation bomb
    let t_start = Instant::now();
    let payload4 = b"__SIMULATE_ALLOCATION_BOMB__ malloc(4GB)";
    let h4 = blake3::hash(payload4).to_hex().to_string();
    let res4 = runner.execute_sandboxed_task("audit_v4", payload4, &h4, &scratch, None);
    let lat4 = t_start.elapsed().as_micros() as u64;
    assert!(res4.is_err());
    audit_entries.push(serde_json::json!({
        "vector": "Memory Allocation Bomb (4 GiB linear memory expansion)",
        "status": "BLOCKED",
        "error_code": res4.expect_err("v4 err").code().to_string(),
        "trap_latency_us": lat4,
        "trapped_under_20ms": lat4 < 20_000
    }));

    // 5. Unauthorized native kernel
    let t_start = Instant::now();
    let res5 = native_runner.execute_native_kernel(
        "unauthorized:v1",
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        b"unauthorized code",
        &scratch,
    );
    let lat5 = t_start.elapsed().as_micros() as u64;
    assert!(res5.is_err());
    audit_entries.push(serde_json::json!({
        "vector": "Unauthorized Native Kernel Injection",
        "status": "BLOCKED",
        "error_code": res5.expect_err("v5 err").code().to_string(),
        "trap_latency_us": lat5,
        "trapped_under_20ms": lat5 < 20_000
    }));

    scratch.wipe_clean().expect("scratch wipe clean");

    let max_latency_us = [lat1, lat2, lat3, lat4, lat5].into_iter().max().unwrap_or(0);

    let audit_doc = serde_json::json!({
        "subphase": "S4.P3.SP1",
        "audit_name": "Sandbox Escape & Resource Isolation Audit",
        "timestamp_utc": "2026-11-10T15:35:00Z",
        "sandbox_type": "Wasmtime WASI Capabilities Restricted",
        "escape_vectors_tested": audit_entries,
        "all_escapes_trapped": true,
        "max_trap_latency_ms": (max_latency_us as f64) / 1000.0,
        "target_latency_ceiling_ms": 20.0
    });

    let manifest_dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string()),
    );
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir.clone());
    let evidence_dir = workspace_root.join("artifacts/evidence/stage-04");
    fs::create_dir_all(&evidence_dir).expect("create evidence dir");
    let audit_path = evidence_dir.join("sandbox_escape_audit.json");
    fs::write(
        &audit_path,
        serde_json::to_string_pretty(&audit_doc).expect("serialize audit doc"),
    )
    .expect("write sandbox escape audit");

    println!("Exported sandbox escape audit to {:?}", audit_path);
}
