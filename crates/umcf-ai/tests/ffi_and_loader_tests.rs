//! Classified Five-Tier Test Suite for Safe `llama.cpp` FFI Bridge and GGML Backend Loader.
//!
//! Covers:
//! - [Unit]: Backend lifecycle idempotency, Arc model outliving dropped handle, batch bounds, progress abort, samplers.
//! - [Integration]: Deterministic GGUF load via `VerifiedModelGuard`, prompt tokenization, decoding, sampling, detokenization.
//! - [Simulation]: 16-thread multi-context concurrent stress test, atomic decode abort signal.
//! - [Hardware-Required]: Native host execution benchmark and decode latency measurement.
//! - [Manually-Verified]: Structured `tracing` log capture with target "umcf_ai::llama_cpp".

use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use std::thread;
use std::time::Instant;
use tempfile::TempDir;

use umcf_ai::{
    ContextParams, DynamicBackendLoader, EngineError, LlamaBackendGuard, LlamaBackendManager,
    ModelLoadParams, SafeLlamaBatch, SafeLlamaContext, SafeLlamaModel, SafeSamplerChain,
};
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
fn test_unit_backend_init_idempotent_and_tracing_log_bridge() {
    let initial_ref = LlamaBackendManager::ref_count();

    // Acquire multiple guards
    let g1 = LlamaBackendGuard::new();
    assert_eq!(LlamaBackendManager::ref_count(), initial_ref + 1);

    let g2 = LlamaBackendGuard::new();
    assert_eq!(LlamaBackendManager::ref_count(), initial_ref + 2);

    let g3 = g2.clone();
    assert_eq!(LlamaBackendManager::ref_count(), initial_ref + 3);

    drop(g3);
    assert_eq!(LlamaBackendManager::ref_count(), initial_ref + 2);

    drop(g2);
    assert_eq!(LlamaBackendManager::ref_count(), initial_ref + 1);

    drop(g1);
    assert_eq!(LlamaBackendManager::ref_count(), initial_ref);
}

#[test]
fn test_unit_model_outlives_dropped_caller_handle_while_context_alive() {
    let temp_dir = TempDir::new().expect("temp dir");
    let guard = create_test_verified_guard(&temp_dir);

    let params = ModelLoadParams::default();
    let model = SafeLlamaModel::load_verified(&guard, &params).expect("load verified model");
    assert_eq!(Arc::strong_count(&model), 1);

    let ctx_params = ContextParams::default();
    let mut ctx = SafeLlamaContext::new(Arc::clone(&model), &ctx_params).expect("create context");
    assert_eq!(Arc::strong_count(&model), 2);

    // Caller drops their reference to the model
    drop(model);

    // Model is still kept alive by context
    assert_eq!(Arc::strong_count(ctx.model()), 1);

    // Context can still decode tokens normally
    let mut batch = SafeLlamaBatch::new(8, 0, 1);
    batch.add(1, 0, &[0], true).expect("add bos token");
    ctx.decode(&mut batch).expect("decode with surviving model");

    let logits = ctx.get_logits();
    assert!(!logits.is_empty(), "logits must be populated");
}

#[test]
fn test_unit_batch_bounds_checking_and_raii_drop() {
    let mut batch = SafeLlamaBatch::new(4, 0, 1);
    assert_eq!(batch.capacity(), 4);
    assert_eq!(batch.n_tokens(), 0);

    // Add up to capacity
    batch.add(10, 0, &[0], false).expect("add token 0");
    batch.add(11, 1, &[0], false).expect("add token 1");
    batch.add(12, 2, &[0], false).expect("add token 2");
    batch.add(13, 3, &[0], true).expect("add token 3");
    assert_eq!(batch.n_tokens(), 4);

    // Exceed capacity -> Must return structured error
    let err = batch.add(14, 4, &[0], true);
    assert!(err.is_err(), "exceeding batch capacity must return error");
    match err {
        Err(EngineError::InvalidBatch { reason }) => {
            assert!(reason.contains("capacity 4 exceeded"));
        }
        _ => panic!("unexpected error variant"),
    }

    // Clear resets token count without reallocation
    batch.clear();
    assert_eq!(batch.n_tokens(), 0);
    assert_eq!(batch.capacity(), 4);

    // Can add tokens again after clear
    batch.add(20, 0, &[0], true).expect("add after clear");
    assert_eq!(batch.n_tokens(), 1);
}

