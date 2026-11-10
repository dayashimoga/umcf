# Subphase S8.P3.SP3 Execution Prompt: Final Hardware Certification Matrix, Release Readiness Audit & v1.0 Sign-Off

## 1. Role & Context
You are the Chief Release Architect & v1.0 Production Certification Officer for the **Unified Mobile-First Compute Fabric (UMCF)**. Subphase `S8.P3.SP3` is the final completion gate of the entire 8-stage engineering program. You will execute the automated Release Readiness Auditor (`scripts/release/certify_v1.rs`), validate the Hardware Certification Matrix across all target tiers, verify that 100% of documentation, `TODO.md`, and `CHANGELOG.md` entries are complete and accurate, and generate the signed `RELEASE_CERTIFICATION_V1.0.md` and `FINAL_V1_0_CERTIFICATION_RECEIPT.json`.

## 2. Prerequisites & Dependencies
- All 7 prior stages (`stage-01`..`stage-07`) and subphases `S8.P1.SP1` through `S8.P3.SP2` 100% complete and verified.

## 3. Implementation Tasks
1. **Automated v1.0 Release Readiness Auditor (`scripts/release/certify_v1.rs`)**:
   - Implement an executable verification command (`cargo run --bin umcf-certify-v1`) that programmatically checks and attests all 10 production release gates:
     1. **Gate 1 (Zero Placeholder / Mock Code Audit)**: Scans the entire production source tree (`crates/`, `ui/`, `sdks/`) verifying zero occurrences of `todo!()`, `unimplemented!()`, `FIXME`, stubbed dummy return values, or mock arrays outside `#[cfg(test)]`.
     2. **Gate 2 (100% Test Pass Rate across All Suites)**: Verifies passing status of all `[UNIT]`, `[INT]`, `[SIM]`, `[HW]`, Chaos, Fuzz Corpus, Red-Team, Polyglot SDK, Flutter, and Svelte 5 Playwright test suites.
     3. **Gate 3 (>= 95.0% Line & Branch Coverage)**: Parses workspace `lcov.info` / `llvm-cov` JSON summary and asserts every production crate and SDK meets `>= 95.0%` coverage.
     4. **Gate 4 (Zero Compiler / Linter / Security Warnings)**: Runs `cargo clippy --workspace --all-targets -- -D warnings`, `cargo deny check`, `flutter analyze`, and `pnpm check` with zero warnings.
     5. **Gate 5 (Performance & Latency SLA Verification)**: Verifies benchmark artifacts against v1.0 SLAs (P2P pairing < 500 ms, WASM cold start < 15 ms, LLM cancellation <= 50 ms, Remote Stream LAN glass-to-glass <= 25 ms, Input latency <= 3 ms, Mobile Eco wakeups <= 1/min, 0 leak slope over 10k cycles).
     6. **Gate 6 (Reproducible Build & Signed SBOM Attestation)**: Verifies SHA-256/BLAKE3 bit-reproducibility, CycloneDX/SPDX SBOM completeness, and Ed25519 provenance signatures (`S8.P2.SP3`).
     7. **Gate 7 (Accessibility & Localization Gate)**: Verifies 0 `axe-core` / Flutter semantics violations and 100% 6-locale parity (`S8.P3.SP2`).
     8. **Gate 8 (Stage 1–8 Evidence Bundle Completeness)**: Verifies every required artifact file across `artifacts/stage-01/` through `artifacts/stage-08/` exists, is non-empty, and has its BLAKE3 hash recorded in the master manifest.
     9. **Gate 9 (`TODO.md` 100% Completion Check)**: Parses `TODO.md` and asserts that `0` unchecked `[ ]` items remain across Stages 1 through 8 (`checked_count > 0 && unchecked_count == 0`).
     10. **Gate 10 (Documentation & `CHANGELOG.md` v1.0.0 Completeness)**: Verifies all architecture docs, API/SDK guides, CLI reference, operations runbook, security audit, and `CHANGELOG.md` `[1.0.0]` release notes are present and broken-link-free.
