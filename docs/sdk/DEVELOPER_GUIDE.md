# DEVELOPER_GUIDE.md — UMCF Developer SDK & API Gateway Reference

This document provides architectural references, quickstarts, and schema definitions for developers integrating with the **Universal Mobile Compute Fabric (UMCF)** using the official **Rust**, **TypeScript**, and **Python** client SDKs.

---

## 1. Versioned API Gateway & Scoped Capability Tokens (`S7.P2.SP1`)

The UMCF API Gateway provides a versioned, secure, and low-latency entry point into the compute mesh. It supports HTTP/REST, WebSocket streaming event channels, and OpenAI-compatible inference routes.

```
+---------------------------------------------------------------------------------------------------------+
|                                           UMCF API GATEWAY ARCHITECTURE                                 |
|                                                                                                         |
|   [Client Applications / CI/CD]                                                                         |
|       |                     |                      |                                                    |
|       v                     v                      v                                                    |
|  [Rust SDK]          [TypeScript SDK]         [Python SDK]                                              |
|  (`umcf-sdk`)        (`@umcf/sdk`)            (`umcf`)                                                  |
|       \                     |                      /                                                    |
|        \                    |                     /                                                     |
|         v                   v                    v                                                      |
|   +-------------------------------------------------------------------------------------------------+   |
|   |                          API GATEWAY ROUTER (`crates/umcf-api-gateway`)                         |   |
|   |  - Port: 8080 (Configurable)                                                                    |   |
|   |  - OpenAPI 3.1: /api/v1/openapi.json                                                            |   |
|   |  - Interactive API Explorer: /api/v1/docs                                                       |   |
|   |                                                                                                 |   |
|   |  [Scoped Bearer Capability Token Validator] (`ApiKeyValidator`)                                 |   |
|   |  - Hash: BLAKE3(secret_token)                                                                   |   |
|   |  - Principle of Least Privilege: scopes enforced per endpoint                                   |   |
|   |  - Sub-millisecond Token-Bucket Rate Limiter                                                    |   |
|   |  - Standard RFC 9457 Problem Details (`application/problem+json`) on error                      |   |
|   |                                                                                                 |   |
|   |  [Event Ring Buffer & Resumption Engine] (`EventRingBuffer`)                                    |   |
|   |  - Monotonic global sequence numbers: seq = 1, 2, 3...                                          |   |
|   |  - WebSocket Endpoint: /ws/v1/jobs/{id}/events                                                  |   |
|   |  - Reconnection cursor: ?since_seq=N or Last-Event-ID header (zero missed events)               |   |
|   +-------------------------------------------------------------------------------------------------+   |
|            |                        |                      |                       |                    |
|            v                        v                      v                       v                    v
|   [Cluster Discovery]         [Job Scheduler]        [AI Inference]       [Storage Engine]      [GPU / Media]
|   - /cluster/nodes            - /jobs                - /ai/models         - /storage/objects    - /gpu/render
|   - /cluster/telemetry        - /jobs/{id}           - /v1/chat/completions - /storage/pins     - /media/transcode
|   - /cluster/pair             - /jobs/{id}/cancel                                               - /stream/sessions
+---------------------------------------------------------------------------------------------------------+
```

### 1.1 Capability Scopes Matrix
Every request must include an `Authorization: Bearer umcf_live_<hex>` header. Tokens are validated against their granted scopes before executing domain logic:

| Scope | Description | Applicable Endpoints |
| :--- | :--- | :--- |
| `cluster:read` | Read-only access to cluster topology, nodes, and telemetry | `GET /api/v1/cluster/nodes`<br>`GET /api/v1/cluster/telemetry` |
| `cluster:admin` | Device pairing and administrative cluster control | `POST /api/v1/cluster/pair` |
| `jobs:submit` | Submit new compute/WASM/media/GPU/AI jobs | `POST /api/v1/jobs`<br>`POST /api/v1/jobs/{id}/cancel` |
| `jobs:read` | Query job status, logs, and progress events | `GET /api/v1/jobs/{id}`<br>`GET /ws/v1/jobs/{id}/events` |
| `ai:inference` | Execute prompt inference and token generation against AI models | `GET /api/v1/ai/models`<br>`POST /api/v1/ai/models/pull`<br>`POST /v1/chat/completions`<br>`POST /v1/embeddings` |
| `gpu:render` | Dispatch distributed GPU shaders and tile rendering jobs | `POST /api/v1/gpu/render` |
| `media:transcode` | Submit segment-parallel media transcoding tasks | `POST /api/v1/media/transcode` |
| `storage:read` | Read content-addressed chunks, objects, and manifests | `GET /api/v1/storage/objects/{cid}` |
| `storage:write` | Ingest, write, or pin objects in the content-addressed store | `PUT /api/v1/storage/objects`<br>`POST /api/v1/storage/pins` |
| `stream:connect` | Connect to interactive remote desktop and game streaming sessions | `POST /api/v1/stream/sessions` |
| `admin:full` | Unrestricted administrative access across all endpoints | Superuser wildcard (satisfies all scopes) |

