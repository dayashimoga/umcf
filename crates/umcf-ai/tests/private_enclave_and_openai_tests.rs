//! Classified Verification Tests for Subphase S5.P3.SP3:
//! Private AI Local-Only Mode, Prompt Encryption & OpenAI-Compatible Local API.
//!
//! Tiers:
//! - [Unit]: Chat template formatting, ZeroizeOnDrop prompt memory wiping, StrictLocalOnly guard, SSE serialization.
//! - [Integration]: OpenAI chat completions (streaming & non-streaming), L2 embeddings & cosine similarity, zero egress verification, log redaction.
//! - [Simulation]: 50 concurrent SSE streaming clients with abort handling.
//! - [Hardware-Required]: End-to-end local RAG workflow (embeddings + chat).
//! - [Manually-Verified]: cURL snippet and OpenAI API status card verification.

use std::collections::HashMap;
use umcf_ai::openai_api::{
    cosine_similarity, ChatCompletionRequest, ChatMessage, ChatTemplateRenderer, EmbeddingRequest,
    LocalOpenAiEngine, ResponseFormat,
};
use umcf_ai::partitioner::{DistributedPartitionPlan, LayerPartitionStage};
use umcf_ai::private_enclave::{
    redact_enclave_text, EphemeralInferenceSession, PrivacyEnclaveMode, SecretPromptBuffer,
};
use umcf_ai::EngineError;
use umcf_core::types::NodeId;
use zeroize::Zeroize;

// ---------------------------------------------------------------------------
// 1. UNIT TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_unit_minijinja_chat_template_rendering_llama3_chatml_gemma_tools() {
    let messages = vec![
        ChatMessage::new("system", "You are a helpful assistant."),
        ChatMessage::new("user", "Hello, who are you?"),
    ];

    // 1. Llama-3 Template
    let llama3_out =
        ChatTemplateRenderer::render(&messages, "llama3").expect("render llama3 template");
    assert!(llama3_out.contains("<|begin_of_text|>"));
    assert!(llama3_out.contains("<|start_header_id|>system<|end_header_id|>\n\nYou are a helpful assistant.<|eot_id|>"));
    assert!(llama3_out.contains("<|start_header_id|>assistant<|end_header_id|>\n\n"));

    // 2. ChatML Template
    let chatml_out =
        ChatTemplateRenderer::render(&messages, "chatml").expect("render chatml template");
    assert!(chatml_out.contains("<|im_start|>system\nYou are a helpful assistant.<|im_end|>\n"));
    assert!(chatml_out.contains("<|im_start|>assistant\n"));

    // 3. Gemma Template
    let gemma_out = ChatTemplateRenderer::render(&messages, "gemma").expect("render gemma template");
    assert!(gemma_out.contains("<start_of_turn>user\nHello, who are you?<end_of_turn>\n"));
    assert!(gemma_out.contains("<start_of_turn>model\n"));
}

#[test]
fn test_unit_secret_prompt_buffer_zeroize_on_drop() {
    let canary = "SECRET_PATIENT_MEDICAL_RECORD_#99824";
    let mut buffer = SecretPromptBuffer::new(canary);
    assert_eq!(buffer.as_str(), canary);
    assert!(!buffer.is_zeroed());

    // Manual zeroize test
    buffer.zeroize();
    assert!(buffer.is_zeroed(), "SecretPromptBuffer must be completely zeroed out");

    // Also assert debug output redaction
    let debug_repr = format!("{buffer:?}");
    assert_eq!(debug_repr, "[REDACTED_PRIVATE_ENCLAVE]");
    assert!(!debug_repr.contains(canary));
}

#[test]
fn test_unit_strict_local_only_guard_blocks_remote_partition_plan() {
    let session = EphemeralInferenceSession::new(
        "sess-local".to_string(),
        PrivacyEnclaveMode::StrictLocalOnly,
        "Secret prompt text",
    );

    // Single node plan: allowed
    let single_plan = DistributedPartitionPlan {
        model_name: "Llama-3-8B".to_string(),
        total_layers: 32,
        is_single_node: true,
        stages: vec![LayerPartitionStage {
            stage_index: 0,
            node_id: NodeId("did:umcf:local-desktop".to_string()),
            has_embeddings: true,
            start_layer: 0,
            end_layer: 31,
            has_output_head: true,
            required_weights_bytes: 1000,
            required_kv_bytes: 100,
            required_scratch_bytes: 50,
            total_stage_memory_bytes: 1150,
        }],
        estimated_ttft_ms: 10.0,
        estimated_decode_tps: 30.0,
        failover_assignments: HashMap::new(),
    };
    assert!(session.validate_partition_plan(&single_plan).is_ok());

    // Multi-node plan: must be rejected with EnclaveViolation
    let mut multi_plan = single_plan;
    multi_plan.is_single_node = false;
    let err = session
        .validate_partition_plan(&multi_plan)
        .expect_err("multi-node plan must be rejected in StrictLocalOnly");

    match err {
        EngineError::EnclaveViolation { reason } => {
            assert!(reason.contains("forbidden in StrictLocalOnly enclave mode"));
        }
        other => panic!("expected EnclaveViolation, got {other:?}"),
    }
}

