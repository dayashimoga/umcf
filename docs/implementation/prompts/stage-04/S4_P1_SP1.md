# Executable Subphase Prompt — `S4.P1.SP1`: CPU/Memory/VRAM Cgroups & Job Object Quota Enforcer

> **Stage:** Stage 4 — Distributed Scheduler, Resource Management, Workload Execution and Recovery (`STAGE_04_PROMPT.md`)  
> **Phase:** Phase `S4.P1` — Local Resource Guard, Consent Enforcement & Lifecycle Monitor (`S4_P1_PROMPT.md`)  
> **Subphase:** `S4.P1.SP1` — CPU/Memory/VRAM Cgroups & Job Object Quota Enforcer  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S4.P1.SP1` (CPU/Memory/VRAM Cgroups & Job Object Quota Enforcer)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement `crates/umcf-guard/src/quota.rs` enforcing hard CPU time shares/cores, resident RAM ceilings, and GPU VRAM allocations derived from the user's `ResourceConsentPolicy`.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S3.P3.SP3` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- Stage 3 complete (`S3.P3.SP3`)
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Create `crates/umcf-guard` and implement `QuotaEnforcer` trait with platform backends: Linux `CgroupV2Enforcer` (`cpu.max`, `memory.max`, `memory.high`, `io.max`), Windows `JobObjectEnforcer` (`JOB_OBJECT_LIMIT_PROCESS_MEMORY`, `JOB_OBJECT_CPU_RATE_CONTROL_ENABLE`), POSIX `setrlimit`/nice fallback, and in-process `AllocatorGuard` + Wasmtime `ResourceLimiter`.
2. Implement `VramBudgetTracker` performing atomic reservation accounting against the consented `max_vram_bytes` before any GPU/LLM allocation is permitted.
3. Wire `QuotaEnforcer` to dynamically update active limits within `< 20 ms` whenever `ResourceConsentPolicy` is modified.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S4.P1.SP1` complete:

- [ ] A worker task attempting to allocate beyond `ResourceConsentPolicy.max_ram_bytes` or `max_vram_bytes` is deterministically denied (`ErrorCode::E4001_MEMORY_QUOTA_EXCEEDED`) without crashing the host daemon.
- [ ] CPU utilization of worker tasks stays within `±5%` of `ResourceConsentPolicy.max_cpu_percent` over a `2.0s` measurement window.
- [ ] When a task terminates or is cancelled, `100%` of its reserved RAM/VRAM quota is released immediately.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test `VramBudgetTracker` concurrent reservations, overflow rejection, and RAII lease drop release.
- **Integration Tests:**
  - Spawn a CPU- and memory-intensive worker workload under a `50%` CPU / `64 MiB` RAM quota and verify enforcement.
- **Simulation / Fault-Injection Tests:**
  - Simulate a runaway allocation loop requesting `1 GiB` inside a `64 MiB` guarded task and verify clean `E4001_MEMORY_QUOTA_EXCEEDED` termination within `< 10 ms`.
- **Hardware-Required Tests:**
  - Linux cgroups v2 / POSIX resource limits inside Podman container.
- **Manually Verified Tests:**
  - Inspect quota lease telemetry counters before, during, and after task execution.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S4.P1.SP1`:

- `artifacts/evidence/stage-04/S4_P1_SP1_evidence.json`
- `artifacts/evidence/stage-04/quota_enforcement_report.json`
- Standard Evidence Schema (`artifacts/evidence/stage-04/S4_P1_SP1_evidence.json`):
  ```json
  {
    "subphase": "S4.P1.SP1",
    "title": "CPU/Memory/VRAM Cgroups & Job Object Quota Enforcer",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S4.P1.SP1`.
2. **`TODO.md` Update:** Flip `- [ ] S4.P1.SP1` to `- [x] S4.P1.SP1` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 4 — Phase 1` summarizing the concrete deliverables and metrics of `S4.P1.SP1`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S4.P1.SP1` changes (`git reset --hard` to the `S3.P3.SP3` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S4.P1.SP1` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-04/S4_P1_SP1_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
