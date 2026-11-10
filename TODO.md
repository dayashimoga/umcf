# TODO — UNIVERSAL MOBILE COMPUTE FABRIC (UMCF)

> **GOVERNANCE NOTICE:** This file is an **append-only** engineering record (`ENGINEERING_CONTRACT.md` §2.7). Never delete historical entries. Mark completed items with `[x]`, completion date, and verification reference, and append new tasks or stage updates chronologically at the bottom of the relevant section.

---

## Bootstrap & Stage 0 Foundation (`2026-11-10`)

- [x] **[BOOT-01]** (`2026-11-10`) Inspect workspace (`g:\ucf`), host hardware (Intel Core Ultra 5 225H, 14 cores, 16 GB RAM, Intel Arc 130T GPU), Podman (`v5.8.3` with 30 GiB machine disk cap), Rust (`1.97.1`), and Node (`v20.18.0`).
- [x] **[BOOT-02]** (`2026-11-10`) Initialize Git monorepo (`main` and `develop` branches), `.gitignore`, `.gitattributes`, `.editorconfig`, and directory hierarchy.
- [x] **[BOOT-03]** (`2026-11-10`) Author `ENGINEERING_CONTRACT.md`, `AGENTS.md`, `SECURITY.md`, `README.md`, `CONTRIBUTING.md`, `docs/USER_GUIDE.md`, and `docs/DEVELOPER_GUIDE.md`.
- [x] **[BOOT-04]** (`2026-11-10`) Create Antigravity rules (`.agent/rules/00..06`), skills (`.agent/skills/umcf-*`), and workflows (`.agent/workflows/*`).
- [x] **[BOOT-05]** (`2026-11-10`) Create architecture documentation (`SYSTEM_OVERVIEW.md`, `COMPONENT_CONTRACTS.md`, `DATA_FLOWS.md`, `RISK_REGISTER_AND_PROTOTYPES.md`, and `ADR-0001` through `ADR-0006`).
- [x] **[BOOT-06]** (`2026-11-10`) Create traceable requirements (`FUNCTIONAL_REQUIREMENTS.md`, `NON_FUNCTIONAL_REQUIREMENTS.md`, `TRACEABILITY_MATRIX.md`).
- [x] **[BOOT-07]** (`2026-11-10`) Create `docs/implementation/MASTER_ROADMAP.md` and all 104 executable Stage, Phase, and Subphase prompt files across `docs/implementation/prompts/stage-01/` through `stage-08/`.
- [x] **[BOOT-08]** (`2026-11-10`) Implement Podman reproducible environment (`infra/podman/`) and 30 GB storage quota enforcement scripts (`scripts/enforce-podman-quota.ps1`, `scripts/enforce-podman-quota.sh`).
- [x] **[BOOT-09]** (`2026-11-10`) Implement baseline Rust workspace (`umcf-proto`, `umcf-core`, `umcf-coordinator`, `umcf-worker`, `umcf-scheduler`, `umcf-cli`, `umcf-daemon`), Svelte 5 web dashboard (`apps/web-dashboard`), Flutter multi-platform app (`apps/mobile-desktop`), and Cloudflare/CloudRun control planes (`services/`).
- [x] **[BOOT-10]** (`2026-11-10`) Create complete GitHub Actions CI/CD workflows (`.github/workflows/ci-rust.yml`, `ci-web.yml`, `ci-flutter.yml`, `ci-security-sbom.yml`, `release.yml`) and issue/PR templates.
- [x] **[BOOT-11]** (`2026-11-10`) Execute initial builds, unit/integration tests, Podman 30 GB quota check, and independent artifact packaging verification; produce `docs/certification/BOOTSTRAP_CERTIFICATION_REPORT.md`.
- [x] **[BOOT-12]** (`2026-11-10`) Add `.agent/workflows/stage-execution-workflow.md` reusable stage execution workflow to Antigravity workflow collection.

---

## Stage 1 — Engineering Foundation, Repository & Development Infrastructure

