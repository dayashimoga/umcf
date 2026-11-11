// types.rs
// Strongly-typed request and response structures for the Rust UMCF SDK.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDescriptor {
    pub node_id: String,
    pub role: String,
    pub os: String,
    pub cpu_cores: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_model: Option<String>,
    pub ram_total_mb: u64,
    pub ram_available_mb: u64,
    pub is_ac_powered: bool,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterTelemetry {
    pub total_nodes: usize,
    pub active_workers: usize,
    pub aggregate_cpu_cores: u32,
    pub aggregate_ram_gb: f64,
    pub cluster_storage_used_gb: f64,
    pub cluster_storage_capacity_gb: f64,
    pub network_mesh_p95_rtt_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSubmission {
    pub job_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_role: Option<String>,
    pub priority: u32,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRecord {
    pub job_id: String,
    pub job_type: String,
    pub status: String,
    pub progress_pct: f64,
    pub assigned_node: String,
    pub payload: serde_json::Value,
    pub created_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub architecture: String,
    pub quantization: String,
    pub context_length: u32,
    pub vram_required_mb: u64,
    pub is_locally_resident: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: usize,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageIngestResponse {
    pub root_cid: String,
    pub size_bytes: usize,
    pub chunk_count: usize,
    pub deduplicated_chunks: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinRequest {
    pub cid: String,
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinResponse {
    pub status: String,
    pub cid: String,
    pub mode: String,
    pub hydration_status: String,
}

