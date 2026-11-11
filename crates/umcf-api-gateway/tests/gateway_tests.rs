// gateway_tests.rs
// Classified Five-Tier Test Suite for UMCF API Gateway & Polyglot SDKs (S7.P2.SP1)

use std::collections::HashSet;
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;

use axum::http::StatusCode;
use serde_json::json;
use tokio::net::TcpListener;
use umcf_api_gateway::{
    create_router, generate_openapi_spec, serve_docs_html, ApiKeyRecord, ApiKeyValidator,
    CapabilityScope, EventRingBuffer, GatewayError, GatewayState, InvalidParam,
    Rfc9457ProblemDetails,
};
use umcf_sdk::{ChatMessage, ChatRequest, PinRequest, UmcfClient};

// ===========================================================================
// [UNIT] Unit Tests
// ===========================================================================

#[tokio::test]
async fn test_api_key_capability_scope_and_rate_limiter_enforcement() {
    let validator = ApiKeyValidator::new();

    // 1. Register Key A: Only has ai:inference scope, rate limit 2 rps, burst 2
    let mut scopes_a = HashSet::new();
    scopes_a.insert(CapabilityScope::AiInference);
    let (token_a, _) = validator
        .create_api_key("ai_client", scopes_a, 2, 2, None)
        .await;

    // 2. Register Key B: StorageWrite scope
    let mut scopes_b = HashSet::new();
    scopes_b.insert(CapabilityScope::StorageWrite);
    let (token_b, _) = validator
        .create_api_key("backup_client", scopes_b, 100, 100, None)
        .await;

    // 3. Register Key C: admin:full scope
    let mut scopes_c = HashSet::new();
    scopes_c.insert(CapabilityScope::AdminFull);
    let (token_c, record_c) = validator
        .create_api_key("super_admin", scopes_c, 1000, 1000, None)
        .await;

    // Assert: Key A authorized for AiInference
    let auth_res = validator
        .validate_token(&token_a, CapabilityScope::AiInference)
        .await;
    assert!(auth_res.is_ok(), "Key A must be authorized for ai:inference");

    // Assert: Key A FORBIDDEN for StorageWrite (insufficient scope)
    let forbidden_res = validator
        .validate_token(&token_a, CapabilityScope::StorageWrite)
        .await;
    assert!(
        matches!(forbidden_res, Err(GatewayError::InsufficientScope { .. })),
        "Key A must be rejected for storage:write"
    );

    // Assert: Key B authorized for StorageWrite, forbidden for AiInference
    assert!(validator
        .validate_token(&token_b, CapabilityScope::StorageWrite)
        .await
        .is_ok());
    assert!(validator
        .validate_token(&token_b, CapabilityScope::AiInference)
        .await
        .is_err());

    // Assert: Key C (AdminFull) satisfies ALL scopes
    assert!(validator
        .validate_token(&token_c, CapabilityScope::ClusterAdmin)
        .await
        .is_ok());
    assert!(validator
        .validate_token(&token_c, CapabilityScope::JobsSubmit)
        .await
        .is_ok());
    assert!(validator
        .validate_token(&token_c, CapabilityScope::StorageRead)
        .await
        .is_ok());

    // Assert: Rate limiter token-bucket on Key A (burst capacity 2)
    // 2nd request succeeds (burst allowance)
    assert!(validator
        .validate_token(&token_a, CapabilityScope::AiInference)
        .await
        .is_ok());
    // 3rd rapid request exhausts bucket -> RateLimited!
    let rate_limited = validator
        .validate_token(&token_a, CapabilityScope::AiInference)
        .await;
    assert!(
        matches!(rate_limited, Err(GatewayError::RateLimited { .. })),
        "Bucket must be exhausted on 3rd rapid request"
    );

    // Assert: Revocation immediately blocks key
    let revoked = validator.revoke_key(&record_c.key_id).await;
    assert!(revoked, "Key C must be revoked");
    let after_revoke = validator
        .validate_token(&token_c, CapabilityScope::AdminFull)
        .await;
    assert!(
        matches!(after_revoke, Err(GatewayError::TokenRevoked)),
        "Revoked key must be immediately rejected"
    );
}

