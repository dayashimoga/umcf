# Stage 08 Execution Prompt: Cross-Platform Integration, Optimization, Security Hardening & Production Certification

## 1. Role & Context
You are the Principal Release Engineering, Chaos Resilience & Security Certification Lead for the **Unified Mobile-First Compute Fabric (UMCF)**. Stage 8 is the final production hardening and v1.0 certification stage of UMCF. It executes cross-OS multi-node integration and chaos engineering suites (`S8.P1`), continuous protocol/FFI fuzzing, sandbox penetration testing, and supply-chain SBOM attestation (`S8.P2`), and zero-copy data-plane optimization, mobile battery tuning, WCAG AA accessibility/localization polish, and the final v1.0 Hardware Certification Sign-Off (`S8.P3`).

Every subphase in Stage 8 follows the strict UMCF hierarchy:
`Stage → Phase → Subphase → Implementation tasks → Acceptance criteria → Tests → Evidence → Documentation`.

## 2. Stage 08 Phase & Subphase Index

| Phase | Subphase | Prompt File | Title & Scope |
| :--- | :--- | :--- | :--- |
| **S8.P1** | — | `docs/implementation/prompts/stage-08/S8_P1_PROMPT.md` | **Cross-Platform End-to-End Integration & Chaos Resilience Testing** |
| S8.P1 | S8.P1.SP1 | `docs/implementation/prompts/stage-08/S8_P1_SP1.md` | Multi-OS Cluster Integration Suite (Android, iOS, Windows, Linux, macOS, Web) |
| S8.P1 | S8.P1.SP2 | `docs/implementation/prompts/stage-08/S8_P1_SP2.md` | Network Partition, Node Churn, Thermal Throttle & Memory Pressure Chaos Tests |
| S8.P1 | S8.P1.SP3 | `docs/implementation/prompts/stage-08/S8_P1_SP3.md` | Long-Haul Soak Testing & Zero-Leak Memory/FD Profiling |
| **S8.P2** | — | `docs/implementation/prompts/stage-08/S8_P2_PROMPT.md` | **Security Hardening, Fuzzing & Cryptographic Audit** |
| S8.P2 | S8.P2.SP1 | `docs/implementation/prompts/stage-08/S8_P2_SP1.md` | Protocol Parser & FFI Fuzzing (`cargo-fuzz` / Proptest) |
| S8.P2 | S8.P2.SP2 | `docs/implementation/prompts/stage-08/S8_P2_SP2.md` | Penetration Testing, Sandbox Escape Verification & RBAC Bypass Audit |
| S8.P2 | S8.P2.SP3 | `docs/implementation/prompts/stage-08/S8_P2_SP3.md` | Dependency License Compliance, Reproducible Build Attestation & SBOM Signing |
| **S8.P3** | — | `docs/implementation/prompts/stage-08/S8_P3_PROMPT.md` | **Performance Optimization, Documentation Completion & Production Certification** |
| S8.P3 | S8.P3.SP1 | `docs/implementation/prompts/stage-08/S8_P3_SP1.md` | Data-Plane Zero-Copy Optimization & Mobile Battery Impact Tuning |
| S8.P3 | S8.P3.SP2 | `docs/implementation/prompts/stage-08/S8_P3_SP2.md` | Accessibility, Localization & UI/UX Polish Verification Across Form Factors |
| S8.P3 | S8.P3.SP3 | `docs/implementation/prompts/stage-08/S8_P3_SP3.md` | Final Hardware Certification Matrix, Release Readiness Audit & v1.0 Sign-Off |

## 3. Prerequisites & Dependencies
- **Stage 07 Completion Gate**: All Stage 1–7 crates, SDKs, CLI, and UIs complete with >= 95% test coverage and 0 open P0/P1 defects.
- **Security & Profiling Tooling**: `cargo-fuzz` / LibFuzzer, `proptest`, `cargo-deny`, `cargo-audit`, `syft`/`cyclonedx` (SBOM), `cosign` / Ed25519 artifact signing, Valgrind / Heaptrack / ASan / LSan, and Playwright / Flutter Driver.

