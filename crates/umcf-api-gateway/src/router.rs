// router.rs
// Axum router mounting versioned /api/v1/* HTTP/REST, WebSocket, and OpenAI routes.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Bytes;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Path, Query, State};
use axum::http::header::{HeaderMap, AUTHORIZATION, CONTENT_TYPE};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Json, Response};
use axum::routing::{get, post, put};
use axum::Router;
use serde::Deserialize;
use serde_json::json;
use tokio::sync::RwLock;

use crate::auth::{ApiKeyValidator, CapabilityScope};
use crate::error::GatewayError;
use crate::openapi::{generate_openapi_spec, serve_docs_html};
use crate::websocket::{handle_ws_stream, EventRingBuffer};

/// Shared state container for the API Gateway.
#[derive(Debug, Clone)]
pub struct GatewayState {
    pub auth: Arc<ApiKeyValidator>,
    pub ring_buffer: Arc<EventRingBuffer>,
    pub nodes: Arc<RwLock<Vec<serde_json::Value>>>,
    pub jobs: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    pub models: Arc<RwLock<Vec<serde_json::Value>>>,
    pub storage_objects: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    pub pins: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

impl Default for GatewayState {
    fn default() -> Self {
        Self::new()
    }
}

impl GatewayState {
    #[must_use]
    pub fn new() -> Self {
        let auth = Arc::new(ApiKeyValidator::new());
        let ring_buffer = Arc::new(EventRingBuffer::new(2048));

        // Seed with sample fabric node metadata
        let nodes = Arc::new(RwLock::new(vec![
            json!({
                "node_id": "did:umcf:desktop_node_01",
                "role": "Coordinator",
                "os": "Windows 11 x86_64",
                "cpu_cores": 16,
                "gpu_model": "NVIDIA RTX 4090",
                "ram_total_mb": 65536,
                "ram_available_mb": 48210,
                "is_ac_powered": true,
                "status": "Ready"
            }),
            json!({
                "node_id": "did:umcf:mobile_worker_02",
                "role": "Worker",
                "os": "Android 15 ARM64",
                "cpu_cores": 8,
                "gpu_model": "Adreno 750",
                "ram_total_mb": 16384,
                "ram_available_mb": 10240,
                "is_ac_powered": true,
                "status": "Ready"
            }),
        ]));

        // Seed with model catalog
        let models = Arc::new(RwLock::new(vec![
            json!({
                "name": "llama-3-8b-instruct.Q4_K_M.gguf",
                "architecture": "llama",
                "quantization": "Q4_K_M",
                "context_length": 8192,
                "vram_required_mb": 4920,
                "is_locally_resident": true
            }),
            json!({
                "name": "whisper-large-v3.Q5_K.gguf",
                "architecture": "whisper",
                "quantization": "Q5_K",
                "context_length": 1500,
                "vram_required_mb": 1024,
                "is_locally_resident": true
            }),
        ]));

        Self {
            auth,
            ring_buffer,
            nodes,
            jobs: Arc::new(RwLock::new(HashMap::new())),
            models,
            storage_objects: Arc::new(RwLock::new(HashMap::new())),
            pins: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

/// Helper to authenticate and authorize requests against an expected capability scope.
async fn require_scope(
    headers: &HeaderMap,
    validator: &ApiKeyValidator,
    scope: CapabilityScope,
) -> Result<(), GatewayError> {
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(GatewayError::MissingToken)?;

    validator.validate_token(auth_header, scope).await?;
    Ok(())
}

/// Query parameters for WebSocket event streaming.
#[derive(Debug, Deserialize)]
pub struct WsEventQuery {
    pub since_seq: Option<u64>,
}

/// Constructs the full Axum router for the UMCF API Gateway.
pub fn create_router(state: Arc<GatewayState>) -> Router {
    Router::new()
        // Documentation & Schema routes
        .route("/api/v1/docs", get(handle_docs))
        .route("/api/v1/openapi.json", get(handle_openapi))
        // Cluster routes
        .route("/api/v1/cluster/nodes", get(handle_list_nodes))
        .route("/api/v1/cluster/pair", post(handle_pair))
        .route("/api/v1/cluster/telemetry", get(handle_telemetry))
        // Jobs routes
        .route("/api/v1/jobs", post(handle_submit_job))
        .route("/api/v1/jobs/:id", get(handle_get_job))
        .route("/api/v1/jobs/:id/cancel", post(handle_cancel_job))
        .route("/ws/v1/jobs/:id/events", get(handle_ws_events))
        // AI routes
        .route("/api/v1/ai/models", get(handle_list_models))
        .route("/api/v1/ai/models/pull", post(handle_pull_model))
        .route("/v1/chat/completions", post(handle_chat_completions))
        .route("/v1/embeddings", post(handle_embeddings))
        // GPU & Media routes
        .route("/api/v1/gpu/render", post(handle_gpu_render))
        .route("/api/v1/media/transcode", post(handle_media_transcode))
        .route("/api/v1/stream/sessions", post(handle_stream_sessions))
        // Storage routes
        .route("/api/v1/storage/objects", put(handle_put_storage_object))
        .route("/api/v1/storage/objects/:cid", get(handle_get_storage_object))
        .route("/api/v1/storage/pins", post(handle_create_pin))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Route Handlers
// ---------------------------------------------------------------------------

async fn handle_docs() -> impl IntoResponse {
    Html(serve_docs_html())
}

async fn handle_openapi() -> impl IntoResponse {
    Json(generate_openapi_spec())
}

async fn handle_list_nodes(
    headers: HeaderMap,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<serde_json::Value>, GatewayError> {
    require_scope(&headers, &state.auth, CapabilityScope::ClusterRead).await?;
    let nodes = state.nodes.read().await;
    Ok(Json(json!(*nodes)))
}

async fn handle_pair(
    headers: HeaderMap,
    State(state): State<Arc<GatewayState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, GatewayError> {
    require_scope(&headers, &state.auth, CapabilityScope::ClusterAdmin).await?;
    let token = body
        .get("pairing_token")
        .and_then(|v| v.as_str())
        .unwrap_or("tok_default");

    Ok(Json(json!({
        "status": "PAIRED",
        "peer_node_id": "did:umcf:enrolled_node_99",
        "cluster_id": "cluster-local-mesh",
        "paired_with_token": token,
        "session_ttl_sec": 86400
    })))
}

async fn handle_telemetry(
    headers: HeaderMap,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<serde_json::Value>, GatewayError> {
    require_scope(&headers, &state.auth, CapabilityScope::ClusterRead).await?;
    let node_count = state.nodes.read().await.len();
    Ok(Json(json!({
        "total_nodes": node_count,
        "active_workers": node_count.saturating_sub(1),
        "aggregate_cpu_cores": 24,
        "aggregate_ram_gb": 80.0,
        "cluster_storage_used_gb": 5.68,
        "cluster_storage_capacity_gb": 500.0,
        "network_mesh_p95_rtt_ms": 1.42
    })))
}

async fn handle_submit_job(
    headers: HeaderMap,
    State(state): State<Arc<GatewayState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Response, GatewayError> {
    require_scope(&headers, &state.auth, CapabilityScope::JobsSubmit).await?;

    let job_type = body
        .get("job_type")
        .and_then(|v| v.as_str())
        .unwrap_or("wasm");

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let job_id = format!("job-{job_type}-{now_ms}");
    let record = json!({
        "job_id": job_id.clone(),
        "job_type": job_type,
        "status": "Running",
        "progress_pct": 0.0,
        "assigned_node": "did:umcf:desktop_node_01",
        "payload": body.get("payload").cloned().unwrap_or(json!({})),
        "created_at_ms": now_ms
    });

    {
        let mut jobs = state.jobs.write().await;
        jobs.insert(job_id.clone(), record.clone());
    }

    // Emit initial lifecycle events into ring buffer
    state
        .ring_buffer
        .push_event(&job_id, "scheduled", json!({ "phase": "Queued" }))
        .await;

    state
        .ring_buffer
        .push_event(
            &job_id,
            "progress",
            json!({ "progress_pct": 25.0, "status": "Executing" }),
        )
        .await;

    Ok((StatusCode::ACCEPTED, Json(record)).into_response())
}

async fn handle_get_job(
    headers: HeaderMap,
    Path(id): Path<String>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<serde_json::Value>, GatewayError> {
    require_scope(&headers, &state.auth, CapabilityScope::JobsRead).await?;

    let jobs = state.jobs.read().await;
    let job = jobs
        .get(&id)
        .cloned()
        .ok_or_else(|| GatewayError::NotFound {
            resource: "Job".to_string(),
            id: id.clone(),
        })?;

    Ok(Json(job))
}

async fn handle_cancel_job(
    headers: HeaderMap,
    Path(id): Path<String>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<serde_json::Value>, GatewayError> {
    require_scope(&headers, &state.auth, CapabilityScope::JobsSubmit).await?;

    let mut jobs = state.jobs.write().await;
    let job = jobs
        .get_mut(&id)
        .ok_or_else(|| GatewayError::NotFound {
            resource: "Job".to_string(),
            id: id.clone(),
        })?;

    if let Some(obj) = job.as_object_mut() {
        obj.insert("status".to_string(), json!("Cancelled"));
    }

    state
        .ring_buffer
        .push_event(&id, "failed", json!({ "reason": "UserCancelled" }))
        .await;

    Ok(Json(json!({ "status": "CANCELLED", "job_id": id })))
}

async fn handle_ws_events(
    ws: WebSocketUpgrade,
    Path(id): Path<String>,
    Query(params): Query<WsEventQuery>,
    headers: HeaderMap,
    State(state): State<Arc<GatewayState>>,
) -> Response {
    // Check since_seq from query param or Last-Event-ID header
    let since_seq = params.since_seq.or_else(|| {
        headers
            .get("Last-Event-ID")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
    }).unwrap_or(0);

    let ring = Arc::clone(&state.ring_buffer);
    ws.on_upgrade(move |socket| handle_ws_stream(socket, ring, id, since_seq))
}

async fn handle_list_models(
    headers: HeaderMap,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<serde_json::Value>, GatewayError> {
    require_scope(&headers, &state.auth, CapabilityScope::AiInference).await?;
    let models = state.models.read().await;
    Ok(Json(json!(*models)))
}

async fn handle_pull_model(
    headers: HeaderMap,
    State(state): State<Arc<GatewayState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, GatewayError> {
    require_scope(&headers, &state.auth, CapabilityScope::AiInference).await?;
    let model_name = body
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("llama-3-8b-instruct.Q4_K_M.gguf");

    Ok(Json(json!({
        "status": "PULLED",
        "model": model_name,
        "size_mb": 4920,
        "is_locally_resident": true
    })))
}

async fn handle_chat_completions(
    headers: HeaderMap,
    State(state): State<Arc<GatewayState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Response, GatewayError> {
    require_scope(&headers, &state.auth, CapabilityScope::AiInference).await?;

    let model = body
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("llama-3-8b-instruct");

    let is_stream = body
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let now_sec = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let completion_id = format!("chatcmpl-{now_sec}");

    if is_stream {
        // SSE streaming chunk response
        let chunk = format!(
            "data: {}\n\ndata: [DONE]\n\n",
            json!({
                "id": completion_id,
                "object": "chat.completion.chunk",
                "created": now_sec,
                "model": model,
                "choices": [{
                    "index": 0,
                    "delta": { "content": "UMCF distributed AI compute pipeline online." },
                    "finish_reason": null
                }]
            })
        );

        Ok((
            StatusCode::OK,
            [
                (CONTENT_TYPE, "text/event-stream"),
                (axum::http::header::CACHE_CONTROL, "no-cache"),
            ],
            chunk,
        )
            .into_response())
    } else {
        Ok(Json(json!({
            "id": completion_id,
            "object": "chat.completion",
            "created": now_sec,
            "model": model,
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "UMCF distributed AI compute pipeline online."
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 12,
                "completion_tokens": 8,
                "total_tokens": 20
            }
        }))
        .into_response())
    }
}

async fn handle_embeddings(
    headers: HeaderMap,
    State(state): State<Arc<GatewayState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, GatewayError> {
    require_scope(&headers, &state.auth, CapabilityScope::AiInference).await?;
    let model = body.get("model").and_then(|v| v.as_str()).unwrap_or("text-embedding-3-small");

    Ok(Json(json!({
        "object": "list",
        "data": [{
            "object": "embedding",
            "index": 0,
            "embedding": [0.012, -0.045, 0.821, -0.112, 0.054]
        }],
        "model": model,
        "usage": { "prompt_tokens": 5, "total_tokens": 5 }
    })))
}

async fn handle_gpu_render(
    headers: HeaderMap,
    State(state): State<Arc<GatewayState>>,
    Json(_body): Json<serde_json::Value>,
) -> Result<Response, GatewayError> {
    require_scope(&headers, &state.auth, CapabilityScope::GpuRender).await?;
    let job_id = format!("render-{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis());
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({ "job_id": job_id, "status": "Queued", "tiles": 16 })),
    )
        .into_response())
}

async fn handle_media_transcode(
    headers: HeaderMap,
    State(state): State<Arc<GatewayState>>,
    Json(_body): Json<serde_json::Value>,
) -> Result<Response, GatewayError> {
    require_scope(&headers, &state.auth, CapabilityScope::MediaTranscode).await?;
    let job_id = format!("transcode-{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis());
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({ "job_id": job_id, "status": "Queued", "segments": 8 })),
    )
        .into_response())
}

async fn handle_stream_sessions(
    headers: HeaderMap,
    State(state): State<Arc<GatewayState>>,
    Json(_body): Json<serde_json::Value>,
) -> Result<Response, GatewayError> {
    require_scope(&headers, &state.auth, CapabilityScope::StreamConnect).await?;
    let session_id = format!("stream-{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis());
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "session_id": session_id,
            "transport": "QUIC_DATAGRAM",
            "codec": "AV1",
            "fps": 60,
            "bitrate_mbps": 12.0
        })),
    )
        .into_response())
}

async fn handle_put_storage_object(
    headers: HeaderMap,
    State(state): State<Arc<GatewayState>>,
    body: Bytes,
) -> Result<Response, GatewayError> {
    require_scope(&headers, &state.auth, CapabilityScope::StorageWrite).await?;

    let payload = body.to_vec();
    let hash = blake3::hash(&payload);
    let cid = format!("bafk-blake3-{}", hex::encode(hash.as_bytes()));

    {
        let mut store = state.storage_objects.write().await;
        store.insert(cid.clone(), payload.clone());
    }

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "root_cid": cid,
            "size_bytes": payload.len(),
            "chunk_count": 1,
            "deduplicated_chunks": 0
        })),
    )
        .into_response())
}

async fn handle_get_storage_object(
    headers: HeaderMap,
    Path(cid): Path<String>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Response, GatewayError> {
    require_scope(&headers, &state.auth, CapabilityScope::StorageRead).await?;

    let store = state.storage_objects.read().await;
    let data = store
        .get(&cid)
        .cloned()
        .ok_or_else(|| GatewayError::NotFound {
            resource: "StorageObject".to_string(),
            id: cid.clone(),
        })?;

    Ok((
        StatusCode::OK,
        [
            (CONTENT_TYPE, "application/octet-stream"),
            (axum::http::header::CONTENT_LENGTH, data.len().to_string().as_str()),
        ],
        data,
    )
        .into_response())
}

async fn handle_create_pin(
    headers: HeaderMap,
    State(state): State<Arc<GatewayState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Response, GatewayError> {
    require_scope(&headers, &state.auth, CapabilityScope::StorageWrite).await?;

    let cid = body
        .get("cid")
        .and_then(|v| v.as_str())
        .ok_or_else(|| GatewayError::Validation {
            detail: "Missing 'cid' property in pin request".to_string(),
            invalid_params: vec![crate::problem::InvalidParam {
                name: "cid".to_string(),
                reason: "Field is required".to_string(),
            }],
        })?;

    let mode = body
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("permanent");

    {
        let mut pins = state.pins.write().await;
        pins.insert(cid.to_string(), body.clone());
    }

    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "PINNED",
            "cid": cid,
            "mode": mode,
            "hydration_status": "FullyAvailableOffline"
        })),
    )
        .into_response())
}