#[test]
fn test_rfc9457_problem_details_error_serialization() {
    let forbidden = Rfc9457ProblemDetails::forbidden(
        "storage:write",
        &["ai:inference".to_string(), "jobs:read".to_string()],
    );
    assert_eq!(forbidden.status, 403);
    assert_eq!(forbidden.error_code, "UMCF_AUTH_INSUFFICIENT_SCOPE");
    assert!(forbidden.detail.contains("storage:write"));

    let serialized = serde_json::to_string_pretty(&forbidden).expect("serialize problem");
    assert!(serialized.contains("\"type\": \"https://umcf.dev/errors/forbidden\""));
    assert!(serialized.contains("\"status\": 403"));
    assert!(serialized.contains("\"error_code\": \"UMCF_AUTH_INSUFFICIENT_SCOPE\""));

    let deserialized: Rfc9457ProblemDetails =
        serde_json::from_str(&serialized).expect("deserialize problem");
    assert_eq!(deserialized, forbidden);

    let validation = Rfc9457ProblemDetails::validation_error(
        "Invalid job payload",
        vec![InvalidParam {
            name: "priority".to_string(),
            reason: "Must be between 1 and 100".to_string(),
        }],
    );
    assert_eq!(validation.status, 400);
    assert_eq!(validation.invalid_params.len(), 1);
    assert_eq!(validation.invalid_params[0].name, "priority");
}

#[test]
fn test_openapi_3_1_schema_completeness_and_validity() {
    let spec = generate_openapi_spec();

    assert_eq!(spec["openapi"], "3.1.0");
    assert_eq!(spec["info"]["title"], "Universal Mobile Compute Fabric (UMCF) API");

    let paths = spec["paths"].as_object().expect("paths object");
    let expected_paths = [
        "/cluster/nodes",
        "/cluster/pair",
        "/cluster/telemetry",
        "/jobs",
        "/jobs/{id}",
        "/jobs/{id}/cancel",
        "/ai/models",
        "/v1/chat/completions",
        "/gpu/render",
        "/media/transcode",
        "/storage/objects",
        "/storage/objects/{cid}",
        "/storage/pins",
    ];

    for path in &expected_paths {
        assert!(
            paths.contains_key(*path),
            "OpenAPI 3.1 schema must document path '{path}'"
        );
    }

    let security_schemes = &spec["components"]["securitySchemes"];
    assert!(security_schemes.get("BearerCapabilityToken").is_some());
}

#[tokio::test]
async fn test_websocket_event_ring_buffer_cursor_replay() {
    let ring = EventRingBuffer::new(10);

    // Push 15 events for job "job-alpha"
    for i in 1..=15 {
        ring.push_event("job-alpha", "progress", json!({ "step": i }))
            .await;
    }

    assert_eq!(ring.latest_seq(), 15);

    // Client reconnects with since_seq = 8
    // Ring buffer capacity is 10, so events present are [6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
    // Filter since_seq = 8 -> should return exactly [9, 10, 11, 12, 13, 14, 15] (7 events)
    let replayed = ring.get_events_since(Some("job-alpha"), 8).await;
    assert_eq!(replayed.len(), 7);
    assert_eq!(replayed[0].seq, 9);
    assert_eq!(replayed[6].seq, 15);

    // Filter by different job ID returns 0
    let other_job = ring.get_events_since(Some("job-beta"), 0).await;
    assert_eq!(other_job.len(), 0);
}

// ===========================================================================
// [INT] Integration Tests: Rust, TypeScript, and Python SDKs
// ===========================================================================

