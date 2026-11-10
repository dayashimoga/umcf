# Executable Phase Prompt — `S1.P2`: Versioned Wire Protocol, Core Domain Types & Capability Schemas

> **Stage:** Stage 1 — Engineering Foundation, Repository and Development Infrastructure (`STAGE_01_PROMPT.md`)  
> **Phase:** `S1.P2` — Versioned Wire Protocol, Core Domain Types & Capability Schemas  
> **Subphases:** `S1.P2.SP1`, `S1.P2.SP2`, `S1.P2.SP3`  
> **Target Environment:** Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are the Lead Systems Architect executing Phase **`S1.P2` (Versioned Wire Protocol, Core Domain Types & Capability Schemas)** of Stage 1 in the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Phase Objective:** Implement the canonical wire protocol framing and semantic version negotiation (`umcf-proto`), strongly-typed hardware/resource domain descriptors (`umcf-core`), and structured OpenTelemetry tracing plus RFC-9457 error taxonomy (`umcf-telemetry`).
- **Execution Discipline:** Execute `S1.P2.SP1`, `S1.P2.SP2`, and `S1.P2.SP3` sequentially. Do not begin a subsequent subphase until the current subphase's Completion Gate and Evidence JSON are verified.

---

## 2. Prerequisites & Dependencies

- **Upstream Dependency:** `S1.P1` must be 100% complete and certified.
- **Target Crates / Services:** `crates/umcf-proto, crates/umcf-core, crates/umcf-telemetry`
- **Quota Enforcement:** `scripts/podman-quota-check.sh` must pass before and after each subphase (`< 29.0 GiB`).

---

## 3. Implementation Tasks & Subphase Execution Sequence

Execute the three constituent subphases in strict sequential order:

### 3.1 Subphase `S1.P2.SP1`: `umcf-proto` Envelope & Semantic Versioning Negotiation
- **Prompt File:** [`S1_P2_SP1.md`](S1_P2_SP1.md)
- **Scope:** Implement wire framing, magic-byte validation, binary/JSON codec abstraction, Ed25519 signature footer slots, and semantic version capability handshake in `crates/umcf-proto`.
- **Key Deliverables:**
  - Implement `crates/umcf-proto/src/envelope.rs` defining `WireEnvelope` with 4-byte magic header `UMCF` (`0x554D4346`), `ProtocolVersion { major: u16, minor: u16, patch: u16 }`, `MessageFlags` bitflags (`COMPRESSED_ZSTD`, `ENCRYPTED_PAYLOAD`, `REQUIRES_ACK`), `MessageId(u128)`, `TimestampHlc`, `SenderDid`, `PayloadType`, `payload: Bytes`, and `signature: [u8; 64]`.
  - Implement `crates/umcf-proto/src/negotiation.rs` defining `HandshakeOffer`, `HandshakeAccept`, `HandshakeReject`, and deterministic `negotiate_version(local_range, remote_range) -> Result<ProtocolVersion, NegotiationError>`.
  - Implement length-delimited async framing codec (`UmcfFrameCodec`) in `crates/umcf-proto/src/codec.rs` with configurable maximum frame size (default `16 MiB` control plane, `64 MiB` data plane) and BLAKE3 payload integrity verification.

### 3.2 Subphase `S1.P2.SP2`: `umcf-core` Hardware & Resource Descriptors
- **Prompt File:** [`S1_P2_SP2.md`](S1_P2_SP2.md)
- **Scope:** Define the canonical domain types in `crates/umcf-core` for node identities, device classes, CPU/GPU/NPU/RAM/storage/network/battery/thermal capability descriptors, and resource units.
- **Key Deliverables:**
  - Implement `crates/umcf-core/src/types.rs` defining `NodeId`, `ClusterId`, `JobId`, `TaskId`, `DeviceClass` (`Smartphone`, `Tablet`, `Laptop`, `Desktop`, `Workstation`, `Server`, `BrowserWasm`), and `OperatingSystem` (`Android`, `Ios`, `Windows`, `Linux`, `MacOs`, `Web`).
  - Implement `crates/umcf-core/src/capabilities.rs` defining `HardwareCapabilityDescriptor` containing `CpuCapability` (architecture, physical/performance/efficiency cores, AVX2/AVX512/NEON/SVE flags, base/boost MHz), `MemoryCapability` (total/available RAM bytes, unified memory flag, bandwidth MB/s), `AcceleratorCapability` (`ComputeBackend`: `Vulkan`, `Metal`, `Dx12`, `Cuda`, `Rocm`, `OpenCl`, `Nnapi`, `CoreMl`, `WebGpu`; VRAM bytes, FP32/FP16/INT8 TFLOPS, compute units), `StorageCapability`, `NetworkCapability`, and `PowerThermalState` (battery %, charging state, thermal state `Nominal | Fair | Serious | Critical`, temperature milli-Celsius).
  - Implement `crates/umcf-core/src/units.rs` providing checked arithmetic types `ByteSize(u64)`, `MilliCores(u32)`, `TeraFlopsFp16(u32)`, and JSON Schema export (`schemars::JsonSchema`) for TypeScript/Dart codegen.

