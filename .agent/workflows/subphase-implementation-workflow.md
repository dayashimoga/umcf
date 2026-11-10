---
description: Mandatory 10-step subphase execution workflow for implementing, testing, reviewing, and documenting UMCF subphases.
---

# UMCF Mandatory 10-Step Subphase Implementation Workflow

Every subphase in the UMCF development lifecycle must execute all 10 steps below in strict sequential order. Never skip steps or mark a subphase complete without passing every gate.

---

## Step 1: Read Contract & Activate Relevant Skills
1. Read the current Phase and Subphase specification, acceptance criteria, and architectural contracts in `docs/` and `TODO.md`.
2. Activate the relevant `.agent/skills/umcf-*/SKILL.md` skills and review `.agent/rules/00-global-governance.md` through `06-ci-cd-and-podman.md`.

## Step 2: Inspect Existing Codebase & Interfaces
1. Inspect all affected crates, FFI definitions (`umcf-ffi`), wire protocol messages (`umcf-proto`), Flutter screens/providers, and Svelte 5 stores.
2. Verify existing tests and Podman storage usage before making modifications.

## Step 3: Implement Production-Grade Domain Logic
1. Write complete, real, non-placeholder implementation code adhering to:
   - Rust `#![forbid(unsafe_code)]` (except audited `*-sys` FFI wrappers with `// SAFETY:` proofs), `thiserror` error types, bounded async channels, and zero `unwrap()`/`expect()`.
   - Explicit hardware capability checks and memory watermark enforcement via `umcf-resource-governor`.
   - Strict separation between control-plane signaling and P2P QUIC data-plane streams.

## Step 4: Integrate Across Boundaries (FFI, Control Plane, UI)
1. Wire new backend capabilities end-to-end through `umcf-node`, `umcf-cli`, `umcf-ffi` (`flutter_rust_bridge`), Flutter Riverpod controllers/widgets, and Svelte 5 runes/components as applicable.
2. Ensure every newly added or modified UI control is connected to a live state transition and real backend action with loading, success, and error states.

## Step 5: Write & Execute Comprehensive Tests (>= 95% Coverage, 100% Pass Rate)
1. Author unit (`[Unit]`), integration (`[Integration]`), simulation (`[Simulation]`), and hardware (`[Hardware-Required]`) tests covering happy paths, edge cases, malformed inputs, timeouts, and security rejections.
2. Run the full test suites (`cargo test --workspace`, `flutter test`, `vitest run`) and verify a **100% pass rate** and **>= 95% line/branch coverage**.

## Step 6: Execute Code Review & Static Analysis Gates
1. Run `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
2. Run `flutter analyze` and `npm run check` (`svelte-check` + `eslint`) where applicable.
3. Run the checklist in `.agent/workflows/code-review-workflow.md` to verify concurrency safety, security capability checks, and UI completeness.

## Step 7: Fix All Findings & Re-Verify
1. Remediate every compiler warning, clippy lint, test failure, coverage gap, or code review issue discovered in Steps 5–6.
2. Re-run all affected test suites and linters to confirm zero regressions.

## Step 8: Update Documentation, `TODO.md`, and `CHANGELOG.md` (Append-Only)
1. Update relevant architecture, protocol, or operator documentation in `docs/`.
2. Mark completed subphase tasks `[x]` in `TODO.md` (preserving all existing lines; append any newly identified follow-up items).
3. Append a detailed subphase entry to `CHANGELOG.md` listing Added, Changed, Fixed, Security, and Verification metrics.

## Step 9: Collect Verification Evidence
1. Record exact test execution counts, pass rates, coverage percentages, benchmark metrics (with test classification tiers), and Podman storage usage (`< 30 GB`) in the subphase verification summary.

## Step 10: Stage & Commit Cleanly
1. Verify `git status` and `git diff` to ensure no secrets, temporary scratch files, or untracked build bloat are staged.
2. Create an atomic, descriptive commit referencing the completed subphase ID.
