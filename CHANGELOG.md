# CHANGELOG — UNIVERSAL MOBILE COMPUTE FABRIC (UMCF)

> **GOVERNANCE NOTICE:** This file is an **append-only** chronological record (`ENGINEERING_CONTRACT.md` §2.7). Existing entries must never be overwritten or deleted. All new stage, phase, subphase, and release completions must be appended below.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0-bootstrap] — 2026-11-10

### Added
- **Monorepo & Git Initialization**:
  - Initialized repository with `main` and `develop` branches, `.gitignore`, `.gitattributes`, and `.editorconfig`.
  - Established Rust workspace (`Cargo.toml` + 7 crates: `umcf-proto`, `umcf-core`, `umcf-coordinator`, `umcf-worker`, `umcf-scheduler`, `umcf-cli`, `umcf-daemon`).
  - Established Svelte 5 + TypeScript Web Dashboard (`apps/web-dashboard`) and Flutter Cross-Platform Mobile/Desktop application (`apps/mobile-desktop`).
  - Established Cloudflare Workers + Durable Objects primary control plane (`services/control-plane-cf`) and Google Cloud Run stateless fallback control plane (`services/control-plane-cloudrun`).
- **Engineering Governance & Antigravity Customizations**:
  - Created `ENGINEERING_CONTRACT.md`, `AGENTS.md`, `SECURITY.md`, `CONTRIBUTING.md`, `TODO.md`, and `CHANGELOG.md`.
  - Created 7 workspace rules in `.agent/rules/`, 9 domain skills in `.agent/skills/`, and 7 repeatable workflows in `.agent/workflows/` (including `stage-execution-workflow.md`).
- **Architecture, Requirements & 8-Stage Implementation Roadmap**:
  - Authored `docs/architecture/SYSTEM_OVERVIEW.md`, `COMPONENT_CONTRACTS.md`, `DATA_FLOWS.md`, `RISK_REGISTER_AND_PROTOTYPES.md`, and `ADR-0001` through `ADR-0006`.
  - Authored `docs/requirements/FUNCTIONAL_REQUIREMENTS.md` (`FR-001`..`FR-080`), `NON_FUNCTIONAL_REQUIREMENTS.md` (`NFR-001`..`NFR-040`), and `TRACEABILITY_MATRIX.md`.
  - Authored `docs/implementation/MASTER_ROADMAP.md` and 104 standalone executable Stage, Phase, and Subphase prompts across `docs/implementation/prompts/stage-01/` through `stage-08/`.
- **Reproducible Podman Environment (30 GB Cap) & CI/CD Workflows**:
  - Added `infra/podman/` multi-target container definitions and `scripts/enforce-podman-quota.ps1` / `scripts/enforce-podman-quota.sh` enforcing the strict 30 GB storage limit.
  - Added `.github/workflows/ci-rust.yml`, `ci-web.yml`, `ci-flutter.yml`, `ci-security-sbom.yml`, and `release.yml` with independent artifact uploads, SHA-256 checksums, build metadata, and SBOM generation.

---

## [Unreleased] — Stage 1 Foundation

### Added (S1.P1.SP1: Git/Governance/Rules/Skills Bootstrap)
- **Dual Licensing**: Added official `LICENSE-APACHE` (Apache-2.0) and `LICENSE-MIT` (MIT) manifests.
- **Root Developer Governance**: Added root `CONTRIBUTING.md` defining architectural invariants, commit format, and verification gates.
- **Automated Placeholder & Governance Gates**:
  - Implemented `scripts/check-no-placeholders.sh` and `scripts/check-no-placeholders.ps1` enforcing zero forbidden tokens (`todo!()`, `unimplemented!()`, `FIXME`) across production paths in `< 60ms`.
  - Implemented `scripts/verify-governance.sh` and `scripts/verify-governance.ps1` verifying presence and non-emptiness of 49 governance files and 72/72 subphases in `TODO.md`.
  - Implemented `scripts/podman-quota-check.sh` and `scripts/podman-quota-check.ps1` enforcing the 29.0 GiB safe threshold and 30 GiB hard ceiling.
