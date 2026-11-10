# Executable Subphase Prompt — `S1.P1.SP3`: Multi-Language Workspace Skeleton & Build Orchestration

> **Stage:** Stage 1 — Engineering Foundation, Repository and Development Infrastructure (`STAGE_01_PROMPT.md`)  
> **Phase:** Phase `S1.P1` — Repository Governance, Monorepo Workspace & Podman 30GB Reproducible Environment (`S1_P1_PROMPT.md`)  
> **Subphase:** `S1.P1.SP3` — Multi-Language Workspace Skeleton & Build Orchestration  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S1.P1.SP3` (Multi-Language Workspace Skeleton & Build Orchestration)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Scaffold the Cargo workspace root, crate stubs for initial foundation crates, Flutter multi-platform app shell, Svelte 5 web console shell, Cloudflare Workers control-plane shell, and unified `Makefile.toml` / `justfile` orchestrator.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S1.P1.SP2` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S1.P1.SP2` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Create root `Cargo.toml` (`[workspace]`, `resolver = "2"`, `[workspace.package]`, `[workspace.dependencies]`, and `[workspace.lints.rust]` with `unsafe_code = "deny"` by default, `missing_docs = "warn"`, Clippy pedantic lints).
2. Initialize foundational crates: `crates/umcf-proto`, `crates/umcf-core`, `crates/umcf-telemetry` with compiling `lib.rs` modules.
3. Initialize `apps/web_console` (`package.json`, `svelte.config.js`, `vite.config.ts`, `tsconfig.json`) using Svelte 5 runes (`$state`, `$derived`).
4. Initialize `services/cf_control_plane` (`wrangler.toml`, `src/index.ts`) and `services/cloudrun_fallback` (`Cargo.toml`, `src/main.rs`).
5. Initialize `apps/flutter_client` (`pubspec.yaml`, `analysis_options.yaml`, `lib/main.dart`) and root `justfile` orchestrating `just build`, `just test`, `just lint`, and `just quota-check` inside Podman.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S1.P1.SP3` complete:

- [ ] `podman run --rm umcf-rust cargo check --workspace --all-targets` completes with `0` warnings and `0` errors.
- [ ] `apps/web_console` builds cleanly via `pnpm check && pnpm build` inside `umcf-web` container.
- [ ] `services/cf_control_plane` passes `tsc --noEmit` and dry-run Wrangler bundle validation.
- [ ] Post-build `scripts/podman-quota-check.sh` confirms total disk footprint remains `< 29.0 GiB`.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Verify workspace crate version inheritance and lint configuration via smoke unit tests in `umcf-proto`, `umcf-core`, and `umcf-telemetry`.
- **Integration Tests:**
  - Execute `just build-all` across Rust workspace, Svelte 5 web console, Cloudflare Worker, and Cloud Run service inside Podman.
- **Simulation / Fault-Injection Tests:**
  - Test clean rebuild from cold `target/` vs warm `sccache` cache and measure cache hit rate (`>= 80%` on second run).
- **Hardware-Required Tests:**
  - Not Required (containerized multi-language build verification).
- **Manually Verified Tests:**
  - Verify `just --list` outputs documented developer recipes.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S1.P1.SP3`:

- `artifacts/evidence/stage-01/S1_P1_SP3_evidence.json` recording build timings, exit codes, and `sccache -s` hit ratios.
- `artifacts/evidence/stage-01/workspace_metadata.json` from `cargo metadata --format-version 1`.
- Standard Evidence Schema (`artifacts/evidence/stage-01/S1_P1_SP3_evidence.json`):
  ```json
  {
    "subphase": "S1.P1.SP3",
    "title": "Multi-Language Workspace Skeleton & Build Orchestration",
    "git_commit": "<HEAD_SHA>",
    "timestamp_utc": "<ISO-8601>",
    "podman_total_bytes": 0,
    "podman_limit_bytes": 31138512896,
    "placeholder_count": 0,
    "tests": {
      "unit": "PASS",
      "integration": "PASS",
      "simulation": "PASS",
      "hardware_required": "PASS_OR_DEFERRED_WITH_MANIFEST",
      "manually_verified": "PASS"
    },
    "artifact_hashes": {}
  }
  ```

---

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S1.P1.SP3`.
2. **`TODO.md` Update:** Flip `- [ ] S1.P1.SP3` to `- [x] S1.P1.SP3` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 1 — Phase 1` summarizing the concrete deliverables and metrics of `S1.P1.SP3`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S1.P1.SP3` changes (`git reset --hard` to the `S1.P1.SP2` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S1.P1.SP3` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-01/S1_P1_SP3_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
