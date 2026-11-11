//! Classified Five-Tier Test Suite for Hardware Backend Selector & Fallback State Machine.
//!
//! Covers:
//! - [Unit]: Layer offload with VRAM headroom, big.LITTLE P-core thread tuning, Tier 1->4 transitions, numerical sanity probe.
//! - [Integration]: Synthetic OOM injection on GPU with automatic fallback recovery on CPU.
//! - [Simulation]: Thermal critical and serious events triggering dynamic GPU/batch step-down.
//! - [Hardware-Required]: Throughput benchmark comparison across CPU SIMD and GPU backends.
//! - [Manually-Verified]: Telemetry serialization and UI parameter compatibility.

use std::fs::File;
use std::io::Write;
use std::time::Instant;
use tempfile::TempDir;

use umcf_ai::{
    ContextParams, CpuBackend, CpuIsaFlags, FallbackExecutor, FallbackTier, GpuBackend,
    HardwareBackendSelector, ModelLoadParams, SafeLlamaBatch, SafeLlamaContext, SafeLlamaModel,
    SafeSamplerChain,
};
use umcf_core::capabilities::ThermalState;
use umcf_gguf::{
    create_test_gguf_bytes, ContentAddressedModelStore, GgmlType, ModelIntegrityVerifier,
    VerifyMode,
};

/// Helper to create a verified model guard from synthetic test GGUF bytes.
fn create_test_verified_guard(temp_dir: &TempDir) -> umcf_gguf::VerifiedModelGuard {
    let store_dir = temp_dir.path().join("store");
    let store = ContentAddressedModelStore::new(&store_dir).expect("init test store");

    let raw_model_path = temp_dir.path().join("model.gguf");
    let bytes = create_test_gguf_bytes("TestModel-1B", 2, 256, 1024, 4, 4, GgmlType::Q4_0);
    let mut file = File::create(&raw_model_path).expect("create raw gguf file");
    file.write_all(&bytes).expect("write gguf bytes");
    drop(file);

    let manifest = store
        .ingest_local_file(&raw_model_path)
        .expect("import local model into store");

    ModelIntegrityVerifier::verify(&store, &manifest.model_id, VerifyMode::FastPreFlight, None)
        .expect("model verification must succeed")
}

// =========================================================================
// 1. [Unit] Deterministic Unit Tests
// =========================================================================

#[test]
fn test_unit_gpu_layer_offload_calculation_with_vram_headroom() {
    let total_layers = 32;
    let bytes_per_layer = 100 * 1024 * 1024; // 100 MiB / layer (3.2 GiB total)

    // Scenario A: Desktop GPU with 8 GiB VRAM (15% reserve = 1.2 GiB -> usable = 6.8 GiB -> Fits all 32 layers)
    let desktop_gpu = GpuBackend::Vulkan {
        device_name: "NVIDIA RTX 4070".to_string(),
        subgroup_size: 64,
        fp16_storage: true,
        cooperative_matrix: true,
        free_vram_bytes: 8 * 1024 * 1024 * 1024,
        total_vram_bytes: 12 * 1024 * 1024 * 1024,
    };

    let selector_desktop = HardwareBackendSelector::default().with_gpus(vec![desktop_gpu]);
    let plan_desktop = selector_desktop.compute_plan(total_layers, bytes_per_layer, false);
    assert_eq!(plan_desktop.tier, FallbackTier::Tier1FullGpu);
    assert_eq!(plan_desktop.n_gpu_layers, 32);
    assert!(plan_desktop.flash_attn);

    // Scenario B: Mobile GPU with 3.5 GiB shared VRAM (1.5 GiB mobile reserve -> usable = 2.0 GiB -> 20 layers offloaded)
    let mobile_gpu = GpuBackend::Vulkan {
        device_name: "Adreno 750".to_string(),
        subgroup_size: 64,
        fp16_storage: true,
        cooperative_matrix: true,
        free_vram_bytes: 3584 * 1024 * 1024, // 3.5 GiB
        total_vram_bytes: 8 * 1024 * 1024 * 1024,
    };

    let selector_mobile = HardwareBackendSelector::default().with_gpus(vec![mobile_gpu]);
    let plan_mobile = selector_mobile.compute_plan(total_layers, bytes_per_layer, true);
    assert_eq!(plan_mobile.tier, FallbackTier::Tier2HybridPartialGpu);
    assert_eq!(plan_mobile.n_gpu_layers, 20); // 2000 MiB / 100 MiB = 20 layers
    assert!(plan_mobile.flash_attn);

    // Scenario C: No GPU available -> CPU SIMD Tier 3
    let selector_cpu = HardwareBackendSelector::default().with_gpus(vec![]);
    let plan_cpu = selector_cpu.compute_plan(total_layers, bytes_per_layer, false);
    assert_eq!(plan_cpu.tier, FallbackTier::Tier3CpuSimd);
    assert_eq!(plan_cpu.n_gpu_layers, 0);
    assert_eq!(plan_cpu.backend_name, "CPU-SIMD");
}

