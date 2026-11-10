# Phase S8.P3 Execution Prompt: Performance Optimization, Documentation Completion & Production Certification

## 1. Role & Context
You are the Principal Performance Architect, UX Polish Lead & v1.0 Release Certification Officer for UMCF. Phase `S8.P3` is the final phase of the entire 8-stage UMCF implementation roadmap. It tunes the hot data plane for zero-copy throughput and minimal mobile battery impact (`S8.P3.SP1`), verifies WCAG 2.1 AA accessibility, 6-language localization (including RTL), and responsive UI polish across all form factors (`S8.P3.SP2`), and executes the final Hardware Certification Matrix, Release Readiness Audit, and v1.0.0 Production Sign-Off (`S8.P3.SP3`).

This phase consists of three sequential subphases:
1. `S8_P3_SP1.md`: Data-Plane Zero-Copy Optimization & Mobile Battery Impact Tuning
2. `S8_P3_SP2.md`: Accessibility, Localization & UI/UX Polish Verification Across Form Factors
3. `S8_P3_SP3.md`: Final Hardware Certification Matrix, Release Readiness Audit & v1.0 Sign-Off

## 2. Prerequisites & Dependencies
- Completion of Phases `S8.P1` (Cross-OS Integration, Chaos & Soak) and `S8.P2` (Fuzzing, Red-Team Security & Signed SBOM).

## 3. Implementation Tasks
1. **Subphase S8.P3.SP1 (`crates/umcf-net/src/zero_copy.rs`, `crates/umcf-hw/src/power_governor.rs`)**:
   - Eliminate intermediate heap copies across QUIC stream/datagram framing (`bytes::Bytes` slice views, page-aligned scatter-gather `IoSlice`, shared memory / mmap ring buffers for local loopback IPC, and direct GPU staging buffer mapping).
   - Implement `MobilePowerGovernor` coalescing background keepalive/gossip timers to respect Android Doze / iOS low-power radio wake windows, keeping idle background battery drain `< 1.0% per hour` and enforcing user-configured thermal/battery ceilings.
2. **Subphase S8.P3.SP2 (`ui/flutter_app/lib/l10n/`, `ui/admin_web/src/lib/i18n/`)**:
   - Complete WCAG 2.1 Level AA accessibility compliance (semantic screen-reader labels, >= 4.5:1 contrast ratios in Light/Dark/High-Contrast themes, full keyboard navigation & focus rings, `prefers-reduced-motion` support) and 100% string localization across 6 locales (`en-US`, `es-ES`, `de-DE`, `ja-JP`, `zh-CN`, and RTL `ar-SA`) across mobile phones, foldables, tablets, desktops, and ultrawide monitors.
3. **Subphase S8.P3.SP3 (`docs/RELEASE_CERTIFICATION_V1.0.md`, `scripts/release/certify_v1.rs`)**:
   - Execute the automated v1.0 Certification Gate Runner verifying every stage (`stage-01` through `stage-08`), every crate's >= 95% coverage & 100% test pass rate, zero open `TODO.md` items, complete architecture/API/CLI/operator documentation, and signed v1.0.0 release packages.

## 4. Acceptance Criteria
- >= 95.0% workspace-wide test coverage and 100% test pass rate.
- Data-plane throughput reaches >= 85% of raw line rate on QUIC shard/activation transfers with zero redundant payload `memcpy` allocations on the hot path.
- Mobile idle background radio/CPU wakeups average `<= 1 wakeup per 60 seconds` in Low-Power Standby mode.
- Automated `axe-core` (Svelte 5) and Flutter `Semantics` accessibility audits report **0 violations** across all screens in Light, Dark, and High-Contrast themes.
- 100% of items in `TODO.md` across all 8 stages are checked `[x]` and backed by verified artifacts.

## 5. Required Tests
- **`[UNIT]`**: `Bytes` zero-copy reference count verification (asserting pointer equality across framing layers), radio wakeup timer coalescing math, i18n locale key parity checker, certification gate auditor.
- **`[INT]`**: Criterion throughput benchmark regression gate; automated `axe-core` WCAG 2.1 AA Playwright audit on all Svelte 5 routes; Flutter Semantics + RTL layout golden/widget tests.
- **`[SIM]`**: 24-hour simulated mobile battery discharge curve under Idle Standby vs Active Charging Contribution.
- **`[HW]`**: Hardware benchmark certification matrix (CPU/GPU/NPU/Network/Storage throughput & latency SLAs on physical targets).
- **`[MANUAL]`**: Full screen-reader walkthrough, RTL (`ar-SA`) layout inspection, and final v1.0.0 installation & smoke test.

## 6. Required Evidence & Artifacts
- `artifacts/stage-08/p3/zero_copy_criterion_benchmarks.json`
- `artifacts/stage-08/p3/mobile_battery_impact_profile.json`
- `artifacts/stage-08/p3/wcag_aa_axe_and_flutter_a11y_report.json`
- `artifacts/stage-08/p3/FINAL_V1_0_CERTIFICATION_RECEIPT.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Finalize `docs/RELEASE_CERTIFICATION_V1.0.md`, `README.md`, `TODO.md` (100% `[x]`), and `CHANGELOG.md` (`[1.0.0] - Production Release`).

## 8. Rollback Criteria & Completion Gate
- **Rollback**: Any performance regression > 5% against baseline SLAs, any WCAG AA accessibility violation, any missing translation key, or any unchecked item in `TODO.md`.
- **Gate**: **UMCF v1.0.0 Production Release Certified.**