- [x] **[S1.P1.SP1]** (`2026-11-10`) Repository Governance, Rules/Skills Validation & Branch Protection Verification (Evidence: `artifacts/evidence/stage-01/S1_P1_SP1_evidence.json`)
- [x] **[S1.P1.SP2]** (`2026-11-10`) Podman 30 GB Container Toolchains & Quota Enforcer Hardening (Evidence: `artifacts/evidence/stage-01/S1_P1_SP2_evidence.json`)
- [x] **[S1.P1.SP3]** (`2026-11-10`) Multi-Language Workspace Build Orchestration & Task Runner (Evidence: `artifacts/evidence/stage-01/S1_P1_SP3_evidence.json`)
- [x] **[S1.P2.SP1]** (`2026-11-10`) `umcf-proto` Wire Envelope, Framing & Semantic Versioning Negotiation (Evidence: `artifacts/evidence/stage-01/S1_P2_SP1_evidence.json`)
- [x] **[S1.P2.SP2]** (`2026-11-10`) `umcf-core` Hardware & Resource Capability Descriptors (Evidence: `artifacts/evidence/stage-01/S1_P2_SP2_evidence.json`)
- [x] **[S1.P2.SP3]** (`2026-11-10`) Structured Telemetry, OpenTelemetry Tracing & Error Taxonomy (Evidence: `artifacts/evidence/stage-01/S1_P2_SP3_evidence.json`)
- [x] **[S1.P3.SP1]** (`2026-11-10`) Rust/Web/Control-Plane CI & Coverage Gates (Evidence: `artifacts/evidence/stage-01/S1_P3_SP1_evidence.json`)
- [ ] **[S1.P3.SP2]** Flutter Multi-OS CI & Independent Artifact Packaging
- [ ] **[S1.P3.SP3]** Release Manifest, CycloneDX/SPDX SBOM Generation & Stage 1 Certification

## Stage 2 — Rust Core, Identity, Enrollment, Device Registry & Cluster Management

- [ ] **[S2.P1.SP1]** Ed25519/X25519 Node Identity & DID Generation
- [ ] **[S2.P1.SP2]** Platform Keychain/Keystore & Encrypted Local Vault
- [ ] **[S2.P1.SP3]** Mutual Attestation & Signed Identity Certificates
- [ ] **[S2.P2.SP1]** Short-Lived QR Enrollment Token Generator & Scanner Flow
- [ ] **[S2.P2.SP2]** Fine-Grained RBAC & Explicit Resource-Sharing Consent Engine
- [ ] **[S2.P2.SP3]** Dynamic Master/Worker/Hybrid State Machine & Coordinator Election
- [ ] **[S2.P3.SP1]** Local Autonomous Coordinator & Embedded ACID State Store
- [ ] **[S2.P3.SP2]** Cloudflare Workers + Durable Objects Control Plane Sync
- [ ] **[S2.P3.SP3]** Google Cloud Run Fallback Control Plane & Split-Brain Reconciliation

## Stage 3 — LAN/Remote Networking, Security, Hardware Discovery & Benchmarking

- [ ] **[S3.P1.SP1]** Zero-Conf mDNS/DNS-SD & Signed UDP Broadcast LAN Discovery
- [ ] **[S3.P1.SP2]** Quinn/QUIC Multiplexed Data-Plane Transport with mTLS 1.3
- [ ] **[S3.P1.SP3]** Control-Plane vs Data-Plane Traffic Isolation & QoS Framing
- [ ] **[S3.P2.SP1]** STUN/ICE Candidate Gathering & UDP Hole Punching
- [ ] **[S3.P2.SP2]** Encrypted TURN/DERP-Style Relay Fallback Protocol
- [ ] **[S3.P2.SP3]** Roaming, Network Interface Handover & QUIC Connection Migration
- [ ] **[S3.P3.SP1]** Cross-Platform CPU/RAM/Storage/Battery/Thermal Probing
- [ ] **[S3.P3.SP2]** GPU & NPU Compute Enumeration (Vulkan/Metal/DX12/OpenCL/NNAPI/CoreML)
- [ ] **[S3.P3.SP3]** Real Compute/Memory/Network Micro-Benchmarks & Signed Attestation

## Stage 4 — Distributed Scheduler, Resource Management, Workload Execution & Recovery

- [ ] **[S4.P1.SP1]** CPU/Memory/VRAM Cgroups & Windows Job Object Quota Enforcer
- [ ] **[S4.P1.SP2]** Thermal Throttling, Battery Guard & Mobile OS Lifecycle Hooks
- [ ] **[S4.P1.SP3]** Interactive User Activity Detector & Instant Preemption
- [ ] **[S4.P2.SP1]** Capability-Negotiated Constraint Solver & Heterogeneous Cost Model
- [ ] **[S4.P2.SP2]** Bandwidth/Latency-Aware Workload DAG Partitioning
- [ ] **[S4.P2.SP3]** Queue Prioritization, Backpressure & Admission Control
- [ ] **[S4.P3.SP1]** Isolated Worker Execution Sandbox & Content-Addressed Artifact Staging
- [ ] **[S4.P3.SP2]** Heartbeat Failure Detection, Checkpointing & Speculative Retry
- [ ] **[S4.P3.SP3]** End-to-End Distributed Job Execution Verification

## Stage 5 — Real Distributed AI Inference, Model Management & Private AI

