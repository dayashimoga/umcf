# Executable Phase Prompt — `S4.P1`: Local Resource Guard, Consent Enforcement & Lifecycle Monitor

> **Stage:** Stage 4 — Distributed Scheduler, Resource Management, Workload Execution and Recovery (`STAGE_04_PROMPT.md`)  
> **Phase:** `S4.P1` — Local Resource Guard, Consent Enforcement & Lifecycle Monitor  
> **Subphases:** `S4.P1.SP1`, `S4.P1.SP2`, `S4.P1.SP3`  
> **Target Environment:** Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are the Lead Systems Architect executing Phase **`S4.P1` (Local Resource Guard, Consent Enforcement & Lifecycle Monitor)** of Stage 4 in the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Phase Objective:** Guarantee host device safety and responsiveness by enforcing hard CPU/RAM/VRAM quotas (`cgroups v2` / Windows Job Objects / Wasm fuel & memory limiters), thermal/battery cutoffs, and `< 50 ms` interactive user activity preemption (`umcf-guard`).
- **Execution Discipline:** Execute `S4.P1.SP1`, `S4.P1.SP2`, and `S4.P1.SP3` sequentially. Do not begin a subsequent subphase until the current subphase's Completion Gate and Evidence JSON are verified.

---

## 2. Prerequisites & Dependencies

- **Upstream Dependency:** `Stage 3` must be 100% complete and certified.
- **Target Crates / Services:** `crates/umcf-guard, crates/umcf-scheduler, crates/umcf-worker`
- **Quota Enforcement:** `scripts/podman-quota-check.sh` must pass before and after each subphase (`< 29.0 GiB`).

---

## 3. Implementation Tasks & Subphase Execution Sequence

Execute the three constituent subphases in strict sequential order:

### 3.1 Subphase `S4.P1.SP1`: CPU/Memory/VRAM Cgroups & Job Object Quota Enforcer
- **Prompt File:** [`S4_P1_SP1.md`](S4_P1_SP1.md)
- **Scope:** Implement `crates/umcf-guard/src/quota.rs` enforcing hard CPU time shares/cores, resident RAM ceilings, and GPU VRAM allocations derived from the user's `ResourceConsentPolicy`.
- **Key Deliverables:**
  - Create `crates/umcf-guard` and implement `QuotaEnforcer` trait with platform backends: Linux `CgroupV2Enforcer` (`cpu.max`, `memory.max`, `memory.high`, `io.max`), Windows `JobObjectEnforcer` (`JOB_OBJECT_LIMIT_PROCESS_MEMORY`, `JOB_OBJECT_CPU_RATE_CONTROL_ENABLE`), POSIX `setrlimit`/nice fallback, and in-process `AllocatorGuard` + Wasmtime `ResourceLimiter`.
  - Implement `VramBudgetTracker` performing atomic reservation accounting against the consented `max_vram_bytes` before any GPU/LLM allocation is permitted.
  - Wire `QuotaEnforcer` to dynamically update active limits within `< 20 ms` whenever `ResourceConsentPolicy` is modified.

### 3.2 Subphase `S4.P1.SP2`: Thermal Throttling, Battery Guard & Mobile OS Lifecycle Hooks
- **Prompt File:** [`S4_P1_SP2.md`](S4_P1_SP2.md)
- **Scope:** Implement `crates/umcf-guard/src/lifecycle.rs` monitoring real-time battery drain, charging transitions, thermal sensors, and mobile OS lifecycle signals (`onTrimMemory`, `AppLifecycleState.paused`, iOS background expiration) to progressively throttle or suspend workloads.
- **Key Deliverables:**
  - Implement `ThermalBatteryGuard` state machine with 4 operational regimes: `FullCapacity` (`< 38°C`, battery `>= min_battery_percent`), `Throttled50` (`38°C..41°C`), `PausedWaitCoolOrCharge` (`>= 41°C` or battery `< min_battery_percent` or unplugged when `require_charging == true`), and `EmergencyEvict` (`>= 44°C` or OS `onTrimMemory(CRITICAL)`).
  - Implement hysteresis deadbands (`3°C` cooling deadband, `3%` battery recharge deadband) to prevent rapid oscillation (flapping) between `FullCapacity` and `PausedWaitCoolOrCharge`.
  - Emit urgent `NodeGuardEvent` (`CriticalControl` QoS class) to the cluster Master so active tasks on a thermally throttled or unplugging mobile node are proactively checkpointed and migrated.

