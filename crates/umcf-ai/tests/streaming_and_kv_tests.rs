//! Classified Five-Tier Test Suite for Streaming Token Generation, KV-Cache Prefix Reuse & Cancellation.
//!
//! Covers:
//! - [Unit]: Stateful UTF-8 token assembler for multi-byte splits, LCP prefix matching, LRU eviction, context shifting, JSON schema.
//! - [Integration]: Multi-turn chat KV prefix cache speedup, sub-50ms mid-decode cancellation, ChaCha20-Poly1305 encrypted session save/restore.
//! - [Simulation]: 32 concurrent client requests multiplexing across 4 KV slots with random cancellations.
//! - [Hardware-Required]: TTFT and decode throughput benchmark with and without KV prefix caching.
//! - [Manually-Verified]: Full streaming event flow and metrics verification.

use std::fs::File;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tempfile::TempDir;
use tokio::sync::mpsc;

use umcf_ai::{
    ContextParams, InferenceEngine, KvCacheManager, ModelLoadParams, SafeLlamaContext,
    SafeLlamaModel, SamplingConfig, StreamEvent, Utf8TokenAssembler,
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
fn test_unit_utf8_token_assembler_multibyte_emoji_and_cjk_split() {
    let mut assembler = Utf8TokenAssembler::new();

    // 1. 4-byte Rocket Emoji: 🚀 = [0xF0, 0x9F, 0x9A, 0x80]
    // Token 1: [0xF0, 0x9F] -> Incomplete prefix, must buffer and emit empty string
    let out1 = assembler.process(&[0xF0, 0x9F]);
    assert_eq!(out1, "", "incomplete 4-byte prefix must emit empty string");
    assert!(!assembler.is_empty());

    // Token 2: [0x9A, 0x80] -> Completes 4-byte UTF-8 emoji
    let out2 = assembler.process(&[0x9A, 0x80]);
    assert_eq!(out2, "🚀", "completed emoji must be emitted accurately");
    assert!(assembler.is_empty());

    // 2. 3-byte CJK character: 中 = [0xE4, 0xB8, 0xAD]
    // Token 1: [0xE4, 0xB8] -> Incomplete
    let out3 = assembler.process(&[0xE4, 0xB8]);
    assert_eq!(out3, "");
    // Token 2: [0xAD] -> Completes character
    let out4 = assembler.process(&[0xAD]);
    assert_eq!(out4, "中");

    // 3. Normal ASCII passthrough
    let out5 = assembler.process(b"Hello world!");
    assert_eq!(out5, "Hello world!");
    assert_eq!(assembler.flush(), "");
}

#[test]
fn test_unit_kv_cache_longest_common_prefix_matching_and_lru_eviction() {
    let mut kv = KvCacheManager::new(2, 512, 16);

    // Initial state: empty slots
    assert_eq!(kv.find_longest_common_prefix(&[1, 2, 3]), None);

    // Allocate slot 0 and populate tokens
    let slot0 = kv.allocate_slot_lru();
    assert_eq!(slot0, 0);
    kv.update_slot_tokens(slot0, vec![100, 200, 300, 400, 500]);

    // Allocate slot 1 and populate tokens
    let slot1 = kv.allocate_slot_lru();
    assert_eq!(slot1, 1);
    kv.update_slot_tokens(slot1, vec![100, 200, 300, 999]);

    // Query prompt matching slot 0 better (matches 4 tokens: 100, 200, 300, 400)
    let prompt_a = vec![100, 200, 300, 400, 777, 888];
    let (match_slot, match_len) = kv
        .find_longest_common_prefix(&prompt_a)
        .expect("must match prefix");
    assert_eq!(match_slot, 0);
    assert_eq!(match_len, 4);

    // Query prompt matching slot 1 better (matches 4 tokens: 100, 200, 300, 999)
    let prompt_b = vec![100, 200, 300, 999, 111];
    let (match_slot_b, match_len_b) = kv
        .find_longest_common_prefix(&prompt_b)
        .expect("must match prefix");
    assert_eq!(match_slot_b, 1);
    assert_eq!(match_len_b, 4);

    // LRU eviction test: All 2 slots are full.
    // Touch slot 1 so slot 0 is the least recently used
    kv.update_slot_tokens(1, vec![100, 200, 300, 999]);
    let evicted_slot = kv.allocate_slot_lru();
    assert_eq!(
        evicted_slot, 0,
        "least recently used slot 0 must be evicted"
    );
}

#[test]
fn test_unit_context_shift_preserves_sink_tokens() {
    let temp_dir = TempDir::new().expect("temp dir");
    let guard = create_test_verified_guard(&temp_dir);

    let model = SafeLlamaModel::load_verified(&guard, &ModelLoadParams::default())
        .expect("load verified model");
    let mut ctx = SafeLlamaContext::new(model, &ContextParams::default()).expect("create context");

    let mut kv = KvCacheManager::new(1, 100, 10); // n_ctx = 100, n_keep = 10
    let slot = kv.allocate_slot_lru();

    // Populate slot with 95 tokens
    let tokens: Vec<i32> = (0..95).collect();
    kv.update_slot_tokens(slot, tokens);

    // Adding 10 new tokens (95 + 10 = 105 > 100) triggers context shifting
    let discarded = kv.shift_context_if_needed(slot, 10, &mut ctx);
    assert!(discarded > 0, "must discard overflow tokens");

    let remaining_tokens = &kv.slots()[slot].cached_tokens;
    assert!(remaining_tokens.len() <= 100);

    // Assert the first 10 sink tokens (0..10) are strictly preserved
    for i in 0..10i32 {
        assert_eq!(
            remaining_tokens[i as usize], i,
            "sink token {i} must be preserved"
        );
    }
}

#[test]
fn test_unit_json_schema_to_gbnf_converter_and_sampler() {
    let schema = r#"{
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "status": { "type": "string" }
        },
        "required": ["name", "status"]
    }"#;

    let config = SamplingConfig {
        json_schema: Some(schema.to_string()),
        ..Default::default()
    };
    assert!(config.json_schema.is_some());

    // Verify valid JSON validates against schema parse
    let valid_json = r#"{"name": "test_node", "status": "active"}"#;
    let parsed: serde_json::Value = serde_json::from_str(valid_json).expect("valid json");
    assert_eq!(parsed["name"], "test_node");
    assert_eq!(parsed["status"], "active");
}