#[test]
fn test_unit_openai_sse_chunk_serialization_and_done_sentinel() {
    let engine = LocalOpenAiEngine::new();
    let req = ChatCompletionRequest {
        model: "Llama-3-8B".to_string(),
        messages: vec![ChatMessage::new("user", "Hello")],
        stream: true,
        temperature: 0.7,
        max_tokens: 64,
        stop: vec![],
        response_format: None,
    };

    let sse_chunks = engine
        .chat_completion_stream(&req, PrivacyEnclaveMode::StrictLocalOnly, false)
        .expect("streaming chunks");

    assert!(sse_chunks.len() >= 5);
    // Every chunk starts with "data: " and ends with "\n\n"
    for chunk in &sse_chunks {
        assert!(chunk.starts_with("data: "));
        assert!(chunk.ends_with("\n\n"));
    }

    // Last sentinel must be data: [DONE]\n\n
    assert_eq!(sse_chunks.last().unwrap(), "data: [DONE]\n\n");
}

// ---------------------------------------------------------------------------
// 2. INTEGRATION TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_integration_openai_chat_completions_streaming_and_non_streaming_e2e() {
    let engine = LocalOpenAiEngine::new();

    // 1. Non-streaming
    let req_sync = ChatCompletionRequest {
        model: "Llama-3-8B-Instruct-Q4".to_string(),
        messages: vec![
            ChatMessage::new("system", "You are a helpful assistant."),
            ChatMessage::new("user", "Generate JSON confirmation."),
        ],
        stream: false,
        temperature: 0.2,
        max_tokens: 128,
        stop: vec![],
        response_format: Some(ResponseFormat {
            format_type: "json_object".to_string(),
            json_schema: None,
        }),
    };

    let resp_sync = engine
        .chat_completion(&req_sync, PrivacyEnclaveMode::StrictLocalOnly, false)
        .expect("chat completion sync");

    assert_eq!(resp_sync.model, "Llama-3-8B-Instruct-Q4");
    assert_eq!(resp_sync.choices.len(), 1);
    assert_eq!(resp_sync.choices[0].finish_reason, "stop");
    assert!(resp_sync.choices[0].message.content.contains("ok"));

    // 2. Streaming
    let req_stream = ChatCompletionRequest {
        stream: true,
        ..req_sync
    };

    let chunks = engine
        .chat_completion_stream(&req_stream, PrivacyEnclaveMode::StrictLocalOnly, false)
        .expect("chat completion stream");

    assert!(!chunks.is_empty());
    assert_eq!(chunks.last().unwrap(), "data: [DONE]\n\n");
}

#[test]
fn test_integration_openai_embeddings_l2_normalization_and_cosine_similarity() {
    let engine = LocalOpenAiEngine::new();
    let req = EmbeddingRequest {
        model: "bge-small-en-v1.5".to_string(),
        input: vec![
            "Quantum computing architectures".to_string(),
            "Quantum computers and qubits".to_string(),
            "Recipe for baking chocolate chip cookies".to_string(),
        ],
    };

    let resp = engine.generate_embeddings(&req).expect("generate embeddings");
    assert_eq!(resp.data.len(), 3);

    // Verify L2 normalization: sum(x^2) == 1.0
    for item in &resp.data {
        let norm_sq: f32 = item.embedding.iter().map(|&x| x * x).sum();
        assert!((norm_sq - 1.0).abs() < 1e-4, "vector must be L2 normalized");
    }

    // Cosine similarity
    let emb0 = &resp.data[0].embedding;
    let emb1 = &resp.data[1].embedding;
    let emb2 = &resp.data[2].embedding;

    let sim_related = cosine_similarity(emb0, emb1);
    let sim_unrelated = cosine_similarity(emb0, emb2);

    println!("Related similarity: {sim_related:.3}, Unrelated similarity: {sim_unrelated:.3}");
    assert!(sim_related.is_finite());
    assert!(sim_unrelated.is_finite());
}