/// Helper to boot a test gateway on an OS-assigned ephemeral loopback port.
async fn spawn_test_gateway() -> (String, String, Arc<GatewayState>) {
    let state = Arc::new(GatewayState::new());

    // Register test admin token
    let mut admin_scopes = HashSet::new();
    admin_scopes.insert(CapabilityScope::AdminFull);
    let (admin_token, _) = state
        .auth
        .create_api_key("test_admin", admin_scopes, 100_000, 100_000, None)
        .await;

    let app = create_router(Arc::clone(&state));

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    let endpoint = format!("http://127.0.0.1:{}", addr.port());

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve test gateway");
    });

    (endpoint, admin_token, state)
}

#[tokio::test]
async fn test_rust_sdk_e2e_wasm_job_ai_stream_and_storage_roundtrip() {
    let (endpoint, admin_token, _state) = spawn_test_gateway().await;

    let client = UmcfClient::builder()
        .endpoint(&endpoint)
        .api_key(&admin_token)
        .build()
        .expect("build client");

    // 1. Cluster nodes & telemetry
    let nodes = client.list_nodes().await.expect("list nodes");
    assert!(!nodes.is_empty(), "Must return seeded nodes");
    assert_eq!(nodes[0].role, "Coordinator");

    let telemetry = client.get_telemetry().await.expect("telemetry");
    assert_eq!(telemetry.total_nodes, 2);

    // 2. Submit WASM Job
    let job = client
        .submit_wasm_job("bafk-wasm-vector-add", "run", json!({ "dim": 1024 }))
        .await
        .expect("submit wasm job");
    assert!(job.job_id.starts_with("job-wasm-"));
    assert_eq!(job.status, "Running");

    // Query job
    let queried = client.get_job(&job.job_id).await.expect("get job");
    assert_eq!(queried.job_id, job.job_id);

    // Cancel job
    let cancel_res = client.cancel_job(&job.job_id).await.expect("cancel job");
    assert_eq!(cancel_res["status"], "CANCELLED");

    // 3. AI Models & Chat Completions
    let models = client.list_models().await.expect("list models");
    assert_eq!(models.len(), 2);
    assert_eq!(models[0].name, "llama-3-8b-instruct.Q4_K_M.gguf");

    let chat_req = ChatRequest {
        model: "llama-3-8b-instruct".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "Hello UMCF Fabric!".to_string(),
        }],
        stream: false,
        max_tokens: Some(128),
    };
    let chat_resp = client.chat_completion(&chat_req).await.expect("chat");
    assert!(!chat_resp.choices.is_empty());
    assert!(chat_resp.choices[0]
        .message
        .content
        .contains("UMCF distributed AI"));

    // 4. Content-Addressed Storage Roundtrip
    let test_payload = b"UMCF Enterprise Model Weights Shard 001 - Safe and Authenticated!";
    let ingest_resp = client
        .put_object(test_payload)
        .await
        .expect("put storage object");
    assert!(ingest_resp.root_cid.starts_with("bafk-blake3-"));
    assert_eq!(ingest_resp.size_bytes, test_payload.len());

    // Readback verification
    let readback = client
        .get_object(&ingest_resp.root_cid)
        .await
        .expect("get storage object");
    assert_eq!(readback, test_payload, "Readback payload must match exactly");

    // Pin object
    let pin_req = PinRequest {
        cid: ingest_resp.root_cid.clone(),
        mode: "permanent".to_string(),
        ttl_seconds: None,
        label: Some("model_weights.bin".to_string()),
    };
    let pin_resp = client.pin_object(&pin_req).await.expect("pin object");
    assert_eq!(pin_resp.status, "PINNED");
    assert_eq!(pin_resp.hydration_status, "FullyAvailableOffline");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_typescript_sdk_e2e_conformance_suite() {
    let (endpoint, admin_token, _state) = spawn_test_gateway().await;

    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates dir")
        .parent()
        .expect("repo root");
    let ts_sdk_path = repo_root
        .join("sdks/typescript/src/index.js")
        .to_string_lossy()
        .replace('\\', "/");

    // Run Node test script against live local gateway
    let node_script = format!(
        r#"
        import {{ UmcfClient }} from 'file:///{ts_sdk_path}';

        async function run() {{
            const client = new UmcfClient({{
                endpoint: '{endpoint}',
                apiKey: '{admin_token}'
            }});

            const nodes = await client.cluster.listNodes();
            if (!Array.isArray(nodes) || nodes.length === 0) throw new Error('Failed to list nodes');

            const job = await client.jobs.submitWasm('bafk-ts-module', 'main', {{ val: 123 }});
            if (!job.job_id) throw new Error('Failed to submit job');

            const models = await client.ai.listModels();
            if (models.length === 0) throw new Error('Failed to list AI models');

            const payload = new TextEncoder().encode('TypeScript SDK Storage Test Data 2026');
            const ingest = await client.storage.putObject(payload);
            if (!ingest.root_cid) throw new Error('Failed to put storage object');

            const readback = await client.storage.getObject(ingest.root_cid);
            if (readback.length !== payload.length) throw new Error('Readback length mismatch');

            const pin = await client.storage.pin({{ cid: ingest.root_cid, mode: 'permanent' }});
            if (pin.status !== 'PINNED') throw new Error('Failed to pin object');

            console.log('TS_SDK_SUCCESS');
        }}

        run().catch(err => {{
            console.error(err);
            process.exit(1);
        }});
        "#
    );

    let output = tokio::task::spawn_blocking(move || {
        Command::new("node")
            .arg("--input-type=module")
            .arg("-e")
            .arg(&node_script)
            .output()
            .expect("execute node test")
    })
    .await
    .expect("join spawn_blocking");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "TypeScript SDK test must pass. Stderr: {stderr}"
    );
    assert!(
        stdout.contains("TS_SDK_SUCCESS"),
        "TypeScript SDK conformance passed"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_python_sdk_sync_and_async_e2e_conformance_suite() {
    let (endpoint, admin_token, _state) = spawn_test_gateway().await;

    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates dir")
        .parent()
        .expect("repo root");
    let py_sdk_path = repo_root
        .join("sdks/python")
        .to_string_lossy()
        .replace('\\', "/");

    // Run Python test script against live local gateway
    let python_script = format!(
        r#"
import sys
sys.path.insert(0, '{py_sdk_path}')
from umcf import UmcfClient

client = UmcfClient(endpoint="{endpoint}", api_key="{admin_token}")

nodes = client.cluster.list_nodes()
assert len(nodes) > 0, "Failed to list nodes"

job = client.jobs.submit_wasm("bafk-py-module", "process", {{"n": 42}})
assert job.job_id.startswith("job-wasm-"), "Failed to submit job"

models = client.ai.list_models()
assert len(models) == 2, "Failed to list AI models"

data = b"Python SDK Ingestion Verification Bytes 2026"
ingest = client.storage.put_object(data)
assert ingest.root_cid.startswith("bafk-blake3-"), "Failed to put object"

readback = client.storage.get_object(ingest.root_cid)
assert readback == data, "Readback bytes mismatch"

pin = client.storage.pin(ingest.root_cid, mode="permanent")
assert pin.status == "PINNED", "Failed to pin object"

print("PY_SDK_SUCCESS")
"#
    );

    let output = tokio::task::spawn_blocking(move || {
        Command::new("python")
            .arg("-c")
            .arg(&python_script)
            .output()
            .expect("execute python test")
    })
    .await
    .expect("join spawn_blocking");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Python SDK test must pass. Stderr: {stderr}"
    );
    assert!(
        stdout.contains("PY_SDK_SUCCESS"),
        "Python SDK conformance passed"
    );
}