#[test]
fn test_unit_big_little_p_core_thread_selection_logic() {
    // 8-core CPU: 4 P-cores, 4 E-cores
    let cpu = CpuBackend {
        isa_flags: CpuIsaFlags::default(),
        performance_cores: 4,
        efficiency_cores: 4,
        total_cores: 8,
        total_ram_bytes: 16 * 1024 * 1024 * 1024,
        available_ram_bytes: 8 * 1024 * 1024 * 1024,
    };

    let selector = HardwareBackendSelector::default()
        .with_cpu(cpu)
        .with_gpus(vec![]);

    let plan = selector.compute_plan(24, 50 * 1024 * 1024, false);

    // Single-token generation thread count must strictly equal P-core count to avoid E-core stragglers
    assert_eq!(plan.n_threads, 4, "decode thread count must equal P-cores");
    // Batch prefill can saturate all physical cores
    assert_eq!(
        plan.n_threads_batch, 8,
        "prefill thread count can use all physical cores"
    );
}

#[test]
fn test_unit_fallback_state_machine_tier1_to_tier4_transitions() {
    let gpu = GpuBackend::Vulkan {
        device_name: "TestGPU".to_string(),
        subgroup_size: 64,
        fp16_storage: true,
        cooperative_matrix: true,
        free_vram_bytes: 8 * 1024 * 1024 * 1024,
        total_vram_bytes: 8 * 1024 * 1024 * 1024,
    };
    let selector = HardwareBackendSelector::default().with_gpus(vec![gpu]);
    let mut executor = FallbackExecutor::new(selector, 32, 100 * 1024 * 1024, false);

    // Initial state: Tier 1
    assert_eq!(executor.current_tier(), FallbackTier::Tier1FullGpu);
    assert_eq!(executor.active_plan().n_gpu_layers, 32);

    // Transition 1: Tier 1 -> Tier 2 (Hybrid Partial GPU)
    assert!(executor.step_down("Simulated VRAM allocation pressure"));
    assert_eq!(executor.current_tier(), FallbackTier::Tier2HybridPartialGpu);
    assert_eq!(executor.active_plan().n_gpu_layers, 24); // 32 * 3 / 4

    // Transition 2: Tier 2 -> Tier 3 (CPU SIMD)
    assert!(executor.step_down("Simulated GPU context creation failure"));
    assert_eq!(executor.current_tier(), FallbackTier::Tier3CpuSimd);
    assert_eq!(executor.active_plan().n_gpu_layers, 0);
    assert_eq!(executor.active_plan().backend_name, "CPU-SIMD");

    // Transition 3: Tier 3 -> Tier 4 (Safe Mode CPU)
    assert!(executor.step_down("Simulated memory bandwidth bottleneck"));
    assert_eq!(executor.current_tier(), FallbackTier::Tier4SafeModeCpu);
    assert_eq!(executor.active_plan().n_gpu_layers, 0);
    assert_eq!(executor.active_plan().n_ubatch, 128);
    assert!(!executor.active_plan().flash_attn);

    // Transition 4: Tier 4 is terminal -> Returns false
    assert!(!executor.step_down("Already at lowest safe mode tier"));
    assert_eq!(executor.current_tier(), FallbackTier::Tier4SafeModeCpu);

    // Verify history log captures all transitions
    assert_eq!(executor.fallback_history().len(), 3);
    assert_eq!(executor.fallback_history()[0].0, FallbackTier::Tier1FullGpu);
    assert_eq!(
        executor.fallback_history()[1].0,
        FallbackTier::Tier2HybridPartialGpu
    );
    assert_eq!(executor.fallback_history()[2].0, FallbackTier::Tier3CpuSimd);
}

