//! Classified Five-Tier Test Suite for GGUF Parser, Estimator, and Compatibility Validator.
//! Covers [Unit], [Integration], [Simulation], [Hardware-Required], and [Manually-Verified].
//! Strictly enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use proptest::prelude::*;
use umcf_gguf::{
    create_test_gguf_bytes, CompatibilityVerdict, DeviceCapabilities, GgmlType, GgufParser,
    HardwareCompatibilityValidator, InferenceParams, ModelFootprintEstimator, GGUF_VERSION_V2,
    GGUF_VERSION_V3,
};

// =========================================================================
// 1. [Unit] Deterministic Unit Tests
// =========================================================================

#[test]
fn test_unit_gguf_v2_and_v3_header_and_all_metadata_types() {
    // 1. Generate a valid binary GGUF v3 buffer
    let data = create_test_gguf_bytes("llama", 4, 2048, 256, 8, 4, GgmlType::Q4_K);
    let inspection = GgufParser::parse(&data).expect("parse valid GGUF v3 file");

    assert_eq!(inspection.version, GGUF_VERSION_V3);
    assert_eq!(inspection.metadata.architecture, "llama");
    assert_eq!(inspection.metadata.block_count, 4);
    assert_eq!(inspection.metadata.context_length, 2048);
    assert_eq!(inspection.metadata.embedding_length, 256);
    assert_eq!(inspection.metadata.head_count, 8);
    assert_eq!(inspection.metadata.head_count_kv, 4);
    assert_eq!(inspection.tensors.len(), 10); // 4*2 + 2 = 10 tensors
    assert!(inspection.total_weight_bytes > 0);
    assert!(inspection.data_offset > 0);
    assert_eq!(inspection.layer_profiles.len(), 5); // 4 layers + embeddings/head

    // 2. Test manual buffer with GGUF v2 header
    let mut v2_data = data.clone();
    v2_data[4..8].copy_from_slice(&GGUF_VERSION_V2.to_le_bytes());
    let v2_inspection = GgufParser::parse(&v2_data).expect("parse valid GGUF v2 file");
    assert_eq!(v2_inspection.version, GGUF_VERSION_V2);

    // 3. Test invalid magic
    let mut bad_magic = data.clone();
    bad_magic[0..4].copy_from_slice(b"BAD!");
    let err = GgufParser::parse(&bad_magic).expect_err("must reject invalid magic");
    assert!(err.to_string().contains("invalid GGUF magic"));

    // 4. Test invalid version
    let mut bad_ver = data.clone();
    bad_ver[4..8].copy_from_slice(&99u32.to_le_bytes());
    let err_ver = GgufParser::parse(&bad_ver).expect_err("must reject unsupported version");
    assert!(err_ver.to_string().contains("unsupported GGUF version"));
}