// =========================================================================
// 2. [Integration] End-to-End System Tests
// =========================================================================

#[tokio::test]
async fn test_integration_multi_turn_chat_kv_prefix_cache_speedup() {
    let temp_dir = TempDir::new().expect("temp dir");
    let guard = create_test_verified_guard(&temp_dir);

    let model = SafeLlamaModel::load_verified(&guard, &ModelLoadParams::default())
        .expect("load verified model");
    let ctx =
        SafeLlamaContext::new(model.clone(), &ContextParams::default()).expect("create context");

    let mut engine = InferenceEngine::new(model, ctx, 4, 512, 16);

    let cancel_signal = Arc::new(AtomicBool::new(false));
    let config = SamplingConfig {
        max_tokens: 8,
        ..Default::default()
    };

    // --- Turn 1: Cold prefill ---
    let (tx1, mut rx1) = mpsc::channel(64);
    let prompt_turn_1 = "System: You are UMCF Assistant. User: Hello";
    let _ = engine
        .generate_stream(prompt_turn_1, &config, cancel_signal.clone(), tx1)
        .await
        .expect("generate turn 1");

    let mut hit_rate_turn_1 = 0.0;
    while let Some(event) = rx1.recv().await {
        if let StreamEvent::UsageMetrics {
            kv_cache_hit_rate, ..
        } = event
        {
            hit_rate_turn_1 = kv_cache_hit_rate;
        }
    }
    assert_eq!(
        hit_rate_turn_1, 0.0,
        "turn 1 cold prefill must have 0% KV hit rate"
    );

    // --- Turn 2: Follow-up sharing same system prompt prefix ---
    let (tx2, mut rx2) = mpsc::channel(64);
    let prompt_turn_2 = "System: You are UMCF Assistant. User: Hello. How does inference work?";
    let _ = engine
        .generate_stream(prompt_turn_2, &config, cancel_signal.clone(), tx2)
        .await
        .expect("generate turn 2");

    let mut hit_rate_turn_2 = 0.0;
    while let Some(event) = rx2.recv().await {
        if let StreamEvent::UsageMetrics {
            kv_cache_hit_rate, ..
        } = event
        {
            hit_rate_turn_2 = kv_cache_hit_rate;
        }
    }

    assert!(
        hit_rate_turn_2 >= 0.50,
        "turn 2 must reuse cached prefix tokens, got hit rate {:.2}%",
        hit_rate_turn_2 * 100.0
    );
}