### 1.2 RFC 9457 Problem Details Error Format
All errors returned by `/api/v1/*` conform strictly to RFC 9457 (`Content-Type: application/problem+json`):

```json
{
  "type": "https://umcf.dev/errors/forbidden",
  "title": "Insufficient Capability Scope",
  "status": 403,
  "detail": "Required scope 'storage:write' is not granted to this token. Granted scopes: [ai:inference, jobs:read]",
  "error_code": "UMCF_AUTH_INSUFFICIENT_SCOPE"
}
```

Standard error codes:
- `UMCF_AUTH_MISSING_TOKEN`: 401 Unauthorized (Missing or malformed header).
- `UMCF_AUTH_TOKEN_EXPIRED`: 401 Unauthorized (Token lifetime exceeded).
- `UMCF_AUTH_TOKEN_REVOKED`: 401 Unauthorized (Key revoked by operator).
- `UMCF_AUTH_INSUFFICIENT_SCOPE`: 403 Forbidden (Missing prerequisite capability).
- `UMCF_RATE_LIMIT_EXCEEDED`: 429 Too Many Requests (Token bucket exhausted).
- `UMCF_RESOURCE_NOT_FOUND`: 404 Not Found (Job, Model, or CID absent).
- `UMCF_VALIDATION_ERROR`: 400 Bad Request (Payload parameter schema mismatch).

---

## 2. Polyglot SDK Quickstarts

### 2.1 Rust Client SDK (`umcf-sdk`)

Add to `Cargo.toml`:
```toml
[dependencies]
umcf-sdk = "0.1.0"
tokio = { version = "1", features = ["full"] }
```

Example usage:
```rust
use umcf_sdk::{UmcfClient, ChatRequest, ChatMessage, PinRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = UmcfClient::builder()
        .endpoint("http://127.0.0.1:8080")
        .api_key("umcf_live_9a8b1c2d...")
        .build()?;

    // 1. List connected cluster nodes
    let nodes = client.list_nodes().await?;
    println!("Fabric nodes online: {}", nodes.len());

    // 2. Submit a distributed WASM task
    let job = client.submit_wasm_job("bafk-wasm-vector-add", "main", serde_json::json!({ "n": 100 })).await?;
    println!("Submitted job: {}", job.job_id);

    // 3. Ingest data into Content-Addressed Storage
    let ingest = client.put_object(b"Enterprise compute payload 2026").await?;
    println!("Ingested CID: {}", ingest.root_cid);

    // 4. Pin object for guaranteed offline access
    client.pin_object(&PinRequest {
        cid: ingest.root_cid.clone(),
        mode: "permanent".to_string(),
        ttl_seconds: None,
        label: Some("offline_dataset.bin".to_string()),
    }).await?;

    Ok(())
}
```

### 2.2 TypeScript & JavaScript SDK (`@umcf/sdk`)

Install or import `@umcf/sdk`:
```typescript
import { UmcfClient } from "@umcf/sdk";

const client = new UmcfClient({
  endpoint: "http://127.0.0.1:8080",
  apiKey: "umcf_live_9a8b1c2d...",
});

// 1. Cluster nodes
const nodes = await client.cluster.listNodes();
console.log(`Connected nodes: ${nodes.length}`);

// 2. Submit WASM job
const job = await client.jobs.submitWasm("bafk-ts-module", "run", { iterations: 1000 });
console.log(`Job status: ${job.status}`);

// 3. Storage put and get
const data = new TextEncoder().encode("TypeScript SDK Storage Test");
const ingest = await client.storage.putObject(data);
const readback = await client.storage.getObject(ingest.root_cid);
console.log(`Reassembled bytes: ${readback.length}`);
```

### 2.3 Python SDK (`umcf`)

```python
from umcf import UmcfClient

client = UmcfClient(endpoint="http://127.0.0.1:8080", api_key="umcf_live_9a8b1c2d...")

# 1. Cluster discovery
nodes = client.cluster.list_nodes()
print(f"Nodes in fabric: {len(nodes)}")

# 2. Submit compute task
job = client.jobs.submit_wasm("bafk-py-module", "entrypoint", {"dim": 512})
print(f"Scheduled Job: {job.job_id}")

# 3. Content-addressed storage
data = b"Python SDK Ingestion Payload"
ingest = client.storage.put_object(data)
print(f"Root CID: {ingest.root_cid}")

# 4. Pin offline
pin = client.storage.pin(ingest.root_cid, mode="permanent")
print(f"Hydration status: {pin.hydration_status}")
```

---

## 3. WebSocket Event Ring Buffer & Cursor Resumption

The WebSocket stream at `GET /ws/v1/jobs/{id}/events` guarantees that clients never drop intermediate progress or lifecycle events during temporary network disconnections:
1. Every emitted event receives a monotonically increasing sequence integer (`seq = 1, 2, 3...`).
2. When reconnecting, provide the sequence number of the last acknowledged event via `?since_seq=N` or `Last-Event-ID: N`.
3. The gateway replays all missing events from `seq = N + 1` up to the present sequence from its in-memory bounded ring buffer before switching to real-time broadcast streaming.


