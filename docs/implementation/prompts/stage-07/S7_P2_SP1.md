# Subphase S7.P2.SP1 Execution Prompt: Rust/TypeScript/Python Client SDKs & Versioned gRPC/HTTP/WebSocket API

## 1. Role & Context
You are the API Gateway & Polyglot SDK Architect for UMCF. In Subphase `S7.P2.SP1`, you will implement the versioned API Gateway (`crates/umcf-api-gateway`) and the three official client SDKs: Rust (`crates/umcf-sdk`), TypeScript/JavaScript (`sdks/typescript`), and Python (`sdks/python`). External applications must be able to submit distributed jobs, stream AI tokens, dispatch GPU shaders, transcode media, and store content-addressed files with clean, strongly-typed async APIs.

## 2. Prerequisites & Dependencies
- `S7.P1` (`umcf-storage`) and Stages 1–6 core engines.
- `axum`, `tonic`, `utoipa` (OpenAPI 3.1 generation) in Rust workspace.

## 3. Implementation Tasks
1. **Versioned API Gateway (`crates/umcf-api-gateway/src/lib.rs`)**:
   - Expose unified gRPC (`tonic`) and HTTP/REST + WebSocket (`axum`) endpoints under `/api/v1/`:
     - `GET /api/v1/cluster/nodes`, `POST /api/v1/cluster/pair`, `GET /api/v1/cluster/telemetry`
     - `POST /api/v1/jobs`, `GET /api/v1/jobs/{id}`, `POST /api/v1/jobs/{id}/cancel`, `GET /ws/v1/jobs/{id}/events` (supports `Last-Event-ID` monotonic sequence cursor so reconnecting clients never miss an event)
     - `GET /api/v1/ai/models`, `POST /api/v1/ai/models/pull`, plus seamless mount of the Stage 5 OpenAI-compatible `/v1/chat/completions` and `/v1/embeddings` routes
     - `POST /api/v1/gpu/render`, `POST /api/v1/media/transcode`, `POST /api/v1/stream/sessions`
     - `PUT /api/v1/storage/objects`, `GET /api/v1/storage/objects/{cid}`, `POST /api/v1/storage/pins`
   - Implement **Scoped Bearer Capability Tokens (`ApiKeyValidator`)**:
     - Support fine-grained scopes (`cluster:read`, `jobs:submit`, `ai:inference`, `storage:read`, `storage:write`, `stream:connect`, `admin:full`) with token expiration and per-token token-bucket rate limiting.
   - Auto-generate `/api/v1/openapi.json` via `utoipa` and serve an embedded offline-capable API Explorer UI at `/api/v1/docs`.
2. **Rust Client SDK (`crates/umcf-sdk`)**:
   - Provide `UmcfClient::builder().endpoint(...).api_key(...).build()` with automatic retry/backoff, cursor-resumable `JobEventStream`, zero-copy streaming upload/download for `umcf-storage`, and typed builders (`WasmJobBuilder`, `AiChatBuilder`, `GpuRenderBuilder`, `MediaTranscodeBuilder`).
3. **TypeScript/JavaScript SDK (`sdks/typescript`)**:
   - Zero-dependency universal `fetch` / `WebSocket` / `ReadableStream` TypeScript package (`@umcf/sdk`) working in Node.js >= 20, Deno, Bun, and modern browsers, with `AsyncIterable` streaming for job events and AI chat completions.
4. **Python SDK (`sdks/python`)**:
   - Sync + `asyncio` Python package (`umcf`) using `httpx` and `pydantic>=2.0` models, providing `client.jobs.submit_and_wait(...)`, `async for token in client.ai.chat_stream(...)`, and `client.storage.upload_file(...)`.

## 4. Acceptance Criteria
- >= 95% line/branch coverage across `crates/umcf-api-gateway`, `crates/umcf-sdk`, `sdks/typescript`, and `sdks/python`; 100% test pass rate.
- Every endpoint enforces its required capability scope (e.g., a token with only `ai:inference` receives `403 Forbidden` with structured RFC 9457 Problem Details JSON when calling `POST /api/v1/storage/pins`).
- Reconnecting a WebSocket `/ws/v1/jobs/{id}/events` client with `?since_seq=N` replays exact missed events `[N+1 .. current]` from the bounded ring buffer without gaps or duplicates.
- Rust, TypeScript, and Python SDKs all pass an identical 15-scenario end-to-end conformance test matrix against a live UMCF daemon.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_api_key_capability_scope_and_rate_limiter_enforcement`
  - `test_rfc9457_problem_details_error_serialization`
  - `test_openapi_3_1_schema_completeness_and_validity`
  - `test_websocket_event_ring_buffer_cursor_replay`
- **`[INT]`**:
  - `test_rust_sdk_e2e_wasm_job_ai_stream_and_storage_roundtrip`
  - `test_typescript_sdk_e2e_conformance_suite`
  - `test_python_sdk_sync_and_async_e2e_conformance_suite`
- **`[SIM]`**:
  - Inject TCP connection resets every 200 ms during a 5-second streaming job event subscription across all 3 SDKs, verifying automatic reconnection and lossless `since_seq` event resumption.
- **`[HW]`**:
  - Measure API gateway request overhead (< 0.5 ms P99 routing latency on local loopback).
- **`[MANUAL]`**:
  - Open `/api/v1/docs` in browser, generate a scoped API key in Svelte 5 Admin Settings, and execute a live API call from the interactive docs explorer.

## 6. Required Evidence & Artifacts
- `artifacts/stage-07/p2/sp1_openapi_v1_schema.json`
- `artifacts/stage-07/p2/sp1_rust_ts_python_sdk_conformance.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document API authentication scopes, RFC 9457 error codes, and Rust/TS/Python SDK quickstarts in `docs/sdk/DEVELOPER_GUIDE.md` (Section 1).
- Mark `S7.P2.SP1` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If any `/api/v1/*` route bypasses capability scope verification, or if SDK streaming methods leak HTTP connections or file handles on cancellation.
- **Completion Gate**: All tests green with >= 95% coverage before proceeding to `S7.P2.SP2`.