- **Classified Test Harness**: Implemented `scripts/test-s1-p1-sp1.sh` and `scripts/test-s1-p1-sp1.ps1` covering `[Unit]`, `[Integration]`, `[Simulation]`, and `[Manually-Verified]` tiers with 100% pass rate.
- **Verification Evidence**: Archived `artifacts/evidence/stage-01/S1_P1_SP1_evidence.json` and `artifacts/evidence/stage-01/placeholder_scan_S1_P1_SP1.log`.

### Added (S1.P1.SP2: Podman 30GB Container Toolchains & Quota Enforcer)
- **Rootless Container Toolchains**:
  - `containers/Podmanfile.rust`: Minimal toolchain image with Rust 1.85, `wasm32-wasip1`, `cargo-nextest`, `cargo-llvm-cov`, and `sccache` (~1.15 GiB).
  - `containers/Podmanfile.web`: Node 22 LTS + pnpm + Wrangler + TypeScript (~780 MiB).
  - `containers/Podmanfile.flutter`: Flutter stable SDK + Linux desktop tools with stripped NDK symbols (~3.80 GiB).
  - `compose.podman.yml`: Multi-service configuration with rootless UID mapping (`userns_mode: keep-id`) and bounded volumes (`umcf_cargo_cache`, `umcf_sccache`).
- **Quota Enforcer & Auto-Prune**:
  - Enhanced `scripts/podman-quota-check.sh` and `scripts/podman-quota-check.ps1` with Soft Warning at 25.0 GiB (`26,843,545,600` bytes) and Hard Abort + Auto-Prune at 29.0 GiB (`31,138,512,896` bytes).
  - Standardized `--json` telemetry schema output matching the specification.
- **Classified Test Harness**: Implemented `scripts/test-s1-p1-sp2.sh` and `scripts/test-s1-p1-sp2.ps1` testing threshold math (20G/26G/29.5G), container syntax, 30.1 GiB fault injection, and rootless permissions with 100% pass rate.
- **Verification Evidence**: Archived `artifacts/evidence/stage-01/S1_P1_SP2_evidence.json` and `artifacts/evidence/stage-01/podman_images_digests.txt`.

### Added (S1.P1.SP3: Multi-Language Workspace Skeleton & Build Orchestration)
- **Rust Workspace Foundation**:
  - Configured root `Cargo.toml` with `[workspace]`, `resolver = "2"`, `[workspace.package]`, `[workspace.dependencies]`, and `[workspace.lints.rust]` (`unsafe_code = "forbid"`).
  - Scaffolded foundational crate `crates/umcf-telemetry` supporting W3C `traceparent` parsing/formatting, metric counters, structured log events, and unit tests.
  - Scaffolded fallback edge service `services/cloudrun_fallback` with typed envelope inspection and self-check unit tests.
- **Web Console & Edge Control Plane**:
  - Scaffolded `apps/web_console` with Svelte 5 runes (`$state`, `$derived`), Vite configuration, and passing unit test suites.
  - Scaffolded `services/cf_control_plane` with Wrangler configuration, TypeScript request handlers, and passing unit test suites.
  - Scaffolded `apps/flutter_client` with Flutter SDK configuration, strict analysis options, and Material 3 application shell.
- **Unified Build Orchestration**:
  - Added root `justfile`, `Makefile.toml`, and `Makefile` orchestrating `verify`, `lint`, `test`, `build-all`, `quota-check`, `check-placeholders`, and `verify-governance`.
- **Classified Test Harness**: Implemented `scripts/test-s1-p1-sp3.sh` and `scripts/test-s1-p1-sp3.ps1` covering `[Unit]`, `[Integration]`, `[Simulation]`, `[Hardware-Required]`, and `[Manually-Verified]` tiers with 100% pass rate.
- **Verification Evidence**: Archived `artifacts/evidence/stage-01/S1_P1_SP3_evidence.json` and `artifacts/evidence/stage-01/workspace_metadata.json`.

