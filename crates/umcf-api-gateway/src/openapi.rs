// openapi.rs
// Canonical OpenAPI 3.1.0 Schema Generator and Embedded API Explorer Docs.
// Exposes machine-readable contracts and offline-capable interactive documentation.

use serde_json::json;

/// Generates the canonical OpenAPI 3.1.0 JSON specification covering all `/api/v1/*` routes.
#[must_use]
pub fn generate_openapi_spec() -> serde_json::Value {
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Universal Mobile Compute Fabric (UMCF) API",
            "version": "1.0.0",
            "description": "Unified versioned REST, WebSocket, and gRPC developer gateway for distributed AI inference, WASM compute, GPU rendering, media transcoding, and content-addressed storage.",
            "contact": {
                "name": "UMCF Platform Engineering",
                "url": "https://umcf.dev"
            },
            "license": {
                "name": "Apache-2.0 OR MIT",
                "url": "https://github.com/umcf/universal-mobile-compute-fabric"
            }
        },
        "servers": [
            {
                "url": "/api/v1",
                "description": "Local / Mesh Gateway Endpoint"
            }
        ],
        "security": [
            {
                "BearerCapabilityToken": []
            }
        ],
        "paths": {
            "/cluster/nodes": {
                "get": {
                    "summary": "List Connected Cluster Nodes",
                    "description": "Returns verified hardware topology, capability descriptors, roles, and status of all nodes in the mesh.",
                    "operationId": "listClusterNodes",
                    "tags": ["Cluster"],
                    "security": [{ "BearerCapabilityToken": ["cluster:read"] }],
                    "responses": {
                        "200": {
                            "description": "Array of active node descriptors",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "array",
                                        "items": { "$ref": "#/components/schemas/NodeDescriptor" }
                                    }
                                }
                            }
                        },
                        "401": { "$ref": "#/components/responses/401Unauthorized" },
                        "403": { "$ref": "#/components/responses/403Forbidden" }
                    }
                }
            },
            "/cluster/pair": {
                "post": {
                    "summary": "Initiate or Confirm Peer Device Pairing",
                    "description": "Exchanges short-lived enrollment tokens and SAS commitments to enroll a new node into the cluster.",
                    "operationId": "pairDevice",
                    "tags": ["Cluster"],
                    "security": [{ "BearerCapabilityToken": ["cluster:admin"] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/PairRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "Pairing confirmation and mTLS session credentials" },
                        "400": { "$ref": "#/components/responses/400Validation" },
                        "403": { "$ref": "#/components/responses/403Forbidden" }
                    }
                }
            },
            "/cluster/telemetry": {
                "get": {
                    "summary": "Get Live Cluster Telemetry",
                    "description": "Retrieves real-time aggregated CPU, GPU, RAM, thermal, battery, and storage metrics across the fabric.",
                    "operationId": "getClusterTelemetry",
                    "tags": ["Cluster"],
                    "security": [{ "BearerCapabilityToken": ["cluster:read"] }],
                    "responses": {
                        "200": {
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ClusterTelemetry" }
                                }
                            }
                        }
                    }
                }
            },
            "/jobs": {
                "post": {
                    "summary": "Submit Distributed Compute Job",
                    "description": "Schedules a WASM, AI inference, GPU render, or media transcoding job onto capable fabric nodes.",
                    "operationId": "submitJob",
                    "tags": ["Jobs"],
                    "security": [{ "BearerCapabilityToken": ["jobs:submit"] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/JobSubmissionRequest" }
                            }
                        }
                    },
                    "responses": {
                        "202": {
                            "description": "Job accepted and queued",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/JobRecord" }
                                }
                            }
                        },
                        "400": { "$ref": "#/components/responses/400Validation" },
                        "403": { "$ref": "#/components/responses/403Forbidden" }
                    }
                }
            },
            "/jobs/{id}": {
                "get": {
                    "summary": "Get Job Status & Results",
                    "description": "Queries current execution phase, progress percentage, assigned nodes, and output artifact CIDs.",
                    "operationId": "getJobStatus",
                    "tags": ["Jobs"],
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "security": [{ "BearerCapabilityToken": ["jobs:read"] }],
                    "responses": {
                        "200": {
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/JobRecord" }
                                }
                            }
                        },
                        "404": { "$ref": "#/components/responses/404NotFound" }
                    }
                }
            },
            "/jobs/{id}/cancel": {
                "post": {
                    "summary": "Cancel Running Job",
                    "description": "Sends cancellation token to active worker nodes and terminates pipeline execution in < 50ms.",
                    "operationId": "cancelJob",
                    "tags": ["Jobs"],
                    "parameters": [{ "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }],
                    "security": [{ "BearerCapabilityToken": ["jobs:submit"] }],
                    "responses": {
                        "200": { "description": "Job cancellation acknowledged" }
                    }
                }
            },
            "/ai/models": {
                "get": {
                    "summary": "List Available GGUF Models",
                    "description": "Returns registered model architectures, quantizations, VRAM footprints, and local cache residency.",
                    "operationId": "listAiModels",
                    "tags": ["AI Inference"],
                    "security": [{ "BearerCapabilityToken": ["ai:inference"] }],
                    "responses": {
                        "200": {
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "array",
                                        "items": { "$ref": "#/components/schemas/ModelInfo" }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/v1/chat/completions": {
                "post": {
                    "summary": "OpenAI-Compatible Chat Completions",
                    "description": "Executes streaming or non-streaming chat completions across distributed llama.cpp node pipelines.",
                    "operationId": "createChatCompletion",
                    "tags": ["AI Inference"],
                    "security": [{ "BearerCapabilityToken": ["ai:inference"] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ChatCompletionRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "Chat completion response or SSE token stream" }
                    }
                }
            },
            "/gpu/render": {
                "post": {
                    "summary": "Dispatch Distributed GPU Render Task",
                    "description": "Partitions 2D tile or 3D frame render tasks across GPU-equipped fabric nodes.",
                    "operationId": "submitGpuRender",
                    "tags": ["GPU & Rendering"],
                    "security": [{ "BearerCapabilityToken": ["gpu:render"] }],
                    "responses": { "202": { "description": "Render job queued" } }
                }
            },
            "/media/transcode": {
                "post": {
                    "summary": "Submit Segment-Parallel Media Transcoding Task",
                    "description": "Distributes video/audio transcoding across hardware encoders with seamless Annex-B stitching.",
                    "operationId": "submitMediaTranscode",
                    "tags": ["Media Processing"],
                    "security": [{ "BearerCapabilityToken": ["media:transcode"] }],
                    "responses": { "202": { "description": "Transcode job queued" } }
                }
            },
            "/storage/objects": {
                "put": {
                    "summary": "Ingest Content-Addressed Object",
                    "description": "Chunks payload via FastCDC, reserves atomic quota, stores chunks in 2-level fanout, and returns root CID.",
                    "operationId": "putStorageObject",
                    "tags": ["Storage"],
                    "security": [{ "BearerCapabilityToken": ["storage:write"] }],
                    "responses": {
                        "201": {
                            "description": "Object ingested successfully",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/StorageIngestResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/storage/objects/{cid}": {
                "get": {
                    "summary": "Retrieve Content-Addressed Object",
                    "description": "Reads and reassembles constituent chunks from local store or peer erasure shards.",
                    "operationId": "getStorageObject",
                    "tags": ["Storage"],
                    "parameters": [{ "name": "cid", "in": "path", "required": true, "schema": { "type": "string" } }],
                    "security": [{ "BearerCapabilityToken": ["storage:read"] }],
                    "responses": {
                        "200": { "description": "Raw object byte stream" },
                        "404": { "$ref": "#/components/responses/404NotFound" }
                    }
                }
            },
            "/storage/pins": {
                "post": {
                    "summary": "Pin Object for Guaranteed Offline Access",
                    "description": "Pins an object manifest with Permanent, TTL lease, or Active Job hold, preventing GC eviction.",
                    "operationId": "createStoragePin",
                    "tags": ["Storage"],
                    "security": [{ "BearerCapabilityToken": ["storage:write"] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/PinRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "Pin registered and hydration queued" }
                    }
                }
            }
        },
        "components": {
            "securitySchemes": {
                "BearerCapabilityToken": {
                    "type": "http",
                    "scheme": "bearer",
                    "bearerFormat": "UMCF Capability Token",
                    "description": "Pass scoped API key (e.g. `Bearer umcf_live_...`) with required permission scope."
                }
            },
            "schemas": {
                "NodeDescriptor": {
                    "type": "object",
                    "required": ["node_id", "role", "os", "cpu_cores", "ram_total_mb", "status"],
                    "properties": {
                        "node_id": { "type": "string", "example": "did:umcf:desktop_node_01" },
                        "role": { "type": "string", "enum": ["Coordinator", "Worker", "Observer"] },
                        "os": { "type": "string", "example": "Windows 11 x86_64" },
                        "cpu_cores": { "type": "integer", "example": 16 },
                        "gpu_model": { "type": "string", "nullable": true, "example": "NVIDIA RTX 4090" },
                        "ram_total_mb": { "type": "integer", "example": 65536 },
                        "ram_available_mb": { "type": "integer", "example": 48210 },
                        "is_ac_powered": { "type": "boolean", "example": true },
                        "status": { "type": "string", "enum": ["Ready", "Busy", "Degraded", "Offline"] }
                    }
                },
                "PairRequest": {
                    "type": "object",
                    "required": ["pairing_token", "initiator_public_key"],
                    "properties": {
                        "pairing_token": { "type": "string" },
                        "initiator_public_key": { "type": "string" },
                        "cluster_id": { "type": "string" }
                    }
                },
                "ClusterTelemetry": {
                    "type": "object",
                    "properties": {
                        "total_nodes": { "type": "integer", "example": 8 },
                        "active_workers": { "type": "integer", "example": 6 },
                        "aggregate_cpu_cores": { "type": "integer", "example": 64 },
                        "aggregate_ram_gb": { "type": "number", "example": 256.0 },
                        "cluster_storage_used_gb": { "type": "number", "example": 120.5 },
                        "cluster_storage_capacity_gb": { "type": "number", "example": 1024.0 }
                    }
                },
                "JobSubmissionRequest": {
                    "type": "object",
                    "required": ["job_type", "payload"],
                    "properties": {
                        "job_type": { "type": "string", "enum": ["wasm", "ai_inference", "gpu_render", "media_transcode"] },
                        "target_role": { "type": "string", "nullable": true },
                        "priority": { "type": "integer", "default": 10 },
                        "payload": { "type": "object" }
                    }
                },
                "JobRecord": {
                    "type": "object",
                    "properties": {
                        "job_id": { "type": "string" },
                        "job_type": { "type": "string" },
                        "status": { "type": "string", "enum": ["Queued", "Running", "Completed", "Failed", "Cancelled"] },
                        "progress_pct": { "type": "number", "example": 75.0 },
                        "assigned_node": { "type": "string" },
                        "created_at_ms": { "type": "integer" }
                    }
                },
                "ModelInfo": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "quantization": { "type": "string" },
                        "vram_required_mb": { "type": "integer" },
                        "is_locally_resident": { "type": "boolean" }
                    }
                },
                "ChatCompletionRequest": {
                    "type": "object",
                    "required": ["model", "messages"],
                    "properties": {
                        "model": { "type": "string" },
                        "messages": { "type": "array", "items": { "type": "object" } },
                        "stream": { "type": "boolean", "default": false },
                        "max_tokens": { "type": "integer", "default": 256 }
                    }
                },
                "StorageIngestResponse": {
                    "type": "object",
                    "properties": {
                        "root_cid": { "type": "string", "example": "bafk-blake3-d92e8fa..." },
                        "size_bytes": { "type": "integer", "example": 4194304 },
                        "chunk_count": { "type": "integer", "example": 4 },
                        "deduplicated_chunks": { "type": "integer", "example": 2 }
                    }
                },
                "PinRequest": {
                    "type": "object",
                    "required": ["cid", "mode"],
                    "properties": {
                        "cid": { "type": "string" },
                        "mode": { "type": "string", "enum": ["permanent", "ttl", "job"] },
                        "ttl_seconds": { "type": "integer", "nullable": true },
                        "label": { "type": "string" }
                    }
                },
                "Rfc9457ProblemDetails": {
                    "type": "object",
                    "required": ["type", "title", "status", "detail", "error_code"],
                    "properties": {
                        "type": { "type": "string" },
                        "title": { "type": "string" },
                        "status": { "type": "integer" },
                        "detail": { "type": "string" },
                        "instance": { "type": "string", "nullable": true },
                        "error_code": { "type": "string" },
                        "invalid_params": { "type": "array", "items": { "type": "object" } }
                    }
                }
            },
            "responses": {
                "401Unauthorized": {
                    "description": "Authentication token missing or invalid",
                    "content": {
                        "application/problem+json": {
                            "schema": { "$ref": "#/components/schemas/Rfc9457ProblemDetails" }
                        }
                    }
                },
                "403Forbidden": {
                    "description": "Insufficient capability scope for this operation",
                    "content": {
                        "application/problem+json": {
                            "schema": { "$ref": "#/components/schemas/Rfc9457ProblemDetails" }
                        }
                    }
                },
                "404NotFound": {
                    "description": "Target resource was not found",
                    "content": {
                        "application/problem+json": {
                            "schema": { "$ref": "#/components/schemas/Rfc9457ProblemDetails" }
                        }
                    }
                },
                "400Validation": {
                    "description": "Invalid request parameter or payload schema",
                    "content": {
                        "application/problem+json": {
                            "schema": { "$ref": "#/components/schemas/Rfc9457ProblemDetails" }
                        }
                    }
                }
            }
        }
    })
}

/// Generates a standalone, dark-themed HTML5 API Explorer interface for `/api/v1/docs`.
#[must_use]
pub fn serve_docs_html() -> String {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>UMCF API Explorer — v1.0.0</title>
  <style>
    :root {
      --bg: #0f172a;
      --card-bg: #1e293b;
      --border: #334155;
      --text: #f8fafc;
      --text-muted: #94a3b8;
      --accent: #38bdf8;
      --accent-hover: #0284c7;
      --green: #4ade80;
      --amber: #fbbf24;
      --purple: #c084fc;
      --code-bg: #090d16;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; font-family: ui-sans-serif, system-ui, sans-serif; }
    body { background: var(--bg); color: var(--text); padding: 24px; line-height: 1.5; }
    header { max-width: 1100px; margin: 0 auto 32px; border-bottom: 1px solid var(--border); padding-bottom: 24px; }
    h1 { font-size: 28px; font-weight: 800; color: var(--accent); display: flex; align-items: center; gap: 12px; }
    .badge { font-size: 12px; font-weight: 700; background: rgba(56, 189, 248, 0.2); color: var(--accent); padding: 4px 8px; border-radius: 6px; }
    .desc { color: var(--text-muted); margin-top: 8px; font-size: 15px; }
    .container { max-width: 1100px; margin: 0 auto; display: flex; flex-direction: column; gap: 16px; }
    .tag-section { font-size: 18px; font-weight: 700; margin-top: 16px; color: var(--text); }
    .endpoint { background: var(--card-bg); border: 1px solid var(--border); border-radius: 8px; padding: 16px; }
    .endpoint-header { display: flex; align-items: center; gap: 12px; }
    .method { font-size: 12px; font-weight: 800; padding: 4px 10px; border-radius: 6px; text-transform: uppercase; }
    .method-get { background: rgba(74, 222, 128, 0.2); color: var(--green); }
    .method-post { background: rgba(56, 189, 248, 0.2); color: var(--accent); }
    .method-put { background: rgba(251, 191, 36, 0.2); color: var(--amber); }
    .path { font-family: monospace; font-size: 15px; font-weight: 700; }
    .summary { color: var(--text-muted); font-size: 13px; margin-left: auto; }
    .details { margin-top: 12px; padding-top: 12px; border-top: 1px solid var(--border); font-size: 13px; color: var(--text-muted); }
    .scope-chip { font-size: 11px; background: rgba(192, 132, 252, 0.2); color: var(--purple); padding: 2px 6px; border-radius: 4px; font-family: monospace; }
    pre { background: var(--code-bg); padding: 12px; border-radius: 6px; overflow-x: auto; font-family: monospace; font-size: 12px; margin-top: 8px; color: var(--green); }
  </style>
</head>
<body>
  <header>
    <h1>Universal Mobile Compute Fabric API <span class="badge">v1.0.0</span></h1>
    <p class="desc">High-performance versioned HTTP/REST, WebSocket, and gRPC endpoints for distributed AI, WASM, GPU, Media, and Storage.</p>
    <p style="margin-top: 8px;"><a href="/api/v1/openapi.json" style="color: var(--accent); text-decoration: none; font-weight: 600;">Download OpenAPI 3.1.0 Specification (JSON) &rarr;</a></p>
  </header>
  <main class="container">
    <div class="tag-section">Cluster & Mesh Coordination</div>
    <div class="endpoint">
      <div class="endpoint-header">
        <span class="method method-get">GET</span>
        <span class="path">/api/v1/cluster/nodes</span>
        <span class="summary">List connected nodes & capabilities</span>
      </div>
      <div class="details">Required Scope: <span class="scope-chip">cluster:read</span></div>
    </div>
    <div class="endpoint">
      <div class="endpoint-header">
        <span class="method method-post">POST</span>
        <span class="path">/api/v1/cluster/pair</span>
        <span class="summary">Initiate or confirm device pairing</span>
      </div>
      <div class="details">Required Scope: <span class="scope-chip">cluster:admin</span></div>
    </div>
    <div class="endpoint">
      <div class="endpoint-header">
        <span class="method method-get">GET</span>
        <span class="path">/api/v1/cluster/telemetry</span>
        <span class="summary">Live cluster resource utilization</span>
      </div>
      <div class="details">Required Scope: <span class="scope-chip">cluster:read</span></div>
    </div>

    <div class="tag-section">Distributed Jobs & Workloads</div>
    <div class="endpoint">
      <div class="endpoint-header">
        <span class="method method-post">POST</span>
        <span class="path">/api/v1/jobs</span>
        <span class="summary">Submit WASM / compute / media job</span>
      </div>
      <div class="details">Required Scope: <span class="scope-chip">jobs:submit</span></div>
    </div>
    <div class="endpoint">
      <div class="endpoint-header">
        <span class="method method-get">GET</span>
        <span class="path">/api/v1/jobs/{id}</span>
        <span class="summary">Query job status & output</span>
      </div>
      <div class="details">Required Scope: <span class="scope-chip">jobs:read</span></div>
    </div>
    <div class="endpoint">
      <div class="endpoint-header">
        <span class="method method-get">GET</span>
        <span class="path">/ws/v1/jobs/{id}/events</span>
        <span class="summary">WebSocket event stream with ?since_seq=N replay</span>
      </div>
      <div class="details">Supports Last-Event-ID header and query param since_seq for lossless reconnection.</div>
    </div>

    <div class="tag-section">AI Inference & LLM Engine</div>
    <div class="endpoint">
      <div class="endpoint-header">
        <span class="method method-get">GET</span>
        <span class="path">/api/v1/ai/models</span>
        <span class="summary">List GGUF model catalog</span>
      </div>
      <div class="details">Required Scope: <span class="scope-chip">ai:inference</span></div>
    </div>
    <div class="endpoint">
      <div class="endpoint-header">
        <span class="method method-post">POST</span>
        <span class="path">/v1/chat/completions</span>
        <span class="summary">OpenAI-compatible streaming chat completion</span>
      </div>
      <div class="details">Compatible with official openai-python and openai-node SDKs.</div>
    </div>

    <div class="tag-section">Content-Addressed Storage & Pinning</div>
    <div class="endpoint">
      <div class="endpoint-header">
        <span class="method method-put">PUT</span>
        <span class="path">/api/v1/storage/objects</span>
        <span class="summary">Ingest byte stream via FastCDC chunking</span>
      </div>
      <div class="details">Required Scope: <span class="scope-chip">storage:write</span></div>
    </div>
    <div class="endpoint">
      <div class="endpoint-header">
        <span class="method method-get">GET</span>
        <span class="path">/api/v1/storage/objects/{cid}</span>
        <span class="summary">Retrieve and reassemble object byte stream</span>
      </div>
      <div class="details">Required Scope: <span class="scope-chip">storage:read</span></div>
    </div>
    <div class="endpoint">
      <div class="endpoint-header">
        <span class="method method-post">POST</span>
        <span class="path">/api/v1/storage/pins</span>
        <span class="summary">Pin object for guaranteed offline availability</span>
      </div>
      <div class="details">Required Scope: <span class="scope-chip">storage:write</span></div>
    </div>
  </main>
</body>
</html>"#
    .to_string()
}

