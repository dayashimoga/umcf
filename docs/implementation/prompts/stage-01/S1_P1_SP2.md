# Executable Subphase Prompt — `S1.P1.SP2`: Podman 30GB Container Toolchains & Quota Enforcer

> **Stage:** Stage 1 — Engineering Foundation, Repository and Development Infrastructure (`STAGE_01_PROMPT.md`)  
> **Phase:** Phase `S1.P1` — Repository Governance, Monorepo Workspace & Podman 30GB Reproducible Environment (`S1_P1_PROMPT.md`)  
> **Subphase:** `S1.P1.SP2` — Podman 30GB Container Toolchains & Quota Enforcer  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S1.P1.SP2` (Podman 30GB Container Toolchains & Quota Enforcer)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Construct rootless Podman container definitions (`Podmanfile.rust`, `Podmanfile.flutter`, `Podmanfile.web`, `compose.podman.yml`) and the hard 30 GB disk quota enforcement daemon/script.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S1.P1.SP1` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S1.P1.SP1` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Create `containers/Podmanfile.rust` using Wolfi/Alpine minimal base with Rust stable toolchain (`1.81+`), `wasm32-wasip1`, `cargo-nextest`, `cargo-llvm-cov`, and `sccache`.
2. Create `containers/Podmanfile.web` (Node 22 + pnpm + Wrangler) and `containers/Podmanfile.flutter` (Flutter stable SDK + Linux/Android build tools with stripped NDK symbols).
3. Create `compose.podman.yml` mounting workspace root with rootless UID mapping (`userns_mode: keep-id`) and bounded named volumes (`umcf_cargo_cache`, `umcf_sccache`).
4. Implement `scripts/podman-quota-check.sh` querying `podman system df --format json`, host `target/`, and cache volumes; enforce Soft Warning at `25.0 GiB` (`26_843_545_600` bytes) and Hard Abort + auto-prune (`cargo sweep` / `sccache --stop-server` / `podman system prune`) at `29.0 GiB` (`31_138_512_896` bytes).

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S1.P1.SP2` complete:

- [ ] Combined rootless Podman images (`rust`, `web`, `flutter`) + build caches occupy `<= 18.5 GiB` on clean build, strictly below the `29.0 GiB` hard gate (`< 30 GB` ceiling).
- [ ] `scripts/podman-quota-check.sh --json` outputs valid JSON schema `{ "images_bytes": ..., "volumes_bytes": ..., "target_bytes": ..., "total_bytes": ..., "limit_bytes": 31138512896, "status": "PASS" }`.
- [ ] All containers run rootless (`id -u` maps cleanly without host root privileges).
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test `scripts/podman-quota-check.sh` math & threshold evaluator with mocked `podman system df` JSON payloads (`20 GiB` PASS, `26 GiB` WARN, `29.5 GiB` FAIL).
- **Integration Tests:**
  - Build `Podmanfile.rust` and `Podmanfile.web` via `podman build` and verify `podman-quota-check.sh` passes.
- **Simulation / Fault-Injection Tests:**
  - Simulate a `30.1 GiB` synthetic byte counter in `PODMAN_QUOTA_SIMULATE_BYTES` env var and verify automatic cache prune trigger and non-zero exit code.
- **Hardware-Required Tests:**
  - Host workstation/laptop with Podman rootless storage driver (`overlayfs`) and `>= 30 GB` disk partition.
- **Manually Verified Tests:**
  - Verify `podman info` confirms rootless execution (`rootless: true`) and storage driver health.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S1.P1.SP2`:

- `artifacts/evidence/stage-01/S1_P1_SP2_evidence.json` containing `podman system df` output and quota check telemetry.
- `artifacts/evidence/stage-01/podman_images_digests.txt` listing image IDs and compressed/uncompressed sizes.
- Standard Evidence Schema (`artifacts/evidence/stage-01/S1_P1_SP2_evidence.json`):
  ```json
  {
    "subphase": "S1.P1.SP2",
    "title": "Podman 30GB Container Toolchains & Quota Enforcer",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S1.P1.SP2`.
2. **`TODO.md` Update:** Flip `- [ ] S1.P1.SP2` to `- [x] S1.P1.SP2` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 1 — Phase 1` summarizing the concrete deliverables and metrics of `S1.P1.SP2`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S1.P1.SP2` changes (`git reset --hard` to the `S1.P1.SP1` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S1.P1.SP2` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-01/S1_P1_SP2_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