### 3.3 Subphase `S4.P1.SP3`: Interactive User Activity Detector & Instant Preemption
- **Prompt File:** [`S4_P1_SP3.md`](S4_P1_SP3.md)
- **Scope:** Implement `crates/umcf-guard/src/preemption.rs` detecting foreground user interaction (keyboard/mouse/touch input, game/fullscreen app launch, local foreground CPU/GPU spike) and preempting or throttling background UMCF workloads in `< 50 ms` so the local user never notices lag.
- **Key Deliverables:**
  - Implement `UserActivityDetector` monitoring OS idle timers (Linux X11/Wayland/evdev idle, Windows `GetLastInputInfo`, macOS `CGEventSourceSecondsSinceLastEventType`, Android/iOS foreground app state) plus non-UMCF host CPU/GPU load delta.
  - Implement `InstantPreemptor`: when user activity transitions from `Idle` to `InteractiveActive`, drop background worker CPU quota to `ConsentedInteractiveCap` (e.g., `<= 15%` low-priority `SCHED_IDLE` / `THREAD_PRIORITY_IDLE`) or pause GPU dispatch within `< 50 ms` (`SIGSTOP` / cooperative yield barrier / Wasm epoch interruption).
  - Resume full consented compute capacity smoothly (`RampUp` over `3.0s`) after user idle timer exceeds `idle_cooldown_secs` (default `30s`).

---

## 4. Acceptance Criteria (Phase-Level Measurable Gates)

- [ ] All acceptance criteria in `S4_P1_SP1.md`, `S4_P1_SP2.md`, and `S4_P1_SP3.md` are satisfied.
- [ ] Cross-subphase integration between `S4.P1.SP1`, `S4.P1.SP2`, and `S4.P1.SP3` compiles cleanly with zero warnings (`cargo clippy --workspace --all-targets -- -D warnings`).
- [ ] Line coverage across crates modified in `S4.P1` is `>= 85.0%`.
- [ ] Zero `todo!()`, `unimplemented!()`, or placeholder stubs exist in the codebase (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Podman storage utilization remains strictly `< 29.0 GiB`.

---

## 5. Required Tests (Classified Rollup)

- **Unit Tests:** All unit test suites across `S4.P1.SP1`, `S4.P1.SP2`, and `S4.P1.SP3` pass via `cargo nextest run`.
- **Integration Tests:** End-to-end phase integration tests verifying `CPU/Memory/VRAM Cgroups & Job Object Quota Enforcer` + `Thermal Throttling, Battery Guard & Mobile OS Lifecycle Hooks` + `Interactive User Activity Detector & Instant Preemption` interoperation.
- **Simulation Tests:** Fault-injection, corrupted input, and concurrency stress tests defined in `S4.P1.SP1..SP3`.
- **Hardware-Required Tests:** Executed on target hardware or recorded in hardware execution manifests where containerized hardware passthrough is unavailable.
- **Manually Verified Tests:** Developer visual/CLI inspection checklists completed for `S4.P1.SP1..SP3`.

---

## 6. Required Evidence & Artifacts

- `artifacts/evidence/stage-04/S4_P1_SP1_evidence.json`
- `artifacts/evidence/stage-04/S4_P1_SP2_evidence.json`
- `artifacts/evidence/stage-04/S4_P1_SP3_evidence.json`
- `artifacts/evidence/stage-04/S4_P1_PHASE_SUMMARY.json` aggregating all 3 subphase evidence bundles and phase-level coverage metrics.

---

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates

- Verify `S4.P1.SP1`, `S4.P1.SP2`, and `S4.P1.SP3` are all checked `[x]` in `TODO.md`.
- Consolidate `S4.P1` capabilities into `CHANGELOG.md` and module-level architecture docs.

---

## 8. Rollback Criteria & Completion Gate

- **Rollback Criteria:** If phase integration tests fail or Podman disk quota exceeds `29.0 GiB` after cache pruning, roll back to the start of `S4.P1` (`v0.4.0-certified`) and isolate the failing subphase.
- **Completion Gate:** Tag repository checkpoint `v0.4.1-certified` once `S4_P1_PHASE_SUMMARY.json` reports `"status": "PASS"`.
