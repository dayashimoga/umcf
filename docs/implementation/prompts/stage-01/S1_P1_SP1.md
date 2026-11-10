# Executable Subphase Prompt — `S1.P1.SP1`: Git/Governance/Rules/Skills Bootstrap

> **Stage:** Stage 1 — Engineering Foundation, Repository and Development Infrastructure (`STAGE_01_PROMPT.md`)  
> **Phase:** Phase `S1.P1` — Repository Governance, Monorepo Workspace & Podman 30GB Reproducible Environment (`S1_P1_PROMPT.md`)  
> **Subphase:** `S1.P1.SP1` — Git/Governance/Rules/Skills Bootstrap  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S1.P1.SP1` (Git/Governance/Rules/Skills Bootstrap)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Initialize repository governance files, strict commit/branch conventions, workspace-level `TODO.md` and `CHANGELOG.md` ledgers, security policy, and developer contribution rules.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `Repository Initialization` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- Empty or newly initialized Git repository at workspace root
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Create `.gitignore`, `.gitattributes` (enforcing LF line endings for `*.rs`, `*.ts`, `*.dart`, `*.sh`, `*.toml`), and `.editorconfig`.
2. Create `CONTRIBUTING.md`, `SECURITY.md`, `CODEOWNERS`, `LICENSE-APACHE`, and `LICENSE-MIT` dual-licensing manifests.
3. Initialize structured `TODO.md` tracking all 8 Stages, 24 Phases, and 72 Subphases (`[ ] S1.P1.SP1` .. `[ ] S8.P3.SP3`) with deterministic status parser support.
4. Initialize Keep-a-Changelog compliant `CHANGELOG.md` with `[Unreleased]` and `[0.1.0-stage1]` sections.
5. Create `scripts/verify-governance.sh` and `scripts/check-no-placeholders.sh` implementing regex gates banning `todo!()`, `unimplemented!()`, `FIXME`, and mock stubs in production paths.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S1.P1.SP1` complete:

- [ ] `scripts/verify-governance.sh` exits with code `0` and verifies presence and non-empty schema of all governance files.
- [ ] `scripts/check-no-placeholders.sh` scans repository in `< 2.0s` and rejects synthetic `todo!()` injections with exit code `1`.
- [ ] `TODO.md` contains exact checkbox entries for all 72 subphases (`S1.P1.SP1` through `S8.P3.SP3`).
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test `scripts/verify-governance.sh` parser against valid and missing governance file fixtures.
- **Integration Tests:**
  - Run Git hook pre-commit simulation verifying `.gitattributes` LF normalization and `check-no-placeholders.sh` enforcement.
- **Simulation / Fault-Injection Tests:**
  - Simulate forbidden `todo!("pending")` commit in a temporary scratch file and confirm pre-commit rejection.
- **Hardware-Required Tests:**
  - Not Required (pure repository & shell governance logic; runs on any standard CPU).
- **Manually Verified Tests:**
  - Inspect rendered Markdown formatting of `CONTRIBUTING.md`, `TODO.md`, and `CHANGELOG.md` in IDE preview.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S1.P1.SP1`:

- `artifacts/evidence/stage-01/S1_P1_SP1_evidence.json` containing governance file SHA-256 digests.
- `artifacts/evidence/stage-01/placeholder_scan_S1_P1_SP1.log` showing `0` forbidden tokens.
- Standard Evidence Schema (`artifacts/evidence/stage-01/S1_P1_SP1_evidence.json`):
  ```json
  {
    "subphase": "S1.P1.SP1",
    "title": "Git/Governance/Rules/Skills Bootstrap",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S1.P1.SP1`.
2. **`TODO.md` Update:** Flip `- [ ] S1.P1.SP1` to `- [x] S1.P1.SP1` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 1 — Phase 1` summarizing the concrete deliverables and metrics of `S1.P1.SP1`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S1.P1.SP1` changes (`git reset --hard` to the `Repository Initialization` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S1.P1.SP1` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-01/S1_P1_SP1_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
