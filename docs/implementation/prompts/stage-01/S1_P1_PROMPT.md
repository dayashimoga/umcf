# Executable Phase Prompt — `S1.P1`: Repository Governance, Monorepo Workspace & Podman 30GB Reproducible Environment

> **Stage:** Stage 1 — Engineering Foundation, Repository and Development Infrastructure (`STAGE_01_PROMPT.md`)  
> **Phase:** `S1.P1` — Repository Governance, Monorepo Workspace & Podman 30GB Reproducible Environment  
> **Subphases:** `S1.P1.SP1`, `S1.P1.SP2`, `S1.P1.SP3`  
> **Target Environment:** Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are the Lead Systems Architect executing Phase **`S1.P1` (Repository Governance, Monorepo Workspace & Podman 30GB Reproducible Environment)** of Stage 1 in the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Phase Objective:** Bootstrap repository governance, branch protection rules, rootless Podman container images with automated 30 GB disk quota enforcement, and the complete multi-language workspace skeleton.
- **Execution Discipline:** Execute `S1.P1.SP1`, `S1.P1.SP2`, and `S1.P1.SP3` sequentially. Do not begin a subsequent subphase until the current subphase's Completion Gate and Evidence JSON are verified.

---

## 2. Prerequisites & Dependencies

- **Upstream Dependency:** `Workspace Bootstrap` must be 100% complete and certified.
- **Target Crates / Services:** `crates/umcf-proto, crates/umcf-core, crates/umcf-telemetry`
- **Quota Enforcement:** `scripts/podman-quota-check.sh` must pass before and after each subphase (`< 29.0 GiB`).

---

## 3. Implementation Tasks & Subphase Execution Sequence

Execute the three constituent subphases in strict sequential order:

### 3.1 Subphase `S1.P1.SP1`: Git/Governance/Rules/Skills Bootstrap
- **Prompt File:** [`S1_P1_SP1.md`](S1_P1_SP1.md)
- **Scope:** Initialize repository governance files, strict commit/branch conventions, workspace-level `TODO.md` and `CHANGELOG.md` ledgers, security policy, and developer contribution rules.
- **Key Deliverables:**
  - Create `.gitignore`, `.gitattributes` (enforcing LF line endings for `*.rs`, `*.ts`, `*.dart`, `*.sh`, `*.toml`), and `.editorconfig`.
  - Create `CONTRIBUTING.md`, `SECURITY.md`, `CODEOWNERS`, `LICENSE-APACHE`, and `LICENSE-MIT` dual-licensing manifests.
  - Initialize structured `TODO.md` tracking all 8 Stages, 24 Phases, and 72 Subphases (`[ ] S1.P1.SP1` .. `[ ] S8.P3.SP3`) with deterministic status parser support.
  - Initialize Keep-a-Changelog compliant `CHANGELOG.md` with `[Unreleased]` and `[0.1.0-stage1]` sections.
  - Create `scripts/verify-governance.sh` and `scripts/check-no-placeholders.sh` implementing regex gates banning `todo!()`, `unimplemented!()`, `FIXME`, and mock stubs in production paths.

### 3.2 Subphase `S1.P1.SP2`: Podman 30GB Container Toolchains & Quota Enforcer
- **Prompt File:** [`S1_P1_SP2.md`](S1_P1_SP2.md)
- **Scope:** Construct rootless Podman container definitions (`Podmanfile.rust`, `Podmanfile.flutter`, `Podmanfile.web`, `compose.podman.yml`) and the hard 30 GB disk quota enforcement daemon/script.
- **Key Deliverables:**
  - Create `containers/Podmanfile.rust` using Wolfi/Alpine minimal base with Rust stable toolchain (`1.81+`), `wasm32-wasip1`, `cargo-nextest`, `cargo-llvm-cov`, and `sccache`.
  - Create `containers/Podmanfile.web` (Node 22 + pnpm + Wrangler) and `containers/Podmanfile.flutter` (Flutter stable SDK + Linux/Android build tools with stripped NDK symbols).
  - Create `compose.podman.yml` mounting workspace root with rootless UID mapping (`userns_mode: keep-id`) and bounded named volumes (`umcf_cargo_cache`, `umcf_sccache`).
  - Implement `scripts/podman-quota-check.sh` querying `podman system df --format json`, host `target/`, and cache volumes; enforce Soft Warning at `25.0 GiB` (`26_843_545_600` bytes) and Hard Abort + auto-prune (`cargo sweep` / `sccache --stop-server` / `podman system prune`) at `29.0 GiB` (`31_138_512_896` bytes).