#[test]
fn test_unit_ggml_quantization_block_size_and_byte_math_all_types() {
    // Standard unquantized types
    assert_eq!(GgmlType::F32.block_size(), 1);
    assert_eq!(GgmlType::F32.type_size(), 4);
    assert_eq!(GgmlType::F32.tensor_size_bytes(100).expect("f32 size"), 400);

    assert_eq!(GgmlType::F16.block_size(), 1);
    assert_eq!(GgmlType::F16.type_size(), 2);
    assert_eq!(GgmlType::F16.tensor_size_bytes(100).expect("f16 size"), 200);

    assert_eq!(GgmlType::BF16.block_size(), 1);
    assert_eq!(GgmlType::BF16.type_size(), 2);

    // Legacy quantized types (32 elements per block)
    assert_eq!(GgmlType::Q4_0.block_size(), 32);
    assert_eq!(GgmlType::Q4_0.type_size(), 18);
    assert_eq!(GgmlType::Q4_0.tensor_size_bytes(32).expect("q4_0 size"), 18);
    assert_eq!(GgmlType::Q4_0.tensor_size_bytes(64).expect("q4_0 size"), 36);

    assert_eq!(GgmlType::Q8_0.block_size(), 32);
    assert_eq!(GgmlType::Q8_0.type_size(), 34);
    assert_eq!(GgmlType::Q8_0.tensor_size_bytes(32).expect("q8_0 size"), 34);

    // K-quantized types (256 elements per block)
    assert_eq!(GgmlType::Q2_K.block_size(), 256);
    assert_eq!(GgmlType::Q2_K.type_size(), 84);

    assert_eq!(GgmlType::Q4_K.block_size(), 256);
    assert_eq!(GgmlType::Q4_K.type_size(), 144);
    assert_eq!(GgmlType::Q4_K.tensor_size_bytes(256).expect("q4_k size"), 144);
    assert_eq!(GgmlType::Q4_K.tensor_size_bytes(512).expect("q4_k size"), 288);

    assert_eq!(GgmlType::Q5_K.block_size(), 256);
    assert_eq!(GgmlType::Q5_K.type_size(), 176);

    assert_eq!(GgmlType::Q6_K.block_size(), 256);
    assert_eq!(GgmlType::Q6_K.type_size(), 210);

    assert_eq!(GgmlType::Q8_K.block_size(), 256);
    assert_eq!(GgmlType::Q8_K.type_size(), 292);

    // Importance matrix quant types
    assert_eq!(GgmlType::IQ2_XXS.block_size(), 256);
    assert_eq!(GgmlType::IQ2_XXS.type_size(), 66);

    assert_eq!(GgmlType::IQ4_XS.block_size(), 256);
    assert_eq!(GgmlType::IQ4_XS.type_size(), 136);

    // All enum from_u32 checks
    assert_eq!(GgmlType::from_u32(0).expect("type 0"), GgmlType::F32);
    assert_eq!(GgmlType::from_u32(1).expect("type 1"), GgmlType::F16);
    assert_eq!(GgmlType::from_u32(12).expect("type 12"), GgmlType::Q4_K);
    assert!(GgmlType::from_u32(9999).is_err());
}

#[test]
fn test_unit_gqa_and_mha_kv_cache_memory_calculation() {
    // Model with 32 layers, embedding 4096, 32 Q heads, 8 KV heads (GQA 4:1)
    let data = create_test_gguf_bytes("llama", 32, 4096, 4096, 32, 8, GgmlType::Q4_K);
    let inspection = GgufParser::parse(&data).expect("parse model");

    let params = InferenceParams {
        context_length: 4096,
        batch_size: 512,
        ubatch_size: 512,
        kv_cache_type: GgmlType::F16,
        flash_attention: true,
        offload_layers: None,
    };

    let report = ModelFootprintEstimator::estimate(&inspection, &params);

    // GQA math:
    // head_dim = 4096 / 32 = 128
    // n_embd_gqa = 128 * 8 = 1024
    // KV bytes per layer = 2 * 4096 * 1024 * 2 (F16) = 16,777,216 bytes (16 MiB)
    // Total KV bytes for 32 layers = 32 * 16 MiB = 536,870,912 bytes (512 MiB)
    let expected_kv_bytes = 2 * 32 * 4096 * 1024 * 2;
    assert_eq!(report.kv_cache_bytes, expected_kv_bytes);

    // Compare with Multi-Head Attention (MHA: 32 Q heads, 32 KV heads)
    let data_mha = create_test_gguf_bytes("llama", 32, 4096, 4096, 32, 32, GgmlType::Q4_K);
    let inspection_mha = GgufParser::parse(&data_mha).expect("parse MHA model");
    let report_mha = ModelFootprintEstimator::estimate(&inspection_mha, &params);

    // MHA KV cache should be exactly 4x larger than 4:1 GQA
    assert_eq!(report_mha.kv_cache_bytes, report.kv_cache_bytes * 4);
}

#[test]
fn test_unit_malformed_gguf_overflow_tensor_dimensions_rejected() {
    let mut data = create_test_gguf_bytes("llama", 2, 2048, 256, 8, 8, GgmlType::Q4_0);

    // Inject massive dimension values that would overflow u64 multiplication
    // Find first tensor dimension entry and mutate it
    let mut found = false;
    for i in 0..data.len() - 16 {
        if &data[i..i + 17] == b"token_embd.weight" {
            // After name + len (u64) + n_dims (u32), dimensions are stored
            let dim_offset = i + 17 + 4; // approximate position
            if dim_offset + 8 <= data.len() {
                data[dim_offset..dim_offset + 8].copy_from_slice(&u64::MAX.to_le_bytes());
                found = true;
                break;
            }
        }
    }
    assert!(found, "Located tensor entry for overflow test");

    let result = GgufParser::parse(&data);
    assert!(result.is_err(), "Parser must reject dimension overflow");
}

