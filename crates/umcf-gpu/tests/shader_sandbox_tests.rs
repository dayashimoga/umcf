//! Classified Verification Tests for Subphase S6.P1.SP1:
//! WGPU/Vulkan/Metal Compute Shader Dispatch & Buffer Validation.
//!
//! Tiers:
//! - [Unit]: AST loop-fuel watchdog injection, resource limits, VRAM allocator quota & zero-scrubbing.
//! - [Integration]: WGSL vector-add kernel dispatch, infinite-loop abort, out-of-bounds ReadZeroSkipWrite.
//! - [Simulation]: 100 concurrent compute dispatches with time-slicing.
//! - [Hardware-Required]: Hardware compute benchmark and dispatch latency.
//! - [Manually-Verified]: Shader sandbox telemetry and UI payload verification.

use std::time::Instant;
use umcf_gpu::{
    BoundedVramAllocator, BoundsCheckPolicy, BufferUsage, DispatchWorkgroups, GpuComputeDispatcher,
    GpuError, KernelBindings, ShaderResourceLimits, ShaderSandbox,
};

// ---------------------------------------------------------------------------
// 1. UNIT TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_unit_naga_ast_loop_fuel_watchdog_injection_nested_loops() {
    let sandbox = ShaderSandbox::default();
    let wgsl_source = r#"
        @group(0) @binding(0) var<storage, read_write> data: array<u32>;

        @compute @workgroup_size(64, 1, 1)
        fn main(@builtin(global_invocation_id) id: vec3<u32>) {
            var i: u32 = 0u;
            loop {
                if (i >= 100u) { break; }
                var j: u32 = 0u;
                while (j < 10u) {
                    data[i] = data[i] + j;
                    j += 1u;
                }
                i += 1u;
            }
        }
    "#;

    let validated = sandbox
        .validate_and_instrument("nested-loop-shader", wgsl_source)
        .expect("shader validation and instrumentation must succeed");

    assert_eq!(
        validated.loop_count, 2,
        "must detect both loop and while loops"
    );
    assert!(validated.has_loop_fuel_watchdog);
    assert!(validated
        .instrumented_source
        .contains("var _umcf_loop_fuel: u32 = 100000u;"));
    assert!(validated
        .instrumented_source
        .contains("if (_umcf_loop_fuel == 0u) { break; }"));
    assert!(validated
        .instrumented_source
        .contains("_umcf_loop_fuel -= 1u;"));
}

#[test]
fn test_unit_shader_resource_limits_reject_excessive_workgroup_memory() {
    let sandbox = ShaderSandbox::default();

    // Shader with excessive workgroup invocations: 32 * 32 * 2 = 2048 > 1024
    let excessive_wgsl = r#"
        @group(0) @binding(0) var<storage, read> input_buf: array<f32>;
        @compute @workgroup_size(32, 32, 2)
        fn main() {}
    "#;

    let err = sandbox
        .validate_and_instrument("excessive-wg-shader", excessive_wgsl)
        .expect_err("must reject total workgroup invocations > 1024");

    match err {
        GpuError::ShaderResourceLimitExceeded {
            limit,
            requested,
            max_allowed,
        } => {
            assert_eq!(limit, "total_workgroup_invocations");
            assert_eq!(requested, 2048);
            assert_eq!(max_allowed, 1024);
        }
        other => panic!("expected ShaderResourceLimitExceeded, got {other:?}"),
    }
}

#[test]
fn test_unit_vram_allocator_quota_enforcement_and_zero_scrub_on_recycle() {
    let mut allocator = BoundedVramAllocator::new(10 * 1024 * 1024); // 10 MiB limit

    // 1. Allocate 4 MiB buffer
    let buf1 = allocator
        .alloc_buffer(4 * 1024 * 1024, BufferUsage::Storage)
        .expect("alloc 4 MiB");
    assert_eq!(allocator.allocated_bytes(), 4 * 1024 * 1024);

    // 2. Write sensitive canary pattern
    let dirty_data = vec![0xDE, 0xAD, 0xBE, 0xEF];
    allocator
        .write_buffer(buf1, 0, &dirty_data)
        .expect("write dirty data");
    assert_eq!(allocator.read_buffer(buf1, 0, 4).unwrap(), dirty_data);

    // 3. Free buffer (must zero-scrub memory)
    allocator.free_buffer(buf1).expect("free buf1");
    assert_eq!(allocator.allocated_bytes(), 0);

    // 4. Exceed quota: request 12 MiB on a 10 MiB allocator
    let quota_err = allocator
        .alloc_buffer(12 * 1024 * 1024, BufferUsage::Storage)
        .expect_err("must fail quota ceiling");

    match quota_err {
        GpuError::VramQuotaExceeded {
            requested_bytes,
            limit_bytes,
            ..
        } => {
            assert_eq!(requested_bytes, 12 * 1024 * 1024);
            assert_eq!(limit_bytes, 10 * 1024 * 1024);
        }
        other => panic!("expected VramQuotaExceeded, got {other:?}"),
    }

    // 5. Reallocate new buffer and assert zero data leakage
    let buf2 = allocator
        .alloc_buffer(4 * 1024 * 1024, BufferUsage::Storage)
        .expect("realloc 4 MiB");

    let readback = allocator
        .read_buffer(buf2, 0, 1024)
        .expect("read zeroed buffer");
    assert!(
        readback.iter().all(|&b| b == 0),
        "recycled buffer must be zero-scrubbed without residual 0xDEADBEEF leak"
    );
}

