# Phase S8.P2 Execution Prompt: Security Hardening, Fuzzing & Cryptographic Audit

## 1. Role & Context
You are the Principal Application Security, Offensive Red-Team & Cryptography Auditor for UMCF. Because UMCF connects personal mobile phones, laptops, and workstations into a peer-to-peer compute mesh executing untrusted code, shaders, models, and network packets, security is existential. Phase `S8.P2` subjects every protocol parser, file format reader, and FFI boundary to continuous coverage-guided fuzzing (`S8.P2.SP1`), executes automated red-team sandbox-escape and RBAC/mTLS bypass penetration tests (`S8.P2.SP2`), and locks down the software supply chain with license compliance gates, reproducible builds, and signed CycloneDX/SPDX SBOMs (`S8.P2.SP3`).

This phase consists of three sequential subphases:
1. `S8_P2_SP1.md`: Protocol Parser & FFI Fuzzing (`cargo-fuzz` / Proptest)
2. `S8_P2_SP2.md`: Penetration Testing, Sandbox Escape Verification & RBAC Bypass Audit
3. `S8_P2_SP3.md`: Dependency License Compliance, Reproducible Build Attestation & SBOM Signing

## 2. Prerequisites & Dependencies
- Completion of Phase `S8.P1` (Cross-OS & Chaos Soak verified).
- `cargo-fuzz` / LibFuzzer, `proptest`, `cargo-deny`, `cargo-audit`, and SBOM tooling.

## 3. Implementation Tasks
1. **Subphase S8.P2.SP1 (`fuzz/fuzz_targets/*.rs`)**:
   - Build and run 15+ coverage-guided fuzzing harnesses covering GGUF v2/v3 headers, WGSL/SPIR-V shaders, WASM modules, QUIC P2P wire frames, Distributed AI activation packets, Remote Streaming input/media packets, Workload Manifests (`umcf.job.yaml`), Storage FastCDC/DAG manifests, and `SignedResourceReceipt` parsers.
2. **Subphase S8.P2.SP2 (`tests/security-redteam/src/*.rs`)**:
   - Build automated adversarial red-team test suites attempting WASM sandbox escapes (filesystem traversal, ambient network socket creation, fuel exhaustion, memory bomb), GPU shader TDR/OOB reads, GGUF arbitrary pointer offset attacks, mTLS certificate spoofing, capability token privilege escalation, Private AI Enclave data exfiltration, and Remote Stream `ViewOnly` input injection bypasses.
3. **Subphase S8.P2.SP3 (`deny.toml`, `scripts/release/build_reproducible.rs`)**:
   - Enforce zero known CVEs (`cargo-audit`) and strict permissive-only dependency licensing (`cargo-deny`), generate signed CycloneDX 1.5 & SPDX 2.3 Software Bills of Materials (SBOMs), and verify deterministic reproducible binary builds with Ed25519 SLSA provenance attestations.

## 4. Acceptance Criteria
- >= 95% line/branch coverage across all security validation modules and 100% pass rate across all fuzzing, red-team, and supply-chain suites.
- **Zero Fuzz Panics/Crashes/OOMs**: >= 10,000,000 cumulative fuzz inputs across the 15 fuzz targets complete with 0 panics, 0 segfaults, 0 sanitizer violations, and 0 unbounded allocation OOMs (any input > 64 KiB rejected or bounded before allocation).
- **100% Red-Team Attack Containment**: Every simulated sandbox escape, path traversal, replay attack, RBAC escalation, and cryptographic forgery attempt is blocked and logged with a structured `SecurityAuditEvent`.
- **Clean Supply Chain**: `cargo deny check advisories bans licenses sources` passes with 0 warnings/errors; signed SBOM and SLSA provenance verify cleanly.

## 5. Required Tests
- **`[UNIT]`**: Fuzz corpus regression tests (running saved seed/crash-free corpus in standard `cargo test`), length-prefix allocation bomb guards, constant-time token comparison (`subtle::ConstantTimeEq`).
- **`[INT]`**: 25-scenario Red-Team Penetration Suite against a live daemon; SBOM & reproducible build attestation verifier.
- **`[SIM]`**: Byzantine Sybil swarm attack + Eclipse DHT routing table poisoning simulation.
- **`[HW]`**: Hardware side-channel / VRAM zero-scrubbing verification across consecutive multi-tenant GPU tasks.
- **`[MANUAL]`**: Inspect the Security Audit Log & SBOM Attestation viewer in Svelte 5 `/settings` and Flutter Security Center.

## 6. Required Evidence & Artifacts
- `artifacts/stage-08/p2/fuzz_15_targets_coverage_and_runs.json`
- `artifacts/stage-08/p2/redteam_25_penetration_vectors_report.json`
- `artifacts/stage-08/p2/cargo_deny_and_audit_clean_report.json`
- `artifacts/stage-08/p2/umcf-v1.0.0.sbom.cyclonedx.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Create `docs/SECURITY_AUDIT.md` detailing threat model, fuzz targets, sandbox boundaries, cryptographic primitives, and supply-chain provenance.
- Mark `S8.P2.SP1`–`S8.P2.SP3` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: Any single fuzz crash/panic on untrusted input, any successful sandbox escape or privilege escalation in the red-team suite, or any unpatched security advisory in dependencies.
- **Gate**: All 3 subphases (`S8_P2_SP1`..`SP3`) pass before starting Phase `S8.P3`.