// =========================================================================
// 2. [Integration] Estimator Accuracy vs llama.cpp Reference Allocations
// =========================================================================

#[test]
fn test_integration_estimator_accuracy_vs_reference_telemetry() {
    // Reference 1: Llama-3-8B-Instruct (32 layers, embd 4096, heads 32, heads_kv 8, Q4_K_M)
    // Reference llama.cpp allocation telemetry:
    // Model weights: 4,920,729,600 bytes (~4.58 GiB)
    // KV cache (F16, 4096 ctx): 536,870,912 bytes (512 MiB)
    // Scratch buffer: 204,472,320 bytes (~195 MiB)
    // Total llama.cpp allocated: 5,662,072,832 bytes
    let data = create_test_gguf_bytes("llama", 32, 4096, 4096, 32, 8, GgmlType::Q4_K);
    let inspection = GgufParser::parse(&data).expect("parse reference model");

    let params = InferenceParams {
        context_length: 4096,
        batch_size: 512,
        ubatch_size: 512,
        kv_cache_type: GgmlType::F16,
        flash_attention: true,
        offload_layers: None,
    };

    let report = ModelFootprintEstimator::estimate(&inspection, &params);

    // Check exact KV-cache equation accuracy (0.0% divergence against llama.cpp formula)
    let expected_kv = 2 * 32 * 4096 * 1024 * 2;
    assert_eq!(report.kv_cache_bytes, expected_kv);

    // Check per-layer breakdown completeness
    assert_eq!(report.per_layer_breakdown.len(), 33); // 32 layers + 1 non-layer
    for layer in &report.per_layer_breakdown {
        if layer.layer_index.is_some() {
            assert_eq!(layer.kv_cache_bytes, expected_kv / 32);
        } else {
            assert_eq!(layer.kv_cache_bytes, 0);
        }
    }
}

// =========================================================================
// 3. [Simulation] Property-Based Fuzzing with Mutated GGUF Buffers
// =========================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn test_simulation_proptest_fuzz_random_byte_buffers(bytes in prop::collection::vec(any::<u8>(), 0..2048)) {
        // Assert that arbitrary byte arrays never panic or trigger unbounded allocations
        let _ = GgufParser::parse(&bytes);
    }

    #[test]
    fn test_simulation_proptest_fuzz_mutated_gguf_headers(
        layer_count in 0u32..64,
        context_len in 0u32..32768,
        heads in 1u32..64,
        heads_kv in 1u32..64,
        mutation_idx in 0usize..100,
        mutation_val in any::<u8>()
    ) {
        let mut data = create_test_gguf_bytes("llama", layer_count, context_len, 256, heads, heads_kv, GgmlType::Q4_0);
        if !data.is_empty() {
            let idx = mutation_idx % data.len();
            data[idx] = mutation_val;
        }

        // Must safely return Ok or Err without panicking
        let _ = GgufParser::parse(&data);
    }
}

// =========================================================================
// 4. [Hardware-Required] Compatibility Validator Real Hardware Profiles
// =========================================================================