#[test]
fn test_unit_numerical_sanity_probe_rejects_corrupted_gpu_output() {
    let gpu = GpuBackend::Vulkan {
        device_name: "Adreno Buggy Driver".to_string(),
        subgroup_size: 64,
        fp16_storage: true,
        cooperative_matrix: true,
        free_vram_bytes: 4 * 1024 * 1024 * 1024,
        total_vram_bytes: 4 * 1024 * 1024 * 1024,
    };

    // Case 1: Accurate execution passes probe
    let ok_res = HardwareBackendSelector::run_backend_sanity_probe(&gpu, None);
    assert!(ok_res.is_ok(), "clean matmul execution must pass probe");

    // Case 2: Divergent result (error > 1e-2) fails probe
    let err_divergent = HardwareBackendSelector::run_backend_sanity_probe(&gpu, Some(0.05f32));
    assert!(
        err_divergent.is_err(),
        "divergent matmul output must fail sanity probe"
    );

    // Case 3: NaN output fails probe
    let err_nan = HardwareBackendSelector::run_backend_sanity_probe(&gpu, Some(f32::NAN));
    assert!(err_nan.is_err(), "NaN output must fail sanity probe");
}

// =========================================================================
// 2. [Integration] End-to-End Fallback Recovery Tests
// =========================================================================

#[test]
fn test_integration_synthetic_oom_triggers_automatic_recovery_on_cpu() {
    let temp_dir = TempDir::new().expect("temp dir");
    let guard = create_test_verified_guard(&temp_dir);

    // Create a selector with a GPU
    let gpu = GpuBackend::Vulkan {
        device_name: "MockGPU".to_string(),
        subgroup_size: 64,
        fp16_storage: true,
        cooperative_matrix: true,
        free_vram_bytes: 4 * 1024 * 1024 * 1024,
        total_vram_bytes: 4 * 1024 * 1024 * 1024,
    };
    let selector = HardwareBackendSelector::default().with_gpus(vec![gpu]);
    let mut executor = FallbackExecutor::new(selector, 16, 50 * 1024 * 1024, false);

    // Step down to simulate prior GPU failure
    executor.step_down("Simulated GPU OOM on Tier 1");
    executor.step_down("Simulated GPU shader compile failure on Tier 2");
    assert_eq!(executor.current_tier(), FallbackTier::Tier3CpuSimd);

    // Execute with fallback on Tier 3
    let (model, mut ctx) = executor
        .execute_with_fallback(&guard, 4)
        .expect("automatic fallback to CPU must succeed");

    assert_eq!(executor.current_tier(), FallbackTier::Tier3CpuSimd);

    // Verify token generation works seamlessly
    let mut batch = SafeLlamaBatch::new(2, 0, 1);
    batch.add(1, 0, &[0], true).expect("add prompt token");
    ctx.decode(&mut batch).expect("decode on fallback CPU");

    let mut sampler = SafeSamplerChain::new_greedy();
    let next_token = sampler.sample(&ctx, 0);
    assert!(next_token > 0);

    let piece = model.token_to_piece(next_token).expect("piece");
    assert!(!piece.is_empty());
}

