# Executable Subphase Prompt — `S1.P3.SP2`: Flutter Multi-OS CI & Artifact Packaging

> **Stage:** Stage 1 — Engineering Foundation, Repository and Development Infrastructure (`STAGE_01_PROMPT.md`)  
> **Phase:** Phase `S1.P3` — Multi-Platform CI/CD Pipelines, SBOM, Checksums & Baseline Verification (`S1_P3_PROMPT.md`)  
> **Subphase:** `S1.P3.SP2` — Flutter Multi-OS CI & Artifact Packaging  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S1.P3.SP2` (Flutter Multi-OS CI & Artifact Packaging)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Configure Flutter static analysis, widget/unit testing, rust_bridge FFI bindings generator stub, and multi-OS build verification pipeline (`apps/flutter_client`).
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S1.P3.SP1` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S1.P3.SP1` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Create `.github/workflows/ci-flutter.yml` and `scripts/ci-flutter-podman.sh` executing `flutter analyze --fatal-infos --fatal-warnings` and `flutter test --coverage`.
2. Implement typed Dart models in `apps/flutter_client/lib/src/models/hardware_capability.dart` matching `schemas/hardware_capability_v1.schema.json`.
3. Create initial status dashboard widget in `apps/flutter_client/lib/src/ui/foundation_status_screen.dart` displaying protocol version, node readiness, and theme adaptation (mobile/desktop breakpoints).

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S1.P3.SP2` complete:

- [ ] `flutter analyze --fatal-infos --fatal-warnings` exits `0` with zero issues.
- [ ] `flutter test` passes all unit and responsive widget tests (mobile `390x844` and desktop `1440x900` viewports).
- [ ] Dart `HardwareCapabilityDescriptor` model passes round-trip JSON compatibility tests against Rust-generated fixtures.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test Dart `HardwareCapabilityDescriptor.fromJson` / `toJson` against golden JSON fixtures emitted by `umcf-core`.
- **Integration Tests:**
  - Run Flutter widget tests verifying responsive layout switch between bottom navigation bar (`< 600px`) and navigation rail (`>= 600px`).
- **Simulation / Fault-Injection Tests:**
  - Simulate malformed JSON capability payloads in Dart client and verify graceful error state rendering.
- **Hardware-Required Tests:**
  - Optional physical Android/Desktop display; headless Flutter test runner executes in Podman.
- **Manually Verified Tests:**
  - Inspect golden widget screenshots generated during `flutter test`.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S1.P3.SP2`:

- `artifacts/evidence/stage-01/S1_P3_SP2_evidence.json`
- `artifacts/evidence/stage-01/flutter_test_lcov.info`
- Standard Evidence Schema (`artifacts/evidence/stage-01/S1_P3_SP2_evidence.json`):
  ```json
  {
    "subphase": "S1.P3.SP2",
    "title": "Flutter Multi-OS CI & Artifact Packaging",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S1.P3.SP2`.
2. **`TODO.md` Update:** Flip `- [ ] S1.P3.SP2` to `- [x] S1.P3.SP2` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 1 — Phase 3` summarizing the concrete deliverables and metrics of `S1.P3.SP2`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S1.P3.SP2` changes (`git reset --hard` to the `S1.P3.SP1` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S1.P3.SP2` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-01/S1_P3_SP2_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
