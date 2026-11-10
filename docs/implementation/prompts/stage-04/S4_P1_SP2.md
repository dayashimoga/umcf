# Executable Subphase Prompt — `S4.P1.SP2`: Thermal Throttling, Battery Guard & Mobile OS Lifecycle Hooks

> **Stage:** Stage 4 — Distributed Scheduler, Resource Management, Workload Execution and Recovery (`STAGE_04_PROMPT.md`)  
> **Phase:** Phase `S4.P1` — Local Resource Guard, Consent Enforcement & Lifecycle Monitor (`S4_P1_PROMPT.md`)  
> **Subphase:** `S4.P1.SP2` — Thermal Throttling, Battery Guard & Mobile OS Lifecycle Hooks  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S4.P1.SP2` (Thermal Throttling, Battery Guard & Mobile OS Lifecycle Hooks)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement `crates/umcf-guard/src/lifecycle.rs` monitoring real-time battery drain, charging transitions, thermal sensors, and mobile OS lifecycle signals (`onTrimMemory`, `AppLifecycleState.paused`, iOS background expiration) to progressively throttle or suspend workloads.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S4.P1.SP1` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S4.P1.SP1` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Implement `ThermalBatteryGuard` state machine with 4 operational regimes: `FullCapacity` (`< 38°C`, battery `>= min_battery_percent`), `Throttled50` (`38°C..41°C`), `PausedWaitCoolOrCharge` (`>= 41°C` or battery `< min_battery_percent` or unplugged when `require_charging == true`), and `EmergencyEvict` (`>= 44°C` or OS `onTrimMemory(CRITICAL)`).
2. Implement hysteresis deadbands (`3°C` cooling deadband, `3%` battery recharge deadband) to prevent rapid oscillation (flapping) between `FullCapacity` and `PausedWaitCoolOrCharge`.
3. Emit urgent `NodeGuardEvent` (`CriticalControl` QoS class) to the cluster Master so active tasks on a thermally throttled or unplugging mobile node are proactively checkpointed and migrated.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S4.P1.SP2` complete:

- [ ] Unplugging AC power when `require_charging == true` or crossing `>= 41.0°C` transitions `ThermalBatteryGuard` to `PausedWaitCoolOrCharge` and suspends local compute threads within `< 50 ms`.
- [ ] Hysteresis prevents state flapping when temperature oscillates between `40.5°C` and `41.2°C` (requires cooling to `<= 38.0°C` before resuming).
- [ ] Mobile OS `onTrimMemory` / background lifecycle hook triggers checkpoint & release of cached buffers within `< 100 ms`.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test `ThermalBatteryGuard` regime transitions and hysteresis deadband logic across temperature and battery trajectories.
- **Integration Tests:**
  - Feed simulated sensor stream (`35°C -> 42°C -> 40°C -> 37°C`) into a running worker and verify task throttle, pause, Master notification, and resumption.
- **Simulation / Fault-Injection Tests:**
  - Simulate abrupt battery drop (`25% -> 14%`) + AC unplug event and verify immediate worker suspension and `NodeGuardEvent::EvacuateNode` emission.
- **Hardware-Required Tests:**
  - Not Required (reads real sensors via `umcf-hwprobe` or deterministic sensor stream in tests).
- **Manually Verified Tests:**
  - Review guard regime transition timeline log.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S4.P1.SP2`:

- `artifacts/evidence/stage-04/S4_P1_SP2_evidence.json`
- `artifacts/evidence/stage-04/thermal_battery_hysteresis_trace.json`
- Standard Evidence Schema (`artifacts/evidence/stage-04/S4_P1_SP2_evidence.json`):
  ```json
  {
    "subphase": "S4.P1.SP2",
    "title": "Thermal Throttling, Battery Guard & Mobile OS Lifecycle Hooks",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S4.P1.SP2`.
2. **`TODO.md` Update:** Flip `- [ ] S4.P1.SP2` to `- [x] S4.P1.SP2` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 4 — Phase 1` summarizing the concrete deliverables and metrics of `S4.P1.SP2`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S4.P1.SP2` changes (`git reset --hard` to the `S4.P1.SP1` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S4.P1.SP2` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-04/S4_P1_SP2_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