- [ ] **[S5.P1.SP1]** Content-Addressed Model Store & Resumable P2P Shard Distribution
- [ ] **[S5.P1.SP2]** GGUF Metadata Inspector, Memory Footprint Estimator & Compatibility Validator
- [ ] **[S5.P1.SP3]** Model Integrity Verification, Licensing & Access Control
- [ ] **[S5.P2.SP1]** Safe Rust FFI Bindings to `llama.cpp` & GGML Backend Loader
- [ ] **[S5.P2.SP2]** Hardware Backend Selection (CPU AVX/NEON, Vulkan, Metal, CUDA/ROCm) with Fallback
- [ ] **[S5.P2.SP3]** Streaming Token Generation, KV-Cache Management & Cancellation
- [ ] **[S5.P3.SP1]** Memory-Aware Layer Partitioning Across Heterogeneous Nodes
- [ ] **[S5.P3.SP2]** Distributed `llama.cpp` RPC Activation Transfer & Fault Recovery
- [ ] **[S5.P3.SP3]** Private AI Local-Only Mode, Prompt Encryption & OpenAI-Compatible Local API

## Stage 6 — GPU Compute, Rendering, Media Processing & Remote Application/Game Streaming

- [ ] **[S6.P1.SP1]** WGPU/Vulkan/Metal Compute Shader Dispatch & Buffer Validation
- [ ] **[S6.P1.SP2]** Tile/Frame-Partitioned Distributed Rendering Coordinator & Compositor
- [ ] **[S6.P1.SP3]** Deterministic Verification & Checksum Auditing of Rendered Outputs
- [ ] **[S6.P2.SP1]** Hardware-Accelerated Video/Audio Codec Probing & FFmpeg/Native Adapter
- [ ] **[S6.P2.SP2]** Segment-Parallel Video Transcoding, Stitching & Audio Sync
- [ ] **[S6.P2.SP3]** Media Pipeline Fault Recovery & Quality Verification
- [ ] **[S6.P3.SP1]** Display Capture & Hardware-Accelerated H.264/HEVC/AV1 Encoding Host
- [ ] **[S6.P3.SP2]** Ultra-Low-Latency QUIC/WebRTC Media Transport & Adaptive Bitrate
- [ ] **[S6.P3.SP3]** Multi-Modal Input Return Channel (Touch, Gamepad, Keyboard, Mouse, Stylus)

## Stage 7 — Distributed Storage, Developer SDK/CLI, Marketplace, Accounting & Administration

- [ ] **[S7.P1.SP1]** BLAKE3 Content-Addressed Chunk Store & Explicit Quota Enforcement
- [ ] **[S7.P1.SP2]** Reed-Solomon Erasure Coding, Replication & Encrypted Shard Placement
- [ ] **[S7.P1.SP3]** Offline Caching, Pinning & Garbage Collection
- [ ] **[S7.P2.SP1]** Rust/TypeScript/Python Client SDKs & Versioned gRPC/HTTP/WebSocket API
- [ ] **[S7.P2.SP2]** Complete `umcf` CLI for Cluster, Job, Model, Storage & Streaming Control
- [ ] **[S7.P2.SP3]** Declarative Workload Manifests & Reproducible Job Templates
- [ ] **[S7.P3.SP1]** Signed Cryptographic Resource Metering Receipts (CPU/GPU/NPU/Bandwidth/Storage)
- [ ] **[S7.P3.SP2]** Double-Entry Compute Credit Ledger & Marketplace Order Matching
- [ ] **[S7.P3.SP3]** Complete Flutter Mobile/Desktop UI & Svelte 5 Admin Dashboard Integration

## Stage 8 — Cross-Platform Integration, Optimization, Security Hardening & Production Certification

- [ ] **[S8.P1.SP1]** Multi-OS Cluster Integration Suite (Android, iOS, Windows, Linux, macOS, Web)
- [ ] **[S8.P1.SP2]** Network Partition, Node Churn, Thermal Throttle & Memory Pressure Chaos Tests
- [ ] **[S8.P1.SP3]** Long-Haul Soak Testing & Zero-Leak Memory/FD Profiling
- [ ] **[S8.P2.SP1]** Protocol Parser & FFI Fuzzing (`cargo-fuzz` / Proptest)
- [ ] **[S8.P2.SP2]** Penetration Testing, Sandbox Escape Verification & RBAC Bypass Audit
- [ ] **[S8.P2.SP3]** Dependency License Compliance, Reproducible Build Attestation & SBOM Signing
- [ ] **[S8.P3.SP1]** Data-Plane Zero-Copy Optimization & Mobile Battery Impact Tuning
- [ ] **[S8.P3.SP2]** Accessibility, Localization & UI/UX Polish Verification Across Form Factors
- [ ] **[S8.P3.SP3]** Final Hardware Certification Matrix, Release Readiness Audit & v1.0 Sign-Off

