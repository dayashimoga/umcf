# Stage 07 Execution Prompt: Distributed Storage, Developer SDK/CLI, Marketplace, Accounting & Administration

## 1. Role & Context
You are the Principal Distributed Storage, Developer Platform & Economic Ledger Architect for the **Unified Mobile-First Compute Fabric (UMCF)**. Stage 7 implements the BLAKE3 content-addressed distributed storage fabric with Reed-Solomon erasure coding and quota enforcement (`crates/umcf-storage`), the polyglot Developer SDKs (Rust, TypeScript, Python), versioned API & full-featured `umcf` CLI (`crates/umcf-sdk`, `crates/umcf-cli`), and the cryptographic resource metering ledger, compute marketplace matching engine, and unified Flutter/Svelte 5 administration consoles (`crates/umcf-ledger`, `crates/umcf-marketplace`).

Every subphase in Stage 7 follows the strict UMCF hierarchy:
`Stage → Phase → Subphase → Implementation tasks → Acceptance criteria → Tests → Evidence → Documentation`.

## 2. Stage 07 Phase & Subphase Index

| Phase | Subphase | Prompt File | Title & Scope |
| :--- | :--- | :--- | :--- |
| **S7.P1** | — | `docs/implementation/prompts/stage-07/S7_P1_PROMPT.md` | **Content-Addressed Distributed Storage & Erasure Coding** |
| S7.P1 | S7.P1.SP1 | `docs/implementation/prompts/stage-07/S7_P1_SP1.md` | BLAKE3 Content-Addressed Chunk Store & Explicit Quota Enforcement |
| S7.P1 | S7.P1.SP2 | `docs/implementation/prompts/stage-07/S7_P1_SP2.md` | Reed-Solomon Erasure Coding, Replication & Encrypted Shard Placement |
| S7.P1 | S7.P1.SP3 | `docs/implementation/prompts/stage-07/S7_P1_SP3.md` | Offline Caching, Pinning & Garbage Collection |
| **S7.P2** | — | `docs/implementation/prompts/stage-07/S7_P2_PROMPT.md` | **Developer SDK, CLI & Programmable Job API** |
| S7.P2 | S7.P2.SP1 | `docs/implementation/prompts/stage-07/S7_P2_SP1.md` | Rust/TypeScript/Python Client SDKs & Versioned gRPC/HTTP/WebSocket API |
| S7.P2 | S7.P2.SP2 | `docs/implementation/prompts/stage-07/S7_P2_SP2.md` | Complete `umcf` CLI for Cluster, Job, Model, Storage & Streaming Control |
| S7.P2 | S7.P2.SP3 | `docs/implementation/prompts/stage-07/S7_P2_SP3.md` | Declarative Workload Manifests & Reproducible Job Templates |
| **S7.P3** | — | `docs/implementation/prompts/stage-07/S7_P3_PROMPT.md` | **Resource Accounting, Compute Marketplace Foundation & Admin Consoles** |
| S7.P3 | S7.P3.SP1 | `docs/implementation/prompts/stage-07/S7_P3_SP1.md` | Signed Cryptographic Resource Metering Receipts (CPU/GPU/NPU/Bandwidth/Storage) |
| S7.P3 | S7.P3.SP2 | `docs/implementation/prompts/stage-07/S7_P3_SP2.md` | Double-Entry Compute Credit Ledger & Marketplace Order Matching |
| S7.P3 | S7.P3.SP3 | `docs/implementation/prompts/stage-07/S7_P3_SP3.md` | Complete Flutter Mobile/Desktop UI & Svelte 5 Admin Dashboard Integration |

## 3. Prerequisites & Dependencies
- **Stage 06 Completion Gate**: All Stage 1–6 crates (`umcf-crypto`, `umcf-net`, `umcf-hw`, `umcf-scheduler`, `umcf-wasm`, `umcf-ai`, `umcf-gpu`, `umcf-media`, `umcf-stream`) passing at >= 95% coverage.
- **Toolchains**: Rust stable, Node.js >= 20 / pnpm (for TypeScript SDK & Svelte 5 Admin UI), Python >= 3.11 / Maturin or PyO3/httpx (for Python SDK), and Flutter >= 3.22 (for mobile/desktop UI).

