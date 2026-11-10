# Phase S7.P2 Execution Prompt: Developer SDK, CLI & Programmable Job API

## 1. Role & Context
You are the Developer Experience (DX), API Gateway & SDK Platform Lead for UMCF. Phase `S7.P2` exposes the entire Unified Mobile-First Compute Fabric to external developers, CI/CD pipelines, and power users via a versioned gRPC/REST/WebSocket API gateway, idiomatic client SDKs in **Rust**, **TypeScript/JavaScript**, and **Python**, a comprehensive `umcf` command-line interface (`crates/umcf-cli`), and declarative reproducible workload manifests (`umcf.job.yaml` / `.toml`).

This phase consists of three sequential subphases:
1. `S7_P2_SP1.md`: Rust/TypeScript/Python Client SDKs & Versioned gRPC/HTTP/WebSocket API
2. `S7_P2_SP2.md`: Complete `umcf` CLI for Cluster, Job, Model, Storage & Streaming Control
3. `S7_P2_SP3.md`: Declarative Workload Manifests & Reproducible Job Templates

## 2. Prerequisites & Dependencies
- Completion of Phase `S7.P1` (`umcf-storage`) and Stages 1–6 (`umcf-scheduler`, `umcf-wasm`, `umcf-ai`, `umcf-gpu`, `umcf-media`, `umcf-stream`).

## 3. Implementation Tasks
1. **Subphase S7.P2.SP1 (`crates/umcf-api-gateway`, `crates/umcf-sdk`, `sdks/typescript`, `sdks/python`)**:
   - Implement the unified versioned API gateway (`/api/v1/*` HTTP/JSON + OpenAPI 3.1 spec, `/ws/v1/*` multiplexed WebSocket event streams, and `umcf.api.v1.*` gRPC services) with scoped API key / capability token authentication and rate limiting.
   - Implement first-party client SDKs (`crates/umcf-sdk` in Rust, `@umcf/sdk` in TypeScript, `umcf` package in Python) with typed methods for cluster discovery, WASM/Container/GPU/Media job submission, streaming logs/progress, AI inference/embeddings, and content-addressed storage put/get/pin.
2. **Subphase S7.P2.SP2 (`crates/umcf-cli`)**:
   - Build the `umcf` CLI binary supporting human-friendly tables/TUI progress bars (`--format text`) and scriptable JSON (`--format json`) across subcommands: `umcf node`, `umcf cluster`, `umcf job`, `umcf ai`, `umcf render`, `umcf media`, `umcf stream`, `umcf storage`, `umcf market`, and `umcf config` + shell completions (`bash`, `zsh`, `fish`, `powershell`).
3. **Subphase S7.P2.SP3 (`crates/umcf-manifest`)**:
   - Implement declarative workload manifest parser, variable interpolator, lockfile resolver (`umcf.lock` pinning exact BLAKE3 hashes of WASM modules, GGUF models, shaders, and input datasets), and DAG workflow runner for multi-step reproducible pipelines.

## 4. Acceptance Criteria
- >= 95% line/branch coverage across `crates/umcf-api-gateway`, `crates/umcf-sdk`, `crates/umcf-cli`, `crates/umcf-manifest`, `sdks/typescript`, and `sdks/python`; 100% test pass rate.
- 100% parity between the generated OpenAPI 3.1 schema (`openapi.v1.json`), Protobuf definitions, and the Rust/TypeScript/Python SDK client types.
- Every `umcf` CLI command supports `--format json`, deterministic exit codes (`0` success, `2` validation error, `3` auth error, `4` quota/resource error, `5` network timeout), and `--dry-run`.
- Executing a locked `umcf.job.yaml` manifest with `umcf.lock` verifies 100% of input asset/model/module BLAKE3 hashes before scheduling a single task.

## 5. Required Tests
- **`[UNIT]`**: API token scope checking, OpenAPI schema validation, CLI argument & flag parsing, manifest DAG cycle detector, lockfile hash verifier.
- **`[INT]`**: Polyglot SDK integration suite (running identical WASM map-reduce, AI chat stream, and storage put/get workflows from Rust, TypeScript/Node, and Python against a live test daemon); full CLI command end-to-end tests.
- **`[SIM]`**: WebSocket stream reconnection with `Last-Event-ID` cursor replay after simulated network drop.
- **`[HW]`**: Interactive CLI TUI progress bar and `umcf top` live cluster monitor on terminal.
- **`[MANUAL]`**: Run `umcf init`, `umcf job run umcf.job.yaml`, and `umcf top` in PowerShell / Bash and inspect Swagger/Scalar API docs served at `/api/v1/docs`.

## 6. Required Evidence & Artifacts
- `artifacts/stage-07/p2/openapi_v1_spec.json`
- `artifacts/stage-07/p2/polyglot_sdk_e2e_results.json`
- `artifacts/stage-07/p2/cli_command_matrix_verification.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Create `docs/sdk/DEVELOPER_GUIDE.md` and `docs/cli/UMCF_CLI_REFERENCE.md`.
- Mark `S7.P2.SP1`–`S7.P2.SP3` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: Any breaking change to `/api/v1/*` contract without version negotiation, any unauthenticated endpoint exposure, or SDK resource leaks on unclosed streams.
- **Gate**: All 3 subphases (`S7_P2_SP1`..`SP3`) pass with >= 95% coverage before starting Phase `S7.P3`.