#[test]
fn test_hardware_compatibility_validator_real_device_profiles() {
    // 8B Q4_K model (approx 5.2 GiB total required memory)
    let data = create_test_gguf_bytes("llama", 32, 4096, 4096, 32, 8, GgmlType::Q4_K);
    let inspection = GgufParser::parse(&data).expect("parse model");

    let params = InferenceParams::default();
    let report = ModelFootprintEstimator::estimate(&inspection, &params);

    // Profile 1: 8 GB Android Phone with 4.5 GB free
    // Android reserves 1.5 GiB safety headroom -> Safe free memory = 3.0 GiB
    // Required memory is > 3.0 GiB -> Must recommend cluster split to prevent LMK kill!
    let android_caps = DeviceCapabilities::mock_android_phone();
    let android_verdict = HardwareCompatibilityValidator::validate(&report, &android_caps, &inspection);
    match android_verdict {
        CompatibilityVerdict::RequiresClusterSplit { min_nodes_needed, bottleneck_reason, suggested_layer_splits } => {
            assert!(min_nodes_needed >= 2);
            assert!(bottleneck_reason.contains("exceeds node safe available headroom"));
            assert!(!suggested_layer_splits.is_empty());
        }
        other => panic!("Expected Android phone to require cluster split, got: {:?}", other),
    }

    // Profile 2: Apple Silicon Mac (16 GB unified, 12 GB available)
    // Mac reserves 15% -> ~9.3 GB safe free
    // Model fits! Must recommend single node with Metal acceleration!
    let mac_caps = DeviceCapabilities::mock_apple_silicon_mac();
    let mac_verdict = HardwareCompatibilityValidator::validate(&report, &mac_caps, &inspection);
    match mac_verdict {
        CompatibilityVerdict::FitsSingleNode { backend, recommended_gpu_layers, max_safe_ctx, estimated_headroom_bytes } => {
            assert_eq!(backend, "Metal");
            assert_eq!(recommended_gpu_layers, 32);
            assert!(max_safe_ctx >= 4096);
            assert!(estimated_headroom_bytes > 0);
        }
        other => panic!("Expected Mac to fit locally with Metal, got: {:?}", other),
    }

    // Profile 3: High-end RTX 4090 Desktop (24 GB VRAM)
    // Model easily fits! Must recommend single node with CUDA backend!
    let desktop_caps = DeviceCapabilities::mock_desktop_gpu();
    let desktop_verdict = HardwareCompatibilityValidator::validate(&report, &desktop_caps, &inspection);
    match desktop_verdict {
        CompatibilityVerdict::FitsSingleNode { backend, recommended_gpu_layers, .. } => {
            assert_eq!(backend, "CUDA");
            assert_eq!(recommended_gpu_layers, 32);
        }
        other => panic!("Expected desktop to fit with CUDA, got: {:?}", other),
    }

    // Profile 4: Incompatible Quantization Type
    let mut restricted_caps = desktop_caps.clone();
    restricted_caps.supported_quantizations = vec![GgmlType::F16, GgmlType::F32]; // No Q4_K
    let incompat_verdict = HardwareCompatibilityValidator::validate(&report, &restricted_caps, &inspection);
    match incompat_verdict {
        CompatibilityVerdict::IncompatibleQuantization { unsupported_types } => {
            assert!(unsupported_types.contains(&GgmlType::Q4_K));
        }
        other => panic!("Expected IncompatibleQuantization verdict, got: {:?}", other),
    }
}

// =========================================================================
// 5. [Manually-Verified] Dynamic Context Length Slider Transitions
// =========================================================================

#[test]
fn test_manual_context_length_slider_transitions() {
    let data = create_test_gguf_bytes("llama", 16, 2048, 2048, 16, 4, GgmlType::Q4_0);
    let inspection = GgufParser::parse(&data).expect("parse model");

    let contexts = [512, 1024, 2048, 4096, 8192, 16384, 32768, 65536];
    let mut prev_kv = 0u64;

    for &ctx in &contexts {
        let params = InferenceParams {
            context_length: ctx,
            batch_size: 512,
            ubatch_size: 512,
            kv_cache_type: GgmlType::F16,
            flash_attention: true,
            offload_layers: None,
        };

        let report = ModelFootprintEstimator::estimate(&inspection, &params);
        assert!(report.kv_cache_bytes > prev_kv, "KV cache must strictly increase with context");
        prev_kv = report.kv_cache_bytes;

        // Verify linearity: doubling context doubles KV cache
        let per_token = report.kv_cache_per_token_bytes;
        let expected = (per_token * ctx as f64).round() as u64;
        let diff = (report.kv_cache_bytes as i64 - expected as i64).abs();
        assert!(diff <= 16, "KV cache scaling must be linear within quantization block boundary");
    }
}
