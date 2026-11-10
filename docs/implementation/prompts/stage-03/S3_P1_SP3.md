# Executable Subphase Prompt — `S3.P1.SP3`: Control-Plane vs Data-Plane Traffic Isolation & QoS

> **Stage:** Stage 3 — LAN/Remote Networking, Security, Hardware Discovery and Benchmarking (`STAGE_03_PROMPT.md`)  
> **Phase:** Phase `S3.P1` — LAN Discovery & Direct P2P QUIC Data Plane (`S3_P1_PROMPT.md`)  
> **Subphase:** `S3.P1.SP3` — Control-Plane vs Data-Plane Traffic Isolation & QoS  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S3.P1.SP3` (Control-Plane vs Data-Plane Traffic Isolation & QoS)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement strict priority scheduling, IP DSCP marking (`EF` / `CS6` for control & heartbeats, `AF41` for streaming, `AF21` / `BE` for bulk model/tensor transfers), and hierarchical token-bucket bandwidth shapers so bulk data transfers never starve cluster heartbeats or interactive streams.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S3.P1.SP2` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S3.P1.SP2` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Implement `crates/umcf-net/src/qos.rs` defining `TrafficClass`: `CriticalControl` (heartbeats, election, preemption signals; highest priority, reserved min `5%` bandwidth), `RealtimeInteractive` (remote streaming, input return, live LLM token stream), `ElasticCompute` (DAG task activations/RPC), and `BackgroundBulk` (GGUF shard sync, storage replication).
2. Configure `quinn::SendStream::set_priority` and user-space weighted fair / deficit round-robin pacing (`BandwidthShaper`) enforcing user-configured upload/download caps from `ResourceConsentPolicy`.
3. Implement OS socket DSCP / TOS byte marking (`IP_TOS` / `IPV6_TCLASS`) with graceful fallback on unprivileged sockets.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S3.P1.SP3` complete:

- [ ] Under 100% link saturation by `BackgroundBulk` model shard transfers, `CriticalControl` heartbeat and preemption messages maintain p99 latency `< 5.0 ms` on LAN.
- [ ] `BandwidthShaper` enforces configured upload/download rate limits within `±3.5%` accuracy over any `1.0s` rolling window.
- [ ] `CriticalControl` messages are never dropped by local send queues even when `BackgroundBulk` experiences backpressure.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test `TrafficClass` priority mapping, DSCP byte values, and `BandwidthShaper` token accounting.
- **Integration Tests:**
  - Saturate a shaped `100 Mbps` virtual QUIC link with `BackgroundBulk` streams while sending `100 Hz` `CriticalControl` pings; verify p99 ping latency `< 5.0 ms` and zero dropped pings.
- **Simulation / Fault-Injection Tests:**
  - Dynamically reduce `BandwidthShaper` limit from `100 Mbps` to `10 Mbps` mid-transfer and verify convergence within `< 200 ms`.
- **Hardware-Required Tests:**
  - Not Required.
- **Manually Verified Tests:**
  - Inspect QoS class telemetry counters in OpenMetrics dump.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S3.P1.SP3`:

- `artifacts/evidence/stage-03/S3_P1_SP3_evidence.json`
- `artifacts/evidence/stage-03/qos_latency_under_load.json`
- Standard Evidence Schema (`artifacts/evidence/stage-03/S3_P1_SP3_evidence.json`):
  ```json
  {
    "subphase": "S3.P1.SP3",
    "title": "Control-Plane vs Data-Plane Traffic Isolation & QoS",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S3.P1.SP3`.
2. **`TODO.md` Update:** Flip `- [ ] S3.P1.SP3` to `- [x] S3.P1.SP3` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 3 — Phase 1` summarizing the concrete deliverables and metrics of `S3.P1.SP3`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S3.P1.SP3` changes (`git reset --hard` to the `S3.P1.SP2` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S3.P1.SP3` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-03/S3_P1_SP3_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