2. **Hardware Certification Matrix & Installer Packaging**:
   - Package signed release artifacts for:
     - Desktop/Server Daemons & CLI: Windows (`x86_64` / `ARM64` `.msi`/`.zip`), Linux (`x86_64` / `aarch64` `.deb`/`.rpm`/`.tar.zst` + systemd user unit), macOS (`Universal2` `.dmg`/`.pkg` + launchd plist).
     - Mobile & Client Apps: Android (`.apk` / `.aab` with 16 KiB page alignment), iOS (`.ipa` framework bundle), Web Admin & PWA bundle.
     - Developer SDKs: Crates (`umcf-sdk`), NPM tarball (`@umcf/sdk-1.0.0.tgz`), Python wheel (`umcf-1.0.0-py3-none-any.whl`).
   - Record the empirical **Hardware Certification Matrix** in `docs/RELEASE_CERTIFICATION_V1.0.md` covering CPU (x86_64 AVX2/AVX-512, ARM64 NEON/dotprod), GPU (Vulkan, Metal, DX12, CUDA/ROCm, WebGPU), Media (NVENC, VideoToolbox, MediaCodec, VAAPI, Software), and Network (LAN, Wi-Fi 6, QUIC Hole-Punched WAN).

## 4. Acceptance Criteria
- `cargo run --bin umcf-certify-v1` exits with code `0` and outputs `"UMCF v1.0.0 PRODUCTION CERTIFICATION PASSED (10/10 GATES VERIFIED)"`.
- Global workspace coverage `>= 95.0%` and test pass rate `== 100.0%`.
- `TODO.md` has `0` remaining `[ ]` items (`100%` of Stage 1–8 subphases marked `[x]`).
- All release packages, SBOMs, and `FINAL_V1_0_CERTIFICATION_RECEIPT.json` are cryptographically signed and verified.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_certify_v1_gate_scanner_detects_synthetic_todo_macro_or_unchecked_box`
  - `test_documentation_internal_markdown_link_validator_zero_broken_links`
- **`[INT]`**:
  - `test_full_10_gate_umcf_certify_v1_execution_passes`
  - `test_clean_install_and_first_run_smoke_test_of_packaged_v1_artifacts`
- **`[SIM]`**:
  - Full-cluster cold-boot from packaged v1.0.0 binaries -> QR pair -> run all 6 built-in workflow templates -> settle marketplace credits -> verify provenance.
- **`[HW]`**:
  - Execute final hardware certification benchmark pass and populate the Hardware Certification Matrix table in `docs/RELEASE_CERTIFICATION_V1.0.md`.
- **`[MANUAL]`**:
  - Final executive sign-off walkthrough of the packaged v1.0.0 CLI, Flutter App, and Svelte 5 Admin Console.

## 6. Required Evidence & Artifacts
- `artifacts/stage-08/p3/FINAL_V1_0_CERTIFICATION_RECEIPT.json`
- `docs/RELEASE_CERTIFICATION_V1.0.md`
- `artifacts/stage-08/release/SHASUMS256.txt` & `SHASUMS256.txt.sig`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Publish `docs/RELEASE_CERTIFICATION_V1.0.md` with the complete Hardware Certification Matrix, SLA attainment table, coverage summary, and SBOM digest.
- Mark `S8.P3.SP3`, Phase `S8.P3`, and **Stage 08** `[x]` in `TODO.md` (achieving **100% completion** of `TODO.md`).
- Finalize `CHANGELOG.md` under `## [1.0.0] - Production Release` summarizing all capabilities delivered across Stages 1 through 8.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If any of the 10 automated certification gates in `umcf-certify-v1` fails, v1.0.0 sign-off is blocked until remediated and re-verified.
- **Completion Gate**: **10/10 Certification Gates Passed. UMCF v1.0.0 is officially certified for production.**
