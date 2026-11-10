# Executable Subphase Prompt — `S1.P3.SP1`: Rust/Web/Control-Plane CI & Coverage Gates

> **Stage:** Stage 1 — Engineering Foundation, Repository and Development Infrastructure (`STAGE_01_PROMPT.md`)  
> **Phase:** Phase `S1.P3` — Multi-Platform CI/CD Pipelines, SBOM, Checksums & Baseline Verification (`S1_P3_PROMPT.md`)  
> **Subphase:** `S1.P3.SP1` — Rust/Web/Control-Plane CI & Coverage Gates  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S1.P3.SP1` (Rust/Web/Control-Plane CI & Coverage Gates)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Create GitHub Actions / local Podman CI runners for Rust workspace, Svelte 5 web console, and Cloudflare/Cloud Run control plane with >= 85% line coverage enforcement.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S1.P2.SP3` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S1.P2.SP3` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Create `.github/workflows/ci-core.yml` and local equivalent `scripts/ci-core-podman.sh` running `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, and `cargo llvm-cov --workspace --json`.
2. Configure web & edge CI steps in `scripts/ci-web-podman.sh` executing `pnpm --filter web_console lint`, `pnpm --filter web_console test:unit`, and `pnpm --filter cf_control_plane test`.
3. Embed `scripts/podman-quota-check.sh` as both pre-step and post-step in every CI runner script.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S1.P3.SP1` complete:

- [ ] `scripts/ci-core-podman.sh` executes with `0` warnings and verifies `>= 85.0%` line coverage across `umcf-proto`, `umcf-core`, and `umcf-telemetry`.
- [ ] `scripts/ci-web-podman.sh` passes type-checking, linting, and unit tests in `< 60s`.
- [ ] Total Podman disk footprint after full CI run remains `< 29.0 GiB`.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test coverage threshold parser script against synthetic `llvm-cov` JSON reports (`84.9%` FAIL vs `85.0%` PASS).
- **Integration Tests:**
  - Run full `scripts/ci-core-podman.sh` and `scripts/ci-web-podman.sh` end-to-end inside rootless Podman.
- **Simulation / Fault-Injection Tests:**
  - Simulate Clippy warning and coverage drop in temporary branch test to verify CI gate non-zero exit.
- **Hardware-Required Tests:**
  - Not Required.
- **Manually Verified Tests:**
  - Review coverage report summary table in terminal output.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S1.P3.SP1`:

- `artifacts/evidence/stage-01/S1_P3_SP1_evidence.json`
- `artifacts/evidence/stage-01/coverage_stage01.json`
- Standard Evidence Schema (`artifacts/evidence/stage-01/S1_P3_SP1_evidence.json`):
  ```json
  {
    "subphase": "S1.P3.SP1",
    "title": "Rust/Web/Control-Plane CI & Coverage Gates",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S1.P3.SP1`.
2. **`TODO.md` Update:** Flip `- [ ] S1.P3.SP1` to `- [x] S1.P3.SP1` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 1 — Phase 3` summarizing the concrete deliverables and metrics of `S1.P3.SP1`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S1.P3.SP1` changes (`git reset --hard` to the `S1.P2.SP3` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S1.P3.SP1` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-01/S1_P3_SP1_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
