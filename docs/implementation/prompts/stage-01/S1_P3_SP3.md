# Executable Subphase Prompt — `S1.P3.SP3`: Release Manifest, SBOM Generation & Stage 1 Certification

> **Stage:** Stage 1 — Engineering Foundation, Repository and Development Infrastructure (`STAGE_01_PROMPT.md`)  
> **Phase:** Phase `S1.P3` — Multi-Platform CI/CD Pipelines, SBOM, Checksums & Baseline Verification (`S1_P3_PROMPT.md`)  
> **Subphase:** `S1.P3.SP3` — Release Manifest, SBOM Generation & Stage 1 Certification  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S1.P3.SP3` (Release Manifest, SBOM Generation & Stage 1 Certification)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement deterministic CycloneDX SBOM generation, SHA-256/BLAKE3 artifact manifest signing, `cargo-deny` license/advisory auditing, and the automated Stage 1 gate verifier.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S1.P3.SP2` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S1.P3.SP2` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Create `deny.toml` configuring `cargo-deny` for license allow-listing (`MIT`, `Apache-2.0`, `BSD-3-Clause`, `ISC`, `Unicode-3.0`, `Zlib`), banned crates, and RustSec advisory checks.
2. Implement `scripts/generate-sbom-and-manifest.sh` generating `artifacts/sbom/stage-01-cyclonedx.json` and `artifacts/manifests/stage-01-sha256sums.txt`.
3. Implement `scripts/verify-stage-01.sh` validating all 9 Stage 1 subphase evidence files, running `podman-quota-check.sh`, `check-no-placeholders.sh`, `cargo deny check`, and emitting `artifacts/evidence/stage-01/STAGE_01_CERTIFICATION.json`.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S1.P3.SP3` complete:

- [ ] `cargo deny check` passes with `0` license violations and `0` unpatched security advisories.
- [ ] `artifacts/sbom/stage-01-cyclonedx.json` validates against CycloneDX v1.5 schema.
- [ ] `scripts/verify-stage-01.sh` exits `0` and certifies all 9 subphases (`S1.P1.SP1` .. `S1.P3.SP3`).
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test manifest generator checksum verification against known test files and tampered files.
- **Integration Tests:**
  - Execute full `scripts/verify-stage-01.sh` end-to-end inside Podman.
- **Simulation / Fault-Injection Tests:**
  - Inject a GPL-3.0 dummy dependency in a temporary Cargo manifest test and confirm `cargo deny check licenses` aborts.
- **Hardware-Required Tests:**
  - Not Required.
- **Manually Verified Tests:**
  - Review `STAGE_01_CERTIFICATION.json` and verify `TODO.md` marks all Stage 1 items `[x]`.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S1.P3.SP3`:

- `artifacts/evidence/stage-01/S1_P3_SP3_evidence.json`
- `artifacts/evidence/stage-01/STAGE_01_CERTIFICATION.json`
- `artifacts/sbom/stage-01-cyclonedx.json`
- Standard Evidence Schema (`artifacts/evidence/stage-01/S1_P3_SP3_evidence.json`):
  ```json
  {
    "subphase": "S1.P3.SP3",
    "title": "Release Manifest, SBOM Generation & Stage 1 Certification",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S1.P3.SP3`.
2. **`TODO.md` Update:** Flip `- [ ] S1.P3.SP3` to `- [x] S1.P3.SP3` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 1 — Phase 3` summarizing the concrete deliverables and metrics of `S1.P3.SP3`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S1.P3.SP3` changes (`git reset --hard` to the `S1.P3.SP2` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S1.P3.SP3` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-01/S1_P3_SP3_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