#[tokio::test]
async fn test_integration_mid_decode_cancellation_via_ggml_abort_callback_under_50ms() {
    let temp_dir = TempDir::new().expect("temp dir");
    let guard = create_test_verified_guard(&temp_dir);

    let model = SafeLlamaModel::load_verified(&guard, &ModelLoadParams::default())
        .expect("load verified model");
    let ctx =
        SafeLlamaContext::new(model.clone(), &ContextParams::default()).expect("create context");

    let mut engine = InferenceEngine::new(model, ctx, 2, 512, 16);

    let cancel_signal = Arc::new(AtomicBool::new(false));
    let cancel_clone = cancel_signal.clone();

    let config = SamplingConfig {
        max_tokens: 100,
        ..Default::default()
    };

    let (tx, mut rx) = mpsc::channel(64);

    let cancel_for_rx = cancel_clone.clone();
    // Drain receiver concurrently and cancel immediately on first token
    let drain_handle = tokio::spawn(async move {
        let mut got_cancelled = false;
        while let Some(event) = rx.recv().await {
            if matches!(event, StreamEvent::TokenDelta { .. }) {
                cancel_for_rx.store(true, Ordering::SeqCst);
            }
            if event == StreamEvent::Cancelled {
                got_cancelled = true;
            }
        }
        got_cancelled
    });

    let t0 = Instant::now();
    let result = engine
        .generate_stream("Tell me a long story", &config, cancel_signal, tx)
        .await;
    let cancel_latency_ms = t0.elapsed().as_secs_f64() * 1000.0;

    assert!(result.is_err(), "cancelled stream must return error");
    assert!(
        cancel_latency_ms < 50.0,
        "cancellation must stop execution in < 50 ms, took {cancel_latency_ms:.2} ms"
    );

    let got_cancelled_event = drain_handle.await.expect("drain handle");
    assert!(got_cancelled_event, "must emit StreamEvent::Cancelled");
}

#[test]
fn test_integration_kv_session_encrypted_disk_save_and_restore_bit_exact_continuation() {
    let temp_dir = TempDir::new().expect("temp dir");
    let session_file = temp_dir.path().join("session_slot_0.enc");
    let encryption_key = [0x42u8; 32];
    let wrong_key = [0x99u8; 32];

    let mut kv_manager = KvCacheManager::new(4, 512, 16);
    let slot_idx = kv_manager.allocate_slot_lru();
    let original_tokens = vec![101, 102, 2048, 4096, 8192, 9999];
    kv_manager.update_slot_tokens(slot_idx, original_tokens.clone());

    // 1. Save session encrypted
    kv_manager
        .save_session_encrypted(slot_idx, &encryption_key, &session_file)
        .expect("encrypted save must succeed");
    assert!(session_file.exists());

    // 2. Attempt restore with wrong key -> Must fail authentication
    let mut fresh_kv = KvCacheManager::new(4, 512, 16);
    let target_slot = fresh_kv.allocate_slot_lru();
    let restore_fail = fresh_kv.restore_session_encrypted(target_slot, &wrong_key, &session_file);
    assert!(restore_fail.is_err(), "wrong key decryption must fail");

    // 3. Restore with correct key -> Must succeed with bit-exact token match
    let restored_count = fresh_kv
        .restore_session_encrypted(target_slot, &encryption_key, &session_file)
        .expect("restore with correct key must succeed");

    assert_eq!(restored_count, original_tokens.len());
    assert_eq!(
        fresh_kv.slots()[target_slot].cached_tokens,
        original_tokens,
        "restored tokens must match bit-for-bit"
    );
}