## 4. Implementation Tasks (Stage-Level)
1. **Workspace Crates & Packages**:
   - `crates/umcf-storage`: FastCDC / fixed-chunk BLAKE3 store, strict hard-cap quota reservation engine, $RS(k, m)$ Reed-Solomon erasure coding with XChaCha20-Poly1305 convergent encryption, failure-domain-aware placement, offline pinning, and mark-and-sweep GC.
   - `crates/umcf-sdk`, `sdks/typescript`, `sdks/python`, `crates/umcf-cli`: Versioned API gateway (`/api/v1/*`), ergonomic client libraries, declarative YAML/TOML workload manifests (`umcf.job.yaml`), and the `umcf` binary CLI.
   - `crates/umcf-ledger` & `crates/umcf-marketplace`: Dual-signed cryptographic resource usage receipts, ACID double-entry credit ledger, escrow & slashing integration, and bid/ask compute order book.
2. **UI & Admin Consoles**:
   - Complete end-to-end integration of all screens in `ui/flutter_app` and `ui/admin_web` with real-time WebSocket/gRPC streams and zero mock data.

## 5. Acceptance Criteria
- >= 95.0% line and branch coverage across `crates/umcf-storage`, `crates/umcf-sdk`, `crates/umcf-cli`, `crates/umcf-ledger`, and `crates/umcf-marketplace` (plus >= 95% coverage in TypeScript & Python SDKs); 100% test pass rate.
- Storage quota enforcement guarantees disk usage never exceeds the user's configured byte cap by even 1 byte, even under 64 concurrent write streams.
- Reed-Solomon $RS(4, 2)$ erasure coding reconstructs original files with 100% BLAKE3 parity when any 2 storage nodes go offline simultaneously.
- Double-entry compute ledger preserves global zero-sum conservation of credits ($\sum \text{Debits} == \sum \text{Credits}$) across 10,000 concurrent settlements and slashings.

## 6. Required Tests
- **Unit (`[UNIT]`)**: FastCDC chunking determinism, atomic quota reservation CAS loop, GF($2^8$) Reed-Solomon encode/reconstruct, manifest schema validation, double-entry ledger invariant checks.
- **Integration (`[INT]`)**: Multi-node erasure-coded file put/get with node failure, Rust/TS/Python SDK end-to-end job submission & streaming, CLI command matrix, marketplace bid-ask matching & escrow settlement.
- **Simulation (`[SIM]`)**: 10-node churn & storage heal simulation; 1,000-agent marketplace auction stress test with Byzantine receipt forgery attempts.
- **Hardware-Required (`[HW]`)**: Storage throughput & GC latency on real NVMe and mobile UFS flash; Flutter mobile/desktop & Svelte 5 live dashboard telemetry.
- **Manually Verified (`[MANUAL]`)**: Full walkthrough of Flutter app and Svelte 5 admin console across Cluster, Jobs, AI, Render/Media, Stream, Storage, and Marketplace tabs.

## 7. Required Evidence & Artifacts
- `artifacts/stage-07/coverage/lcov.info` (>= 95% coverage).
- `artifacts/stage-07/storage/erasure_coding_heal_audit.json`.
- `artifacts/stage-07/sdk/polyglot_sdk_conformance_report.json`.
- `artifacts/stage-07/ledger/double_entry_conservation_proof.json`.

## 8. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Create `docs/architecture/DISTRIBUTED_STORAGE.md`, `docs/sdk/DEVELOPER_GUIDE.md`, and `docs/architecture/MARKETPLACE_AND_LEDGER.md`.
- Mark all `S7.P1.*`, `S7.P2.*`, and `S7.P3.*` items `[x]` in `TODO.md` and update `CHANGELOG.md`.

## 9. Rollback Criteria & Completion Gate
- **Rollback Criteria**: Any storage write exceeding the user's configured disk quota, any unencrypted shard stored on an untrusted node, or any ledger balance discrepancy.
- **Completion Gate**: Stage 08 (`stage-08`) may not begin until `S7.P1`, `S7.P2`, and `S7.P3` pass all completion gates.