### 3.3 Subphase `S1.P1.SP3`: Multi-Language Workspace Skeleton & Build Orchestration
- **Prompt File:** [`S1_P1_SP3.md`](S1_P1_SP3.md)
- **Scope:** Scaffold the Cargo workspace root, crate stubs for initial foundation crates, Flutter multi-platform app shell, Svelte 5 web console shell, Cloudflare Workers control-plane shell, and unified `Makefile.toml` / `justfile` orchestrator.
- **Key Deliverables:**
  - Create root `Cargo.toml` (`[workspace]`, `resolver = "2"`, `[workspace.package]`, `[workspace.dependencies]`, and `[workspace.lints.rust]` with `unsafe_code = "deny"` by default, `missing_docs = "warn"`, Clippy pedantic lints).
  - Initialize foundational crates: `crates/umcf-proto`, `crates/umcf-core`, `crates/umcf-telemetry` with compiling `lib.rs` modules.
  - Initialize `apps/web_console` (`package.json`, `svelte.config.js`, `vite.config.ts`, `tsconfig.json`) using Svelte 5 runes (`$state`, `$derived`).
  - Initialize `services/cf_control_plane` (`wrangler.toml`, `src/index.ts`) and `services/cloudrun_fallback` (`Cargo.toml`, `src/main.rs`).
  - Initialize `apps/flutter_client` (`pubspec.yaml`, `analysis_options.yaml`, `lib/main.dart`) and root `justfile` orchestrating `just build`, `just test`, `just lint`, and `just quota-check` inside Podman.

---

## 4. Acceptance Criteria (Phase-Level Measurable Gates)

- [ ] All acceptance criteria in `S1_P1_SP1.md`, `S1_P1_SP2.md`, and `S1_P1_SP3.md` are satisfied.
- [ ] Cross-subphase integration between `S1.P1.SP1`, `S1.P1.SP2`, and `S1.P1.SP3` compiles cleanly with zero warnings (`cargo clippy --workspace --all-targets -- -D warnings`).
- [ ] Line coverage across crates modified in `S1.P1` is `>= 85.0%`.
- [ ] Zero `todo!()`, `unimplemented!()`, or placeholder stubs exist in the codebase (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Podman storage utilization remains strictly `< 29.0 GiB`.

---

## 5. Required Tests (Classified Rollup)

- **Unit Tests:** All unit test suites across `S1.P1.SP1`, `S1.P1.SP2`, and `S1.P1.SP3` pass via `cargo nextest run`.
- **Integration Tests:** End-to-end phase integration tests verifying `Git/Governance/Rules/Skills Bootstrap` + `Podman 30GB Container Toolchains & Quota Enforcer` + `Multi-Language Workspace Skeleton & Build Orchestration` interoperation.
- **Simulation Tests:** Fault-injection, corrupted input, and concurrency stress tests defined in `S1.P1.SP1..SP3`.
- **Hardware-Required Tests:** Executed on target hardware or recorded in hardware execution manifests where containerized hardware passthrough is unavailable.
- **Manually Verified Tests:** Developer visual/CLI inspection checklists completed for `S1.P1.SP1..SP3`.

---

## 6. Required Evidence & Artifacts

- `artifacts/evidence/stage-01/S1_P1_SP1_evidence.json`
- `artifacts/evidence/stage-01/S1_P1_SP2_evidence.json`
- `artifacts/evidence/stage-01/S1_P1_SP3_evidence.json`
- `artifacts/evidence/stage-01/S1_P1_PHASE_SUMMARY.json` aggregating all 3 subphase evidence bundles and phase-level coverage metrics.

---

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates

- Verify `S1.P1.SP1`, `S1.P1.SP2`, and `S1.P1.SP3` are all checked `[x]` in `TODO.md`.
- Consolidate `S1.P1` capabilities into `CHANGELOG.md` and module-level architecture docs.

---

## 8. Rollback Criteria & Completion Gate

- **Rollback Criteria:** If phase integration tests fail or Podman disk quota exceeds `29.0 GiB` after cache pruning, roll back to the start of `S1.P1` (`v0.1.0-certified`) and isolate the failing subphase.
- **Completion Gate:** Tag repository checkpoint `v0.1.1-certified` once `S1_P1_PHASE_SUMMARY.json` reports `"status": "PASS"`.