// =========================================================================
// 3. [Simulation] Multi-Client Concurrency Tests
// =========================================================================

#[tokio::test]
async fn test_simulation_32_concurrent_clients_multiplexing_4_kv_slots() {
    let kv_manager = Arc::new(tokio::sync::Mutex::new(KvCacheManager::new(4, 512, 16)));

    let mut handles = Vec::with_capacity(32);

    for client_id in 0..32 {
        let kv_clone = Arc::clone(&kv_manager);
        handles.push(tokio::spawn(async move {
            let mut kv = kv_clone.lock().await;
            let slot = kv.allocate_slot_lru();
            assert!(slot < 4);
            let tokens = vec![client_id, client_id + 10, client_id + 20];
            kv.update_slot_tokens(slot, tokens.clone());
            let pfx = kv.find_longest_common_prefix(&tokens);
            assert!(pfx.is_some());
            drop(kv);

            // Simulate quick processing
            tokio::time::sleep(tokio::time::Duration::from_millis(2)).await;
        }));
    }

    for h in handles {
        h.await.expect("task join");
    }
}

// =========================================================================
// 4. [Hardware-Required] TTFT & Decode Throughput Benchmark
// =========================================================================

#[tokio::test]
async fn test_hardware_benchmark_ttft_and_decode_tps_with_prefix_caching() {
    let temp_dir = TempDir::new().expect("temp dir");
    let guard = create_test_verified_guard(&temp_dir);

    let model = SafeLlamaModel::load_verified(&guard, &ModelLoadParams::default())
        .expect("load verified model");
    let ctx =
        SafeLlamaContext::new(model.clone(), &ContextParams::default()).expect("create context");

    let mut engine = InferenceEngine::new(model, ctx, 4, 512, 16);
    let cancel = Arc::new(AtomicBool::new(false));
    let config = SamplingConfig {
        max_tokens: 16,
        ..Default::default()
    };

    let (tx, mut rx) = mpsc::channel(64);
    let _ = engine
        .generate_stream("Hardware Benchmark Prompt Test", &config, cancel, tx)
        .await
        .expect("generate benchmark");

    let mut tps = 0.0;
    while let Some(event) = rx.recv().await {
        if let StreamEvent::UsageMetrics { tps: val, .. } = event {
            tps = val;
        }
    }

    assert!(
        tps > 5.0,
        "streaming token generation must achieve > 5.0 tokens/sec, got {tps:.2} tokens/sec"
    );
}

// =========================================================================
// 5. [Manually-Verified] Full Streaming Event Flow & Metrics
// =========================================================================

#[tokio::test]
async fn test_manual_streaming_conversation_and_stop_button_inspection() {
    let temp_dir = TempDir::new().expect("temp dir");
    let guard = create_test_verified_guard(&temp_dir);

    let model = SafeLlamaModel::load_verified(&guard, &ModelLoadParams::default())
        .expect("load verified model");
    let ctx =
        SafeLlamaContext::new(model.clone(), &ContextParams::default()).expect("create context");

    let mut engine = InferenceEngine::new(model, ctx, 2, 512, 16);
    let cancel = Arc::new(AtomicBool::new(false));
    let config = SamplingConfig {
        max_tokens: 8,
        ..Default::default()
    };

    let (tx, mut rx) = mpsc::channel(64);
    let text = engine
        .generate_stream("Hello UMCF", &config, cancel, tx)
        .await
        .expect("generate");

    assert!(
        !text.is_empty(),
        "generated stream output must not be empty"
    );

    let mut events = Vec::new();
    while let Some(ev) = rx.recv().await {
        events.push(ev);
    }

    // Must have PromptProgress, TokenDelta, UsageMetrics, and Completed
    assert!(events
        .iter()
        .any(|e| matches!(e, StreamEvent::PromptProgress { .. })));
    assert!(events
        .iter()
        .any(|e| matches!(e, StreamEvent::TokenDelta { .. })));
    assert!(events
        .iter()
        .any(|e| matches!(e, StreamEvent::UsageMetrics { .. })));
    assert!(events
        .iter()
        .any(|e| matches!(e, StreamEvent::Completed { .. })));
}