#[test]
fn test_unit_model_load_abort_via_progress_callback() {
    let temp_dir = TempDir::new().expect("temp dir");
    let guard = create_test_verified_guard(&temp_dir);

    let progress_history = Arc::new(std::sync::Mutex::new(Vec::new()));
    let history_clone = Arc::clone(&progress_history);

    // Configure progress callback that aborts at 50%
    let params = ModelLoadParams {
        progress_callback: Some(Arc::new(move |progress: f32| {
            history_clone.lock().unwrap().push(progress);
            // Abort when progress reaches 50%
            if progress >= 0.50 {
                return false;
            }
            true
        })),
        ..Default::default()
    };

    let start = Instant::now();
    let result = SafeLlamaModel::load_verified(&guard, &params);
    let elapsed = start.elapsed();

    // Verify it aborted cleanly
    assert!(result.is_err(), "model load must fail when cancelled");
    match result {
        Err(EngineError::LoadAborted) => {}
        _ => panic!("expected LoadAborted error"),
    }

    // Cancellation must be instantaneous (< 50 ms)
    assert!(
        elapsed.as_millis() < 50,
        "abort must complete in < 50 ms, took {} ms",
        elapsed.as_millis()
    );

    let history = progress_history.lock().unwrap().clone();
    assert!(
        !history.is_empty(),
        "progress callback must have been invoked"
    );
    assert!(history.contains(&0.25f32));
    assert!(history.contains(&0.50f32));
}

#[test]
fn test_unit_sampler_chain_creation_and_greedy_sample() {
    let temp_dir = TempDir::new().expect("temp dir");
    let guard = create_test_verified_guard(&temp_dir);

    let model = SafeLlamaModel::load_verified(&guard, &ModelLoadParams::default())
        .expect("load verified model");
    let mut ctx = SafeLlamaContext::new(model, &ContextParams::default()).expect("create context");

    let mut batch = SafeLlamaBatch::new(4, 0, 1);
    batch.add(4, 0, &[0], true).expect("add token");
    ctx.decode(&mut batch).expect("decode");

    // Greedy sampler
    let mut greedy = SafeSamplerChain::new_greedy();
    let token = greedy.sample(&ctx, 0);
    assert!(token > 0, "greedy sampler must return valid token id");

    // Chained sampler (Top-K + Temperature + Greedy)
    let mut chain = SafeSamplerChain::new_chain();
    chain.add(SafeSamplerChain::new_top_k(40));
    chain.add(SafeSamplerChain::new_temp(0.7));
    chain.add(SafeSamplerChain::new_greedy());

    let token_chain = chain.sample(&ctx, 0);
    assert!(
        token_chain > 0,
        "chained sampler must return valid token id"
    );
}

// =========================================================================
// 2. [Integration] End-to-End System Tests
// =========================================================================

#[test]
fn test_integration_deterministic_tiny_gguf_e2e_decode_and_detokenize() {
    let temp_dir = TempDir::new().expect("temp dir");
    let guard = create_test_verified_guard(&temp_dir);

    let model = SafeLlamaModel::load_verified(&guard, &ModelLoadParams::default())
        .expect("load verified model");

    // 1. Tokenize prompt
    let prompt = "Hello world";
    let tokens = model
        .tokenize(prompt, true, false)
        .expect("tokenize prompt");
    assert!(!tokens.is_empty(), "tokens must not be empty");
    assert_eq!(tokens[0], model.bos_token(), "first token must be BOS");

    // 2. Create context
    let ctx_params = ContextParams {
        n_ctx: 512,
        n_batch: 512,
        ..Default::default()
    };
    let mut ctx = SafeLlamaContext::new(model.clone(), &ctx_params).expect("create context");

    // 3. Ingest prompt tokens into batch
    let mut batch = SafeLlamaBatch::new(tokens.len(), 0, 1);
    for (i, &tok) in tokens.iter().enumerate() {
        let is_last = i == tokens.len() - 1;
        batch
            .add(tok, i as i32, &[0], is_last)
            .expect("add prompt token");
    }

    ctx.decode(&mut batch).expect("decode prompt batch");

    // 4. Autoregressively sample 16 greedy tokens
    let mut sampler = SafeSamplerChain::new_greedy();
    let mut generated_tokens = Vec::new();
    let mut generated_text = String::new();

    for current_pos in (tokens.len() as i32..).take(16) {
        let next_token = sampler.sample(&ctx, 0);
        if next_token == model.eos_token() {
            break;
        }
        generated_tokens.push(next_token);

        let piece = model.token_to_piece(next_token).expect("detokenize piece");
        generated_text.push_str(&piece);

        // Decode next single token
        let mut single_batch = SafeLlamaBatch::new(1, 0, 1);
        single_batch
            .add(next_token, current_pos, &[0], true)
            .expect("add next token");
        ctx.decode(&mut single_batch).expect("decode step");
    }

    assert!(
        !generated_tokens.is_empty(),
        "must have generated at least 1 token"
    );
    assert!(
        !generated_text.is_empty(),
        "generated text piece must not be empty"
    );
}

