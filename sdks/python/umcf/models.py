"""models.py — Data structures and payload schemas for the UMCF Python SDK."""

from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional


@dataclass
class NodeDescriptor:
    node_id: str
    role: str
    os: str
    cpu_cores: int
    ram_total_mb: int
    ram_available_mb: int
    is_ac_powered: bool
    status: str
    gpu_model: Optional[str] = None


@dataclass
class ClusterTelemetry:
    total_nodes: int
    active_workers: int
    aggregate_cpu_cores: int
    aggregate_ram_gb: float
    cluster_storage_used_gb: float
    cluster_storage_capacity_gb: float
    network_mesh_p95_rtt_ms: float


@dataclass
class JobRecord:
    job_id: str
    job_type: str
    status: str
    progress_pct: float
    assigned_node: str
    payload: Dict[str, Any]
    created_at_ms: int


@dataclass
class ModelInfo:
    name: str
    architecture: str
    quantization: str
    context_length: int
    vram_required_mb: int
    is_locally_resident: bool


@dataclass
class ChatMessage:
    role: str
    content: str


@dataclass
class StorageIngestResponse:
    root_cid: str
    size_bytes: int
    chunk_count: int
    deduplicated_chunks: int


@dataclass
class PinResponse:
    status: str
    cid: str
    mode: str
    hydration_status: str