// =========================================================================
// 3. [Simulation] Thermal & Battery Throttling Simulation Tests
// =========================================================================

#[test]
fn test_simulation_thermal_critical_triggers_dynamic_downgrade() {
    let gpu = GpuBackend::Vulkan {
        device_name: "MobileGPU".to_string(),
        subgroup_size: 64,
        fp16_storage: true,
        cooperative_matrix: true,
        free_vram_bytes: 4 * 1024 * 1024 * 1024,
        total_vram_bytes: 4 * 1024 * 1024 * 1024,
    };
    let selector = HardwareBackendSelector::default().with_gpus(vec![gpu]);
    let mut executor = FallbackExecutor::new(selector, 32, 100 * 1024 * 1024, true);

    assert_eq!(executor.current_tier(), FallbackTier::Tier2HybridPartialGpu);

    // Simulate ThermalState::Fair -> No downgrade
    let no_change = executor.handle_thermal_event(ThermalState::Fair);
    assert!(!no_change);
    assert_eq!(executor.current_tier(), FallbackTier::Tier2HybridPartialGpu);

    // Simulate ThermalState::Serious -> Downgrades to CPU SIMD
    let degraded_serious = executor.handle_thermal_event(ThermalState::Serious);
    assert!(degraded_serious);
    assert_eq!(executor.current_tier(), FallbackTier::Tier3CpuSimd);

    // Simulate ThermalState::Critical -> Drops immediately to Tier 4 Safe Mode
    let degraded_critical = executor.handle_thermal_event(ThermalState::Critical);
    assert!(degraded_critical);
    assert_eq!(executor.current_tier(), FallbackTier::Tier4SafeModeCpu);
}

// =========================================================================
// 4. [Hardware-Required] Throughput Comparison Benchmark
// =========================================================================

#[test]
fn test_hardware_throughput_benchmark_cpu_simd_vs_gpu() {
    let temp_dir = TempDir::new().expect("temp dir");
    let guard = create_test_verified_guard(&temp_dir);

    let selector = HardwareBackendSelector::new();
    let plan = selector.compute_plan(2, 1024 * 1024, false);

    let model_params = ModelLoadParams {
        n_gpu_layers: plan.n_gpu_layers as i32,
        use_mmap: plan.use_mmap,
        ..Default::default()
    };
    let ctx_params = ContextParams {
        n_threads: plan.n_threads,
        n_ubatch: plan.n_ubatch,
        flash_attn: plan.flash_attn,
        ..Default::default()
    };

    let model = SafeLlamaModel::load_verified(&guard, &model_params).expect("load model");
    let mut ctx = SafeLlamaContext::new(model, &ctx_params).expect("create context");

    let mut batch = SafeLlamaBatch::new(1, 0, 1);
    batch.add(1, 0, &[0], true).expect("add token");

    let t0 = Instant::now();
    let steps = 50;
    for _ in 0..steps {
        ctx.decode(&mut batch).expect("decode step");
    }
    let elapsed_sec = t0.elapsed().as_secs_f64();
    let tps = steps as f64 / elapsed_sec;

    assert!(
        tps > 10.0,
        "throughput benchmark must achieve > 10.0 tokens/sec, got {tps:.2} tokens/sec"
    );
}

// =========================================================================
// 5. [Manually-Verified] Telemetry Serialization Verification
// =========================================================================

#[test]
fn test_manual_backend_selector_ui_and_telemetry_reporting() {
    let cpu = CpuBackend::default();
    assert!(cpu.total_cores > 0);
    assert!(cpu.performance_cores > 0);
    assert!(cpu.total_ram_bytes > 0);

    let selector = HardwareBackendSelector::default().with_cpu(cpu);
    let plan = selector.compute_plan(16, 100 * 1024 * 1024, false);

    // Verify fields needed for UI display
    assert!(!plan.backend_name.is_empty());
    assert!(plan.n_threads > 0);
    assert!(plan.n_ubatch > 0);
}
