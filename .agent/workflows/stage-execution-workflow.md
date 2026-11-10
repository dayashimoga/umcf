---
description: Reusable workflow to execute the next approved UMCF stage using its complete stage, phase, and subphase specifications.
---

# UMCF Stage Execution Workflow

Execute the next approved UMCF stage using its complete stage, phase, and subphase specifications.

---

## Execution Directives

1. **Read Governance, Contracts & Prior Evidence**:
   - Read [`ENGINEERING_CONTRACT.md`](../../ENGINEERING_CONTRACT.md), [`AGENTS.md`](../../AGENTS.md), applicable rules in `.agent/rules/`, domain skills in `.agent/skills/`, architectural decisions in `docs/architecture/adr/`, and previous certification reports in `docs/certification/` (e.g., [`BOOTSTRAP_CERTIFICATION_REPORT.md`](../../docs/certification/BOOTSTRAP_CERTIFICATION_REPORT.md) or prior stage certification reports).

2. **Inspect Existing Implementation**:
   - Inspect the existing implementation across crates, services, and client applications.
   - Never assume a feature is complete because documentation claims it is; verify actual source files, FFI bindings, types, and wire protocols.

3. **Analyze Gaps, Dependencies & Risks**:
   - Identify gaps, component dependencies, performance risks, and affected system interfaces across Rust crates (`crates/*`), Flutter (`apps/mobile-desktop`), Svelte 5 (`apps/web-dashboard`), and control plane services (`services/*`).

4. **Sequential Subphase Implementation**:
   - Implement one subphase at a time using its executable prompt from `docs/implementation/prompts/stage-0X/`.
   - Complete real integration, non-placeholder domain logic, and user-visible functionality end-to-end following [`.agent/workflows/subphase-implementation-workflow.md`](./subphase-implementation-workflow.md).

5. **Continuous Verification & Validation**:
   - Run tests, security checks, performance benchmarks, and regression validation after each meaningful change.
   - Enforce a 100% test pass rate and `>= 95%` meaningful coverage across modified components.

6. **Independent Review & Defect Remediation**:
   - Independently review the implementation against [`.agent/workflows/code-review-workflow.md`](./code-review-workflow.md) and security standards in `.agent/rules/04-security-and-privacy.md`.
   - Fix all identified compiler warnings, Clippy lints, test failures, or contract violations before proceeding.

7. **Real Runtime & Hardware Verification**:
   - Verify actual application behavior and real hardware execution wherever available.
   - Classify all tests explicitly (`[Unit]`, `[Integration]`, `[Simulation]`, `[Hardware-Required]`, `[Manually-Verified]`).
   - If physical accelerators (GPU/NPU) are absent, report limitations accurately and mark them `Hardware-Unverified`; never conflate simulation with real hardware certification.

8. **Append-Only Tracking & Documentation**:
   - Update architecture docs, component contracts, and user/developer guides as interfaces evolve.
   - Update [`TODO.md`](../../TODO.md) (mark completed items `[x]` with timestamps; never delete history).
   - Append a structured entry to [`CHANGELOG.md`](../../CHANGELOG.md) documenting Added, Changed, Fixed, Security, and Verification Evidence.

9. **Stage Certification Report**:
   - Produce a comprehensive stage report in `docs/certification/` with requirement traceability (`FR-xxx`, `NFR-xxx`), test execution matrices, coverage evidence, packaged artifacts, SHA-256 checksums, and documented known limitations.

10. **Strict Stage Completion Gate**:
    - Do not advance to the next stage until all mandatory acceptance criteria pass, all quality gates are satisfied, and explicit user approval is granted.

---

## Core Invariants

- **Preserve Previous Functionality**: Never regress existing, certified crates, endpoints, or UI features.
- **Zero Placeholders**: Never use placeholders, `todo!()`, `unimplemented!()`, fake success responses, hardcoded production data, or unsupported completion claims.
- **Resource & Quota Safety**: Respect the 30 GB Podman storage quota cap (`scripts/enforce-podman-quota.*`) and safe device consent defaults (`sharing_enabled: false` by default).
- **If Blocked**: Identify the exact blocker, document alternative paths or degraded modes, and continue independent tasks without compromising correctness or safety.