## 4. Implementation Tasks (Stage-Level)
1. **Cross-OS & Chaos Harness (`tests/e2e-cluster`, `crates/umcf-chaos`)**:
   - Build the unified multi-OS cluster orchestrator running simultaneous workloads (WASM + Distributed AI + GPU Tile Render + Video Transcode + Remote Stream + Erasure Storage + Marketplace Settlement) across heterogeneous OS targets under injected network split-brain, thermal spikes, and memory pressure.
2. **Fuzzing, Red-Team & Supply-Chain Hardening (`fuzz/`, `tests/security-redteam`, `scripts/release/`)**:
   - Implement 15+ LibFuzzer/Proptest targets covering every wire parser, GGUF/WASM/WGSL loader, and C++ FFI boundary.
   - Build automated sandbox-escape and RBAC/mTLS red-team test suite.
   - Implement deterministic reproducible build pipelines, CycloneDX/SPDX SBOM generation, `cargo-deny` license/vulnerability gates, and Ed25519 release signature verification.
3. **Optimization, Accessibility & v1.0 Sign-Off**:
   - Eliminate redundant heap allocations/copies on hot QUIC/activation/storage/streaming data paths; tune mobile background battery drain to `< 1.5%/hour` idle.
   - Complete WCAG 2.1 AA accessibility, RTL/LTR localization (6+ locales), and execute the final v1.0 Release Readiness Audit.

## 5. Acceptance Criteria
- Global workspace line and branch coverage >= 95.0% with 100% pass rate across all unit, integration, simulation, chaos, security, and UI suites.
- 0 memory leaks, 0 file descriptor leaks, and < 1% RSS drift over a continuous multi-workload soak test.
- 0 crashes, panics, or UB across >= 10,000,000 cumulative fuzzing iterations across all 15 fuzz targets.
- 0 sandbox escapes, 0 unauthenticated RPC bypasses, and 0 `cargo-audit` / `cargo-deny` advisories or copyleft license violations.
- All 8 Stages (`stage-01`..`stage-08`) marked `[x]` in `TODO.md` with complete evidence bundles and v1.0.0 release tag artifacts.

## 6. Required Tests
- **Unit (`[UNIT]`)**: Zero-copy buffer pool recycling, battery coalescing timer logic, i18n message key completeness, SBOM/provenance signature verifiers.
- **Integration (`[INT]`)**: Full 6-target cross-platform interop matrix; red-team penetration suite; reproducible build hash comparison.
- **Simulation (`[SIM]`)**: Jepsen-style split-brain network partition & heal; 72-hour accelerated soak test; 10M-iteration fuzz campaigns.
- **Hardware-Required (`[HW]`)**: Real multi-device heterogeneous cluster certification across Android, iOS/macOS, Windows, Linux, and Web; mobile battery & thermal profiling.
- **Manually Verified (`[MANUAL]`)**: Screen-reader (TalkBack/VoiceOver/NVDA) & keyboard-only navigation audit, RTL Arabic/Hebrew & CJK layout inspection, and final v1.0 release installer smoke test.

## 7. Required Evidence & Artifacts
- `artifacts/stage-08/chaos/jepsen_partition_and_soak_report.json`
- `artifacts/stage-08/security/fuzz_campaign_summary.json`
- `artifacts/stage-08/security/redteam_penetration_audit.json`
- `artifacts/stage-08/release/SBOM.cyclonedx.json` & `RELEASE_CERTIFICATION_V1.0.md`

## 8. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Finalize all architecture, operator, security, and developer documentation (`docs/SECURITY_AUDIT.md`, `docs/OPERATIONS_RUNBOOK.md`, `docs/RELEASE_CERTIFICATION_V1.0.md`).
- Mark all `S8.P1.*`, `S8.P2.*`, and `S8.P3.*` items `[x]` in `TODO.md` (reaching 100% completion of `TODO.md`) and cut `[1.0.0]` in `CHANGELOG.md`.

## 9. Rollback Criteria & Completion Gate
- **Rollback Criteria**: Any unresolved crash/panic discovered by fuzzing, any memory/FD leak under soak testing, any RBAC/sandbox bypass, or any regression below 95% test coverage blocks v1.0 sign-off.
- **Completion Gate**: Final v1.0 Production Certification Sign-Off.