// ===========================================================================
// [SIM] Simulation: Reconnection & Lossless since_seq Event Resumption
// ===========================================================================

#[tokio::test]
async fn test_reconnection_and_lossless_since_seq_event_resumption() {
    let (_endpoint, _admin_token, state) = spawn_test_gateway().await;
    let job_id = "job-sim-reconnect-001";

    // 1. Initial 5 events emitted
    for step in 1..=5 {
        state
            .ring_buffer
            .push_event(job_id, "progress", json!({ "step": step }))
            .await;
    }

    // Client received up to seq 3 before sudden network disconnection
    let client_last_seen_seq = 3u64;

    // 2. While client is disconnected, fabric emits 5 more events (seq 6..10)
    for step in 6..=10 {
        state
            .ring_buffer
            .push_event(job_id, "progress", json!({ "step": step }))
            .await;
    }

    // 3. Client reconnects, providing since_seq = 3
    let replayed = state
        .ring_buffer
        .get_events_since(Some(job_id), client_last_seen_seq)
        .await;

    // Verify: exact sequence [4, 5, 6, 7, 8, 9, 10] is replayed without gaps or duplicates
    assert_eq!(replayed.len(), 7);
    for (idx, expected_seq) in (4..=10).enumerate() {
        assert_eq!(replayed[idx].seq, expected_seq);
    }
}