---

## 4. Declarative Workload Manifests & Reproducible Job Templates (`S7.P2.SP3`)

UMCF supports defining complex, multi-stage distributed compute pipelines—combining FastCDC storage ingestion, sandboxed WASM analytics, GPU shader compute, distributed GGUF AI inference, and hardware-accelerated media transcoding—in a single declarative `umcf.job.yaml` (or `.toml`) manifest.

### 4.1 `apiVersion: umcf.io/v1` Manifest Specification

Workload manifests declare metadata, typed input parameters, confidentiality constraints, DAG steps, and exported outputs:

```yaml
apiVersion: umcf.io/v1
kind: DistributedWorkflow
metadata:
  name: hybrid-media-ai-pipeline
  version: "1.0.0"
  description: "Multi-stage Video -> Keyframes -> WASM Filter -> LLM Synthesis"
parameters:
  input_video:
    type: path
    required: true
    description: "Source video file to summarize"
  summary_language:
    type: string
    required: false
    default: "en"
  llm_model:
    type: string
    required: false
    default: "llama-3-8b-instruct.Q4_K_M.gguf"
privacy:
  mode: trusted-devices # strict-local | trusted-devices | cluster-encrypted
steps:
  - id: extract_keyframes
    kind: media.transcode
    inputs:
      source: "${params.input_video}"
    config:
      codec: h264
      resolution: "1280x720"
  - id: preprocess_wasm
    kind: wasm.mapreduce
    depends_on: [extract_keyframes]
    module: "cid://blake3:8f9a2b1c3d4e5f60"
    inputs:
      frames_cid: "${steps.extract_keyframes.output_cid}"
    config:
      action: "ocr_and_visual_descriptors"
    resources:
      max_memory_mb: 256
      timeout_sec: 60
  - id: summarize_llm
    kind: ai.inference
    depends_on: [preprocess_wasm]
    model: "${params.llm_model}"
    inputs:
      descriptors: "${steps.preprocess_wasm.output_text}"
    prompt_template: "Summarize in ${params.summary_language}: ${steps.preprocess_wasm.output_text}"
    sampling:
      temperature: 0.0
      seed: 42
  - id: export_report
    kind: storage.ingest
    depends_on: [summarize_llm]
    inputs:
      content: "${steps.summarize_llm.output_text}"
    config:
      format: markdown
outputs:
  report_cid: "${steps.export_report.output_cid}"
```

### 4.2 Static Validation & Kahn's Topological Stage Grouping
The manifest validator (`ManifestValidator`) enforces strict invariants prior to runtime execution:
1. **Unknown Field Denied (`#[serde(deny_unknown_fields)]`)**: Prevents typos or unrecognized fields from passing silently.
2. **Topological DAG Acyclicity**: Kahn's algorithm detects cycles (`A -> B -> C -> A`) and returns structured `ManifestError::CyclicDependency`.
3. **Stage Parallelism**: Independent steps with zero mutual dependencies are grouped into concurrent stages (`Vec<Vec<String>>`).
4. **Variable & Type Validation**: Asserts that all `${params.*}` and `${steps.<id>.<output>}` expressions reference valid entities, and that producer/consumer data types match.

### 4.3 Cryptographic Lockfile (`umcf.lock`) & Frozen Execution
Workflows can be locked using `umcf job lock <MANIFEST>` generating a cryptographically signed `umcf.lock`:
- Pinned BLAKE3 digests for all step definitions, configurations, and input bindings.
- Immutable CIDs and exact byte sizes for all referenced models, modules, and datasets.
- Ed25519 developer signature over canonical lockfile digest.
- Running with `--frozen-lockfile` guarantees 100% bit-identical reproducibility; any tampered step or artifact immediately aborts with `ManifestError::LockfileMismatch`.

### 4.4 Built-In Job Templates Catalog
UMCF ships 6 ready-to-use, tested templates accessible via `umcf job init --template <NAME>`:
1. `wasm-parallel-map-reduce`: Distributed MapReduce data crunching across heterogeneous cluster nodes.
2. `ai-batch-rag-embeddings`: Distributed document chunking, batched vector embeddings, and index generation.
3. `ai-private-document-qa`: `StrictLocalOnly` confidential document QA with on-device GGUF LLM synthesis.
4. `gpu-distributed-raytracer`: Tile-parallel 4K raytracing distributed across cluster GPU shaders.
5. `media-4k-av1-transcode`: GOP-segmented hardware-accelerated 4K AV1 video transcoding with Annex-B restitching.
6. `hybrid-media-ai-pipeline`: Multi-stage pipeline: Video -> Keyframe Extraction -> WASM Visual Filter -> LLM Synthesis.

### 4.5 Workflow Resumption & Cached Step Outputs
If a long-running pipeline is interrupted (e.g. node thermal cutoff or network partition), `umcf job resume <RUN_ID>` reuses all cached step CIDs from earlier stages, executing only the remaining uncompleted steps without duplicate compute.