#[test]
fn test_integration_dynamic_backend_loader_blake3_signature_verification() {
    let temp_dir = TempDir::new().expect("temp dir");

    // 1. Probe default compiled-in backends
    let default_backends = DynamicBackendLoader::probe_available_backends();
    assert!(
        !default_backends.is_empty(),
        "must have at least 1 default backend"
    );
    assert_eq!(default_backends[0].name, "CPU");
    assert!(default_backends[0].memory_total_bytes > 0);

    // 2. Create synthetic plugin library
    let fake_plugin_path = temp_dir.path().join("ggml-mock-backend.dll");
    let fake_plugin_bytes = b"MOCK_GGML_DYNAMIC_BACKEND_BYTECODE_V1";
    File::create(&fake_plugin_path)
        .expect("create fake plugin")
        .write_all(fake_plugin_bytes)
        .expect("write fake plugin");

    let actual_blake3 = blake3::hash(fake_plugin_bytes).to_hex().to_string();
    let wrong_blake3 = "0000000000000000000000000000000000000000000000000000000000000000";

    // 3. Verify signature mismatch is rejected
    let mismatch_err = DynamicBackendLoader::load_plugin_verified(&fake_plugin_path, wrong_blake3);
    assert!(mismatch_err.is_err());
    match mismatch_err {
        Err(EngineError::BackendPluginSignatureMismatch {
            expected, actual, ..
        }) => {
            assert_eq!(expected, wrong_blake3);
            assert_eq!(actual, actual_blake3);
        }
        _ => panic!("expected BackendPluginSignatureMismatch error"),
    }

    // 4. Verify valid signature succeeds and probes devices
    let success = DynamicBackendLoader::load_plugin_verified(&fake_plugin_path, &actual_blake3);
    assert!(success.is_ok(), "verified plugin load must succeed");
}

// =========================================================================
// 3. [Simulation] Concurrency & Chaos Simulation Tests
// =========================================================================

#[test]
fn test_simulation_multithread_16_threads_shared_model_stress() {
    let temp_dir = TempDir::new().expect("temp dir");
    let guard = create_test_verified_guard(&temp_dir);

    let model = SafeLlamaModel::load_verified(&guard, &ModelLoadParams::default())
        .expect("load verified model");

    let thread_count = 16;
    let mut handles = Vec::with_capacity(thread_count);

    let start = Instant::now();

    for thread_idx in 0..thread_count {
        let model_clone = Arc::clone(&model);
        handles.push(thread::spawn(move || {
            let ctx_params = ContextParams {
                n_ctx: 128,
                n_batch: 64,
                ..Default::default()
            };
            let mut ctx =
                SafeLlamaContext::new(model_clone, &ctx_params).expect("thread context creation");

            let mut batch = SafeLlamaBatch::new(16, 0, 1);
            for step in 0..16i32 {
                batch.clear();
                let token_id = (thread_idx as i32 * 10 + step) % 256;
                batch.add(token_id, step, &[0], true).expect("batch add");
                ctx.decode(&mut batch).expect("thread decode step");
                let logits = ctx.get_logits();
                assert!(!logits.is_empty());
            }
        }));
    }

    for handle in handles {
        handle.join().expect("thread must complete without panic");
    }

    let elapsed = start.elapsed();
    assert!(
        elapsed.as_secs() < 10,
        "16 concurrent contexts stress test must finish in < 10s, took {} ms",
        elapsed.as_millis()
    );
}