### Added (S1.P2.SP1: `umcf-proto` Wire Envelope, Framing & Semantic Versioning Negotiation)
- **Deterministic Wire Envelope & Framing (`crates/umcf-proto/src/envelope.rs`)**:
  - Implemented 92-byte fixed header layout with 4-byte magic bytes `UMCF` (`0x554D4346`), `ProtocolVersion` (major, minor, patch), `PlaneKind`, `MessageKind`, `MessageFlags` (NONE, COMPRESSED_ZSTD, ENCRYPTED_PAYLOAD, REQUIRES_ACK), `MessageId` (128-bit), `TimestampHlc` (physical ms + logical counter), BLAKE3 payload digest, FNV-1a checksum, and 64-byte Ed25519 signature footer slot.
  - Zero-copy payload borrowing and transparent Zstd compression/decompression on demand.
  - Micro-benchmark latency: **1.286 µs** per 1 KiB roundtrip in release mode (target: `< 2.5 µs`).
- **Semantic Version Range Handshake (`crates/umcf-proto/src/negotiation.rs`)**:
  - Implemented `HandshakeOffer`, `HandshakeAccept`, `HandshakeReject`, and deterministic `negotiate_version` selecting the highest mutually supported `(major, minor)` version while strictly rejecting mismatched major versions with `NegotiationError::IncompatibleMajorVersion`.
- **Async Streaming Framing Codec (`crates/umcf-proto/src/codec.rs`)**:
  - Implemented `UmcfFrameCodec` for `tokio_util::codec::Decoder` and `Encoder` with configurable maximum frame limits (16 MiB control plane, 64 MiB data plane).
  - O(1) header inspection immunizing against `0xFFFFFFFF` OOM payload length denial-of-service attacks before buffer allocation.
- **Classified Test Harness**: Implemented `scripts/test-s1-p2-sp1.sh` and `scripts/test-s1-p2-sp1.ps1` covering `[Unit]`, `[Integration]` (10,000 envelopes streamed over duplex stream), `[Simulation]` (10,000 bit-flip mutations), `[Hardware-Required]` (SIMD acceleration), and `[Manually-Verified]` (Rustdoc generation) with 100% pass rate.
- **Verification Evidence**: Archived `artifacts/evidence/stage-01/S1_P2_SP1_evidence.json` and `artifacts/evidence/stage-01/umcf_proto_test_report.xml`.

### Added (S1.P2.SP2: `umcf-core` Hardware & Resource Descriptors)
- **Canonical Domain Identifiers (`crates/umcf-core/src/types.rs`)**:
  - Implemented `NodeId`, `ClusterId`, `JobId`, `TaskId`, `DeviceClass` (`Smartphone`, `Tablet`, `Laptop`, `Desktop`, `Workstation`, `Server`, `BrowserWasm`), and `OperatingSystem` (`Android`, `Ios`, `Windows`, `Linux`, `MacOs`, `Web`).
- **Checked Arithmetic Units (`crates/umcf-core/src/units.rs`)**:
  - Implemented `ByteSize`, `MilliCores`, `TeraFlopsFp16`, and `MegabytesPerSec` with checked addition, subtraction, multiplication, and division preventing integer overflow/underflow across all operations.
- **Heterogeneous Hardware Capability Descriptors (`crates/umcf-core/src/capabilities.rs`)**:
  - Implemented `HardwareCapabilityDescriptor` covering `CpuCapability` (SIMD AVX2/AVX512/NEON/SVE flags, P/E cores, clock MHz), `MemoryCapability` (unified memory flag, bandwidth), `AcceleratorCapability` (Vulkan, Metal, Dx12, Cuda, Rocm, OpenCl, Nnapi, CoreMl, WebGpu), `StorageCapability`, `NetworkCapability`, and `PowerThermalState` (`ThermalState` Nominal, Fair, Serious, Critical).
  - Implemented `HardwareCapabilityDescriptor::satisfies(&ResourceRequirement)` accurately evaluating multi-dimensional hardware constraints across 25 device profiles.
  - Achieved binary (`bincode`) and canonical JSON (`serde_json`) serialization parity.
  - Exported standard Draft 2020-12 JSON Schema to `schemas/hardware_capability_v1.schema.json` for TypeScript and Flutter codegen.