### 3.3 Subphase `S1.P2.SP3`: Structured Telemetry, Tracing & Error Taxonomy
- **Prompt File:** [`S1_P2_SP3.md`](S1_P2_SP3.md)
- **Scope:** Implement `crates/umcf-telemetry` providing W3C TraceContext propagation across `WireEnvelope`, OpenTelemetry JSON/OTLP exporters, Prometheus/OpenMetrics counters/histograms, and standardized error codes (`UmcfError`).
- **Key Deliverables:**
  - Implement `crates/umcf-telemetry/src/error.rs` defining `ErrorCode` (`E1xxx` Protocol, `E2xxx` Identity/Auth, `E3xxx` Network, `E4xxx` Scheduler/Resource, `E5xxx` Inference, `E6xxx` Media/GPU, `E7xxx` Storage/Ledger, `E8xxx` Internal), `Retryability` (`RetryableAfter(Duration)`, `RetryableImmediate`, `Fatal`), and RFC-9457 Problem Details JSON serialization.
  - Implement `crates/umcf-telemetry/src/tracing_init.rs` configuring `tracing-subscriber` with structured JSON formatting, redactable sensitive fields (`SecretRedactor`), and W3C `traceparent` / `tracestate` injection/extraction into `umcf_proto::WireEnvelope`.
  - Implement `crates/umcf-telemetry/src/metrics.rs` providing lock-free atomic counters, gauges, and HDR latency histograms exportable in OpenMetrics text format.

---

## 4. Acceptance Criteria (Phase-Level Measurable Gates)

- [ ] All acceptance criteria in `S1_P2_SP1.md`, `S1_P2_SP2.md`, and `S1_P2_SP3.md` are satisfied.
- [ ] Cross-subphase integration between `S1.P2.SP1`, `S1.P2.SP2`, and `S1.P2.SP3` compiles cleanly with zero warnings (`cargo clippy --workspace --all-targets -- -D warnings`).
- [ ] Line coverage across crates modified in `S1.P2` is `>= 85.0%`.
- [ ] Zero `todo!()`, `unimplemented!()`, or placeholder stubs exist in the codebase (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Podman storage utilization remains strictly `< 29.0 GiB`.

---

## 5. Required Tests (Classified Rollup)

- **Unit Tests:** All unit test suites across `S1.P2.SP1`, `S1.P2.SP2`, and `S1.P2.SP3` pass via `cargo nextest run`.
- **Integration Tests:** End-to-end phase integration tests verifying ``umcf-proto` Envelope & Semantic Versioning Negotiation` + ``umcf-core` Hardware & Resource Descriptors` + `Structured Telemetry, Tracing & Error Taxonomy` interoperation.
- **Simulation Tests:** Fault-injection, corrupted input, and concurrency stress tests defined in `S1.P2.SP1..SP3`.
- **Hardware-Required Tests:** Executed on target hardware or recorded in hardware execution manifests where containerized hardware passthrough is unavailable.
- **Manually Verified Tests:** Developer visual/CLI inspection checklists completed for `S1.P2.SP1..SP3`.

---

## 6. Required Evidence & Artifacts

- `artifacts/evidence/stage-01/S1_P2_SP1_evidence.json`
- `artifacts/evidence/stage-01/S1_P2_SP2_evidence.json`
- `artifacts/evidence/stage-01/S1_P2_SP3_evidence.json`
- `artifacts/evidence/stage-01/S1_P2_PHASE_SUMMARY.json` aggregating all 3 subphase evidence bundles and phase-level coverage metrics.

---

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates

- Verify `S1.P2.SP1`, `S1.P2.SP2`, and `S1.P2.SP3` are all checked `[x]` in `TODO.md`.
- Consolidate `S1.P2` capabilities into `CHANGELOG.md` and module-level architecture docs.

---

## 8. Rollback Criteria & Completion Gate

- **Rollback Criteria:** If phase integration tests fail or Podman disk quota exceeds `29.0 GiB` after cache pruning, roll back to the start of `S1.P2` (`v0.1.1-certified`) and isolate the failing subphase.
- **Completion Gate:** Tag repository checkpoint `v0.1.2-certified` once `S1_P2_PHASE_SUMMARY.json` reports `"status": "PASS"`.