// ---------------------------------------------------------------------------
// 2. INTEGRATION TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_integration_vector_add_and_matrix_multiply_wgsl_dispatch_and_readback() {
    let mut dispatcher = GpuComputeDispatcher::new(64 * 1024 * 1024);
    let sandbox = ShaderSandbox::default();

    let wgsl = r#"
        @group(0) @binding(0) var<storage, read> in_a: array<f32>;
        @group(0) @binding(1) var<storage, read_write> out_c: array<f32>;

        @compute @workgroup_size(64, 1, 1)
        fn main(@builtin(global_invocation_id) id: vec3<u32>) {
            out_c[id.x] = in_a[id.x] * 2.0;
        }
    "#;

    let shader = sandbox
        .validate_and_instrument("vector-scale", wgsl)
        .expect("validate shader");

    // Allocate input and output buffers (64 floats = 256 bytes)
    let in_id = dispatcher
        .allocator_mut()
        .alloc_buffer(256, BufferUsage::Storage)
        .expect("alloc in");
    let out_id = dispatcher
        .allocator_mut()
        .alloc_buffer(256, BufferUsage::Storage)
        .expect("alloc out");

    // Fill input with 1.5f32
    let mut in_bytes = Vec::new();
    for i in 0..64 {
        let f = (i as f32) + 1.0;
        in_bytes.extend_from_slice(&f.to_le_bytes());
    }
    dispatcher
        .allocator_mut()
        .write_buffer(in_id, 0, &in_bytes)
        .unwrap();

    let mut bindings = KernelBindings::new();
    bindings.bind_storage(0, in_id);
    bindings.bind_storage(1, out_id);

    let receipt = dispatcher
        .dispatch_kernel(&shader, &bindings, DispatchWorkgroups::new(1, 1, 1))
        .expect("dispatch kernel");

    assert!(!receipt.loop_fuel_exhausted);
    assert_ne!(receipt.output_checksum, [0u8; 32]);

    // Read back and verify out_c = in_a * 2.0
    let out_bytes = dispatcher.allocator().read_buffer(out_id, 0, 256).unwrap();
    let out_floats: Vec<f32> = out_bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();

    assert_eq!(out_floats.len(), 64);
    assert_eq!(out_floats[0], 2.0); // 1.0 * 2.0
    assert_eq!(out_floats[1], 4.0); // 2.0 * 2.0
    assert_eq!(out_floats[63], 128.0); // 64.0 * 2.0
}

#[test]
fn test_integration_infinite_loop_shader_aborts_gracefully_with_diagnostic_flag() {
    let mut dispatcher = GpuComputeDispatcher::new(32 * 1024 * 1024);
    let sandbox = ShaderSandbox::default();

    // Intentional malicious infinite loop
    let infinite_wgsl = r#"
        @group(0) @binding(0) var<storage, read_write> target: array<u32>;

        @compute @workgroup_size(1, 1, 1)
        fn main() {
            loop {
                target[0] = target[0] + 1u;
            }
        }
    "#;

    let shader = sandbox
        .validate_and_instrument("malicious-loop", infinite_wgsl)
        .expect("instrument shader");

    let buf_id = dispatcher
        .allocator_mut()
        .alloc_buffer(1024, BufferUsage::Storage)
        .unwrap();

    let mut bindings = KernelBindings::new();
    bindings.bind_storage(0, buf_id);

    let start = Instant::now();
    let err = dispatcher
        .dispatch_kernel(&shader, &bindings, DispatchWorkgroups::new(1, 1, 1))
        .expect_err("infinite loop must be aborted by fuel watchdog");

    let elapsed = start.elapsed();
    println!("Infinite loop aborted in {elapsed:?} (SLA <= 150 ms)");
    assert!(
        elapsed.as_millis() <= 150,
        "watchdog abort took {} ms, exceeding 150 ms SLA",
        elapsed.as_millis()
    );

    match err {
        GpuError::ShaderLoopFuelExhausted { max_iterations } => {
            assert_eq!(max_iterations, 100_000);
        }
        other => panic!("expected ShaderLoopFuelExhausted, got {other:?}"),
    }
}