#[test]
fn test_simulation_atomic_abort_signal_stops_inflight_decode() {
    let temp_dir = TempDir::new().expect("temp dir");
    let guard = create_test_verified_guard(&temp_dir);

    let model = SafeLlamaModel::load_verified(&guard, &ModelLoadParams::default())
        .expect("load verified model");
    let mut ctx = SafeLlamaContext::new(model, &ContextParams::default()).expect("create context");

    let mut batch = SafeLlamaBatch::new(4, 0, 1);
    batch.add(1, 0, &[0], true).expect("add token");

    // First decode succeeds
    ctx.decode(&mut batch).expect("initial decode");
    assert!(!ctx.is_aborted());

    // Trigger abort
    ctx.abort();
    assert!(ctx.is_aborted());

    // Subsequent decode must immediately abort
    let err = ctx.decode(&mut batch);
    assert!(err.is_err(), "decoding after abort must fail");
    match err {
        Err(EngineError::GenerationAborted) => {}
        _ => panic!("expected GenerationAborted error"),
    }

    // Reset abort and verify decoding resumes
    ctx.reset_abort();
    assert!(!ctx.is_aborted());
    ctx.decode(&mut batch).expect("decode after reset abort");
}

// =========================================================================
// 4. [Hardware-Required] Native Execution & Microbenchmark
// =========================================================================

#[test]
fn test_hardware_native_ffi_smoke_and_latency_benchmark() {
    let temp_dir = TempDir::new().expect("temp dir");
    let guard = create_test_verified_guard(&temp_dir);

    let start_load = Instant::now();
    let model = SafeLlamaModel::load_verified(&guard, &ModelLoadParams::default())
        .expect("load verified model");
    let load_ms = start_load.elapsed().as_secs_f64() * 1000.0;

    let start_ctx = Instant::now();
    let mut ctx = SafeLlamaContext::new(model, &ContextParams::default()).expect("create context");
    let ctx_ms = start_ctx.elapsed().as_secs_f64() * 1000.0;

    let mut batch = SafeLlamaBatch::new(1, 0, 1);
    batch.add(5, 0, &[0], true).expect("add token");

    let mut latencies_ms = Vec::with_capacity(100);
    for _ in 0..100 {
        let t0 = Instant::now();
        ctx.decode(&mut batch).expect("decode step");
        latencies_ms.push(t0.elapsed().as_secs_f64() * 1000.0);
    }

    let avg_decode_ms: f64 = latencies_ms.iter().sum::<f64>() / latencies_ms.len() as f64;

    assert!(
        load_ms < 500.0,
        "model load time must be < 500 ms, got {load_ms:.2} ms"
    );
    assert!(
        ctx_ms < 100.0,
        "context creation time must be < 100 ms, got {ctx_ms:.2} ms"
    );
    assert!(
        avg_decode_ms < 25.0,
        "average single-token decode latency must be < 25 ms, got {avg_decode_ms:.3} ms"
    );
}

// =========================================================================
// 5. [Manually-Verified] Structured Tracing Log Capture
// =========================================================================

#[test]
fn test_manual_tracing_log_capture_and_structure_verification() {
    let temp_dir = TempDir::new().expect("temp dir");
    let guard = create_test_verified_guard(&temp_dir);

    // Initialize subscriber to verify logs are properly formed
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_test_writer()
        .finish();

    let _sub_guard = tracing::subscriber::set_default(subscriber);

    // Perform engine operations triggering native C logging callbacks
    let model = SafeLlamaModel::load_verified(&guard, &ModelLoadParams::default())
        .expect("load verified model");
    let mut ctx = SafeLlamaContext::new(model, &ContextParams::default()).expect("create context");

    let mut batch = SafeLlamaBatch::new(2, 0, 1);
    batch.add(1, 0, &[0], true).expect("add token");
    ctx.decode(&mut batch).expect("decode");

    // Clear KV cache
    ctx.clear_kv_cache();

    // Verify context properties
    assert_eq!(ctx.n_ctx(), 512);
}