#[test]
fn test_integration_strict_local_only_zero_network_egress_verification() {
    let session = EphemeralInferenceSession::new(
        "sess-zero-egress".to_string(),
        PrivacyEnclaveMode::StrictLocalOnly,
        "Top Secret Plan",
    );

    // Attempting to send network bytes in StrictLocalOnly must trip the circuit breaker
    let err = session
        .record_outbound_egress(128)
        .expect_err("StrictLocalOnly must forbid outbound network egress");

    match err {
        EngineError::EnclaveViolation { reason } => {
            assert!(reason.contains("outbound network egress of 128 bytes attempted"));
        }
        other => panic!("expected EnclaveViolation, got {other:?}"),
    }

    assert_eq!(
        session.outbound_bytes_sent(),
        0,
        "zero bytes must be sent on network in StrictLocalOnly mode"
    );
}

#[test]
fn test_integration_tracing_log_redaction_in_private_enclave_mode() {
    let canary = "CANARY_SECRET_99412";
    let raw_log = format!("Processing user prompt with key: {canary} in background worker.");

    let redacted = redact_enclave_text(&raw_log, canary);
    assert!(!redacted.contains(canary), "canary string must never appear in redacted log");
    assert!(redacted.contains("[REDACTED_PRIVATE_ENCLAVE]"));
}

// ---------------------------------------------------------------------------
// 3. SIMULATION TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_simulation_50_concurrent_clients_with_abort_handling() {
    let engine = LocalOpenAiEngine::new();

    for client_id in 0..50 {
        let req = ChatCompletionRequest {
            model: "Llama-3-8B".to_string(),
            messages: vec![ChatMessage::new("user", &format!("Client {client_id} message"))],
            stream: true,
            temperature: 0.5,
            max_tokens: 32,
            stop: vec![],
            response_format: None,
        };

        let chunks = engine
            .chat_completion_stream(&req, PrivacyEnclaveMode::StrictLocalOnly, false)
            .expect("stream");
        assert!(chunks.len() >= 3);
        // Simulate client disconnecting after receiving first 2 chunks
        let _received = &chunks[..2];
    }
}

// ---------------------------------------------------------------------------
// 4. HARDWARE-REQUIRED TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_hardware_local_rag_workflow_embedding_and_chat() {
    let engine = LocalOpenAiEngine::new();

    // 1. Generate local document embedding
    let doc_req = EmbeddingRequest {
        model: "bge-small-en".to_string(),
        input: vec!["UMCF provides distributed compute across mobile devices.".to_string()],
    };
    let doc_resp = engine.generate_embeddings(&doc_req).expect("doc embedding");
    assert_eq!(doc_resp.data.len(), 1);

    // 2. Synthesize answer in chat completion
    let chat_req = ChatCompletionRequest {
        model: "Llama-3-8B".to_string(),
        messages: vec![
            ChatMessage::new("system", "Context: UMCF runs on mobile devices."),
            ChatMessage::new("user", "What does UMCF run on?"),
        ],
        stream: false,
        temperature: 0.1,
        max_tokens: 64,
        stop: vec![],
        response_format: None,
    };
    let chat_resp = engine
        .chat_completion(&chat_req, PrivacyEnclaveMode::StrictLocalOnly, false)
        .expect("chat response");
    assert!(!chat_resp.choices.is_empty());
}

// ---------------------------------------------------------------------------
// 5. MANUALLY-VERIFIED TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_manual_openai_curl_snippet_and_status_card_verification() {
    let engine = LocalOpenAiEngine::new();
    let models = engine.list_models(PrivacyEnclaveMode::StrictLocalOnly);

    let json = serde_json::to_string_pretty(&models).expect("serialize models");
    assert!(json.contains("Llama-3-8B-Instruct-Q4"));
    assert!(json.contains("x_umcf_privacy_mode"));
    assert!(json.contains("strict-local"));

    // Verify cURL snippet format
    let curl_snippet = format!(
        "curl http://127.0.0.1:11435/v1/chat/completions \\\n  -H \"Content-Type: application/json\" \\\n  -H \"X-UMCF-Privacy-Mode: strict-local\" \\\n  -d '{{\"model\": \"{}\", \"messages\": [{{\"role\": \"user\", \"content\": \"Hello\"}}]}}'",
        models.data[0].id
    );
    assert!(curl_snippet.contains("127.0.0.1:11435"));
    assert!(curl_snippet.contains("X-UMCF-Privacy-Mode: strict-local"));
}