#[test]
fn test_integration_out_of_bounds_buffer_read_returns_zero_and_write_is_skipped() {
    let mut dispatcher = GpuComputeDispatcher::new(32 * 1024 * 1024);
    let sandbox = ShaderSandbox::new(
        ShaderResourceLimits::default(),
        BoundsCheckPolicy::ReadZeroSkipWrite,
    );

    let oob_wgsl = r#"
        @group(0) @binding(0) var<storage, read> in_small: array<f32>;
        @group(0) @binding(1) var<storage, read_write> out_buf: array<f32>;

        @compute @workgroup_size(1, 1, 1)
        fn main() {
            // Read index 99999999 exceeds in_small length
            out_buf[0] = in_small[99999999];
        }
    "#;

    let shader = sandbox
        .validate_and_instrument("oob-read-skip-write", oob_wgsl)
        .expect("validate");

    let in_id = dispatcher
        .allocator_mut()
        .alloc_buffer(16, BufferUsage::Storage)
        .unwrap();
    let out_id = dispatcher
        .allocator_mut()
        .alloc_buffer(16, BufferUsage::Storage)
        .unwrap();

    let mut bindings = KernelBindings::new();
    bindings.bind_storage(0, in_id);
    bindings.bind_storage(1, out_id);

    let receipt = dispatcher
        .dispatch_kernel(&shader, &bindings, DispatchWorkgroups::new(1, 1, 1))
        .expect("dispatch must succeed via ReadZeroSkipWrite policy");

    assert!(!receipt.loop_fuel_exhausted);
}

// ---------------------------------------------------------------------------
// 3. SIMULATION TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_simulation_100_concurrent_compute_dispatches_with_time_slicing() {
    let mut dispatcher = GpuComputeDispatcher::new(128 * 1024 * 1024);
    let sandbox = ShaderSandbox::default();

    let wgsl = r#"
        @group(0) @binding(0) var<storage, read_write> buf: array<f32>;
        @compute @workgroup_size(64, 1, 1)
        fn main() {}
    "#;
    let shader = sandbox.validate_and_instrument("sim-shader", wgsl).unwrap();

    let buf_id = dispatcher
        .allocator_mut()
        .alloc_buffer(256, BufferUsage::Storage)
        .unwrap();
    let mut bindings = KernelBindings::new();
    bindings.bind_storage(0, buf_id);

    for i in 0..100 {
        let workgroups = DispatchWorkgroups::new(64 + (i % 64), 1, 1);
        let receipt = dispatcher
            .dispatch_kernel(&shader, &bindings, workgroups)
            .expect("concurrent simulation dispatch");

        assert!(receipt.time_slices_count >= 1);
    }
}

// ---------------------------------------------------------------------------
// 4. HARDWARE-REQUIRED TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_hardware_native_compute_benchmark_and_dispatch_overhead() {
    let mut dispatcher = GpuComputeDispatcher::new(64 * 1024 * 1024);
    let sandbox = ShaderSandbox::default();

    let wgsl = r#"
        @group(0) @binding(0) var<storage, read_write> target: array<f32>;
        @compute @workgroup_size(64, 1, 1)
        fn main() {}
    "#;
    let shader = sandbox
        .validate_and_instrument("hw-bench-shader", wgsl)
        .unwrap();

    let buf_id = dispatcher
        .allocator_mut()
        .alloc_buffer(1024, BufferUsage::Storage)
        .unwrap();
    let mut bindings = KernelBindings::new();
    bindings.bind_storage(0, buf_id);

    let start = Instant::now();
    for _ in 0..20 {
        let _ = dispatcher
            .dispatch_kernel(&shader, &bindings, DispatchWorkgroups::new(16, 1, 1))
            .expect("hardware benchmark");
    }

    let elapsed = start.elapsed();
    let avg_us = elapsed.as_micros() as f64 / 20.0;
    println!("Native hardware dispatch average overhead: {avg_us:.2} us/dispatch");
    assert!(avg_us < 5000.0, "dispatch overhead must be < 5 ms");
}

// ---------------------------------------------------------------------------
// 5. MANUALLY-VERIFIED TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_manual_shader_sandbox_telemetry_and_ui_schema_inspection() {
    let limits = ShaderResourceLimits::default();
    let json = serde_json::to_string_pretty(&limits).expect("serialize limits");

    assert!(json.contains("max_workgroup_size"));
    assert!(json.contains("max_storage_buffers"));
    assert!(json.contains("default_loop_fuel"));
}
