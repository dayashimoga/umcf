// types.ts — TypeScript schemas matching OpenAPI 3.1.0 contracts

export type NodeRole = "Coordinator" | "Worker" | "Observer";
export type NodeStatus = "Ready" | "Busy" | "Degraded" | "Offline";

export interface NodeDescriptor {
  node_id: string;
  role: NodeRole;
  os: string;
  cpu_cores: number;
  gpu_model?: string | null;
  ram_total_mb: number;
  ram_available_mb: number;
  is_ac_powered: boolean;
  status: NodeStatus;
}

export interface ClusterTelemetry {
  total_nodes: number;
  active_workers: number;
  aggregate_cpu_cores: number;
  aggregate_ram_gb: number;
  cluster_storage_used_gb: number;
  cluster_storage_capacity_gb: number;
  network_mesh_p95_rtt_ms: number;
}

export type JobType = "wasm" | "ai_inference" | "gpu_render" | "media_transcode";
export type JobStatus = "Queued" | "Running" | "Completed" | "Failed" | "Cancelled";

export interface JobSubmission {
  job_type: JobType;
  target_role?: string | null;
  priority?: number;
  payload: Record<string, unknown>;
}

export interface JobRecord {
  job_id: string;
  job_type: JobType;
  status: JobStatus;
  progress_pct: number;
  assigned_node: string;
  payload: Record<string, unknown>;
  created_at_ms: number;
}

export interface JobEventRecord {
  seq: number;
  job_id: string;
  timestamp_ms: number;
  event_type: string;
  payload: Record<string, unknown>;
}

export interface ModelInfo {
  name: string;
  architecture: string;
  quantization: string;
  context_length: number;
  vram_required_mb: number;
  is_locally_resident: boolean;
}

export interface ChatMessage {
  role: "system" | "user" | "assistant";
  content: string;
}

export interface ChatRequest {
  model: string;
  messages: ChatMessage[];
  stream?: boolean;
  max_tokens?: number;
}

export interface ChatResponse {
  id: string;
  object: string;
  created: number;
  model: string;
  choices: Array<{
    index: number;
    message: ChatMessage;
    finish_reason: string | null;
  }>;
}

export interface StorageIngestResponse {
  root_cid: string;
  size_bytes: number;
  chunk_count: number;
  deduplicated_chunks: number;
}

export interface PinRequest {
  cid: string;
  mode: "permanent" | "ttl" | "job";
  ttl_seconds?: number | null;
  label?: string | null;
}

export interface PinResponse {
  status: string;
  cid: string;
  mode: string;
  hydration_status: string;
}

export interface Rfc9457ProblemDetails {
  type: string;
  title: string;
  status: number;
  detail: string;
  instance?: string | null;
  error_code: string;
  invalid_params?: Array<{ name: string; reason: string }>;
}