- **Classified Test Harness**: Implemented `scripts/test-s1-p2-sp2.sh` and `scripts/test-s1-p2-sp2.ps1` covering `[Unit]` (25 device profiles + checked arithmetic), `[Integration]` (`WireEnvelope` embedding + schema export), `[Simulation]` (boundary overflows & critical thermal rejection), `[Hardware-Required]` (CPU logical core probing), and `[Manually-Verified]` (Draft 2020-12 JSON Schema validation) with 100% pass rate.
- **Verification Evidence**: Archived `artifacts/evidence/stage-01/S1_P2_SP2_evidence.json` and `schemas/hardware_capability_v1.schema.json`.

### Added (S1.P2.SP3: Structured Telemetry, OpenTelemetry Tracing & Error Taxonomy)
- **Standardized Error Taxonomy & RFC-9457 Problem Details (`crates/umcf-telemetry/src/error.rs`)**:
  - Implemented 27 exhaustive error codes across 8 categories: `E1xxx` (Protocol: `E1001`..`E1005`), `E2xxx` (Identity/Auth: `E2001`..`E2004`), `E3xxx` (Network: `E3001`..`E3003`), `E4xxx` (Scheduler/Resource: `E4001`..`E4004`), `E5xxx` (Inference: `E5001`..`E5003`), `E6xxx` (Media/GPU: `E6001`..`E6003`), `E7xxx` (Storage/Ledger: `E7001`..`E7003`), and `E8xxx` (Internal: `E8001`..`E8002`).
  - Mapped every error code to deterministic HTTP status codes (RFC-9110), gRPC status codes, and `Retryability` classifications (`Immediate`, `After(Duration)`, `Fatal`).
  - Implemented RFC-9457 `ProblemDetails` builder and serialized machine-readable catalog exportable via `all_error_taxonomy_entries()`.
- **W3C TraceContext Propagation & Sensitive Secret Redaction (`crates/umcf-telemetry/src/tracing_init.rs`)**:
  - Implemented W3C `traceparent` (version `00`, 16-byte `trace-id`, 8-byte `parent-id`, 1-byte `trace-flags`) injection into and extraction from `umcf_proto::WireEnvelope` headers, preserving end-to-end distributed trace continuity across asynchronous network boundaries.
  - Implemented `Redacted<T>` memory wrapper enforcing total leak prevention: displays `[REDACTED]` in `Display`, `[REDACTED_SECRET]` in `Debug` and JSON serialization, with audited `.expose_secret()` access.
- **Lock-Free OpenMetrics Telemetry Engine (`crates/umcf-telemetry/src/metrics.rs`)**:
  - Implemented thread-safe, lock-free atomic `Counter`, `Gauge`, and `LatencyHistogram` using atomic primitives (`AtomicU64`, `Ordering::Relaxed`/`SeqCst`) with zero mutex contention across hardware threads.
  - Implemented `MetricsRegistry` with OpenMetrics text format exporter formatting `# HELP`, `# TYPE`, gauge values, counter values, and HDR histogram buckets.
- **Classified Test Harness**: Implemented `scripts/test-s1-p2-sp3.sh` and `scripts/test-s1-p2-sp3.ps1` covering `[Unit]` (error taxonomy, RFC-9457, metrics), `[Integration]` (W3C traceparent round-trip across `WireEnvelope`, catalog export), `[Simulation]` (synthetic secret redaction leak prevention), `[Hardware-Required]` (14 hardware threads lock-free atomic concurrency verification), and `[Manually-Verified]` (RFC-9457 structure & OpenMetrics format compliance) with 100% pass rate.
- **Verification Evidence**: Archived `artifacts/evidence/stage-01/S1_P2_SP3_evidence.json` and `artifacts/evidence/stage-01/error_taxonomy_catalog.json`.