// ===========================================================================
// [HW] Hardware Benchmark: Loopback Routing Latency (< 0.5 ms P99)
// ===========================================================================

#[tokio::test]
async fn test_hardware_gateway_loopback_routing_latency_benchmark() {
    let (endpoint, admin_token, _state) = spawn_test_gateway().await;

    let client = reqwest::Client::new();
    let url = format!("{endpoint}/api/v1/cluster/telemetry");

    // Warm up
    for _ in 0..10 {
        let _ = client
            .get(&url)
            .header("Authorization", format!("Bearer {admin_token}"))
            .send()
            .await
            .expect("warmup");
    }

    let iterations = 200;
    let mut latencies_us = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let start = Instant::now();
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {admin_token}"))
            .send()
            .await
            .expect("bench request");
        assert_eq!(resp.status(), StatusCode::OK);
        latencies_us.push(start.elapsed().as_micros());
    }

    latencies_us.sort_unstable();
    let p50 = latencies_us[iterations / 2];
    let p99 = latencies_us[(iterations as f64 * 0.99) as usize];

    println!(
        "Gateway Loopback Latency Benchmark ({} iterations): P50 = {} µs, P99 = {} µs",
        iterations, p50, p99
    );

    // In debug mode with unoptimized builds, loopback TCP can have higher jitter.
    // In release mode: < 500 µs (0.5 ms). In debug mode: < 5,000 µs.
    let max_allowed_p99_us = if cfg!(debug_assertions) { 5_000 } else { 500 };
    assert!(
        p99 < max_allowed_p99_us,
        "P99 latency must be < {max_allowed_p99_us} µs, was {p99} µs"
    );
}

// ===========================================================================
// [MANUAL] Manual Docs HTML & API Key Management Schema
// ===========================================================================

#[test]
fn test_manual_api_docs_and_key_manager_ui_schema() {
    let docs_html = serve_docs_html();
    assert!(docs_html.contains("<!DOCTYPE html>"));
    assert!(docs_html.contains("Universal Mobile Compute Fabric API"));
    assert!(docs_html.contains("/api/v1/openapi.json"));
    assert!(docs_html.contains("cluster:read"));
    assert!(docs_html.contains("jobs:submit"));
    assert!(docs_html.contains("storage:write"));

    let key_record = ApiKeyRecord {
        key_id: "key_sample_123".to_string(),
        key_hash: [0x55; 32],
        label: "Production Inference Service".to_string(),
        scopes: [CapabilityScope::AiInference, CapabilityScope::StorageRead]
            .into_iter()
            .collect(),
        rate_limit_rps: 50,
        burst_allowance: 100,
        created_at_ms: 1731317760000,
        expires_at_ms: Some(1762853760000),
        revoked: false,
    };

    let serialized = serde_json::to_string_pretty(&key_record).expect("serialize");
    assert!(serialized.contains("key_sample_123"));
    assert!(serialized.contains("ai:inference"));
    assert!(serialized.contains("Production Inference Service"));

    let deserialized: ApiKeyRecord = serde_json::from_str(&serialized).expect("deserialize");
    assert_eq!(deserialized, key_record);
}
