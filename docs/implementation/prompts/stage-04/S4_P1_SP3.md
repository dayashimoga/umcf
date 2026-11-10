# Executable Subphase Prompt — `S4.P1.SP3`: Interactive User Activity Detector & Instant Preemption

> **Stage:** Stage 4 — Distributed Scheduler, Resource Management, Workload Execution and Recovery (`STAGE_04_PROMPT.md`)  
> **Phase:** Phase `S4.P1` — Local Resource Guard, Consent Enforcement & Lifecycle Monitor (`S4_P1_PROMPT.md`)  
> **Subphase:** `S4.P1.SP3` — Interactive User Activity Detector & Instant Preemption  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S4.P1.SP3` (Interactive User Activity Detector & Instant Preemption)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement `crates/umcf-guard/src/preemption.rs` detecting foreground user interaction (keyboard/mouse/touch input, game/fullscreen app launch, local foreground CPU/GPU spike) and preempting or throttling background UMCF workloads in `< 50 ms` so the local user never notices lag.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S4.P1.SP2` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S4.P1.SP2` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Implement `UserActivityDetector` monitoring OS idle timers (Linux X11/Wayland/evdev idle, Windows `GetLastInputInfo`, macOS `CGEventSourceSecondsSinceLastEventType`, Android/iOS foreground app state) plus non-UMCF host CPU/GPU load delta.
2. Implement `InstantPreemptor`: when user activity transitions from `Idle` to `InteractiveActive`, drop background worker CPU quota to `ConsentedInteractiveCap` (e.g., `<= 15%` low-priority `SCHED_IDLE` / `THREAD_PRIORITY_IDLE`) or pause GPU dispatch within `< 50 ms` (`SIGSTOP` / cooperative yield barrier / Wasm epoch interruption).
3. Resume full consented compute capacity smoothly (`RampUp` over `3.0s`) after user idle timer exceeds `idle_cooldown_secs` (default `30s`).

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S4.P1.SP3` complete:

- [ ] Preemption latency from `UserInputDetected` event to worker CPU/GPU suspension is `< 50.0 ms` at p99.
- [ ] Zero task data corruption occurs during preemption; in-flight tasks either pause cleanly in place or yield a checkpoint for migration.
- [ ] Host foreground responsiveness stays unimpacted during synthetic interactive bursts.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test `UserActivityDetector` state machine (`Idle -> InteractiveActive -> Cooldown -> Idle`) and ramp-up curve.
- **Integration Tests:**
  - Run a continuous compute workload in `umcf-worker`, inject `UserInputDetected` signal, and measure exact microseconds until worker threads park (`must be < 50,000 µs`).
- **Simulation / Fault-Injection Tests:**
  - Inject rapid 100 Hz user input bursts interleaved with idle gaps and verify zero task state corruption and smooth cooldown behavior.
- **Hardware-Required Tests:**
  - Not Required.
- **Manually Verified Tests:**
  - Verify preemption latency histogram in evidence output.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S4.P1.SP3`:

- `artifacts/evidence/stage-04/S4_P1_SP3_evidence.json`
- `artifacts/evidence/stage-04/preemption_latency_histogram.json`
- Standard Evidence Schema (`artifacts/evidence/stage-04/S4_P1_SP3_evidence.json`):
  ```json
  {
    "subphase": "S4.P1.SP3",
    "title": "Interactive User Activity Detector & Instant Preemption",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S4.P1.SP3`.
2. **`TODO.md` Update:** Flip `- [ ] S4.P1.SP3` to `- [x] S4.P1.SP3` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 4 — Phase 1` summarizing the concrete deliverables and metrics of `S4.P1.SP3`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S4.P1.SP3` changes (`git reset --hard` to the `S4.P1.SP2` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S4.P1.SP3` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-04/S4_P1_SP3_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
