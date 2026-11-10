# Executable Subphase Prompt — `S1.P2.SP3`: Structured Telemetry, Tracing & Error Taxonomy

> **Stage:** Stage 1 — Engineering Foundation, Repository and Development Infrastructure (`STAGE_01_PROMPT.md`)  
> **Phase:** Phase `S1.P2` — Versioned Wire Protocol, Core Domain Types & Capability Schemas (`S1_P2_PROMPT.md`)  
> **Subphase:** `S1.P2.SP3` — Structured Telemetry, Tracing & Error Taxonomy  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S1.P2.SP3` (Structured Telemetry, Tracing & Error Taxonomy)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement `crates/umcf-telemetry` providing W3C TraceContext propagation across `WireEnvelope`, OpenTelemetry JSON/OTLP exporters, Prometheus/OpenMetrics counters/histograms, and standardized error codes (`UmcfError`).
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S1.P2.SP2` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S1.P2.SP2` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Implement `crates/umcf-telemetry/src/error.rs` defining `ErrorCode` (`E1xxx` Protocol, `E2xxx` Identity/Auth, `E3xxx` Network, `E4xxx` Scheduler/Resource, `E5xxx` Inference, `E6xxx` Media/GPU, `E7xxx` Storage/Ledger, `E8xxx` Internal), `Retryability` (`RetryableAfter(Duration)`, `RetryableImmediate`, `Fatal`), and RFC-9457 Problem Details JSON serialization.
2. Implement `crates/umcf-telemetry/src/tracing_init.rs` configuring `tracing-subscriber` with structured JSON formatting, redactable sensitive fields (`SecretRedactor`), and W3C `traceparent` / `tracestate` injection/extraction into `umcf_proto::WireEnvelope`.
3. Implement `crates/umcf-telemetry/src/metrics.rs` providing lock-free atomic counters, gauges, and HDR latency histograms exportable in OpenMetrics text format.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S1.P2.SP3` complete:

- [ ] Every `UmcfError` variant maps deterministically to a unique `E{1..8}xxx` code, HTTP status code, gRPC code, and `Retryability` classification.
- [ ] Secret values wrapped in `Redacted<T>` never appear in `Debug`, `Display`, or JSON `tracing` logs (verified by automated leak test).
- [ ] Distributed trace `trace_id` and `span_id` survive round-trip injection and extraction across `WireEnvelope` headers.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test `ErrorCode` mappings, RFC-9457 serialization, and `Redacted<T>` formatting.
- **Integration Tests:**
  - Capture structured JSON log output from a multi-span async workflow and verify parent-child span linkage across simulated wire boundaries.
- **Simulation / Fault-Injection Tests:**
  - Inject synthetic private keys into logged structs wrapped with `Redacted` and scan output buffer to guarantee zero byte matches.
- **Hardware-Required Tests:**
  - Not Required.
- **Manually Verified Tests:**
  - Inspect sample OpenMetrics output and RFC-9457 error payload formatting.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S1.P2.SP3`:

- `artifacts/evidence/stage-01/S1_P2_SP3_evidence.json`
- `artifacts/evidence/stage-01/error_taxonomy_catalog.json`
- Standard Evidence Schema (`artifacts/evidence/stage-01/S1_P2_SP3_evidence.json`):
  ```json
  {
    "subphase": "S1.P2.SP3",
    "title": "Structured Telemetry, Tracing & Error Taxonomy",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S1.P2.SP3`.
2. **`TODO.md` Update:** Flip `- [ ] S1.P2.SP3` to `- [x] S1.P2.SP3` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 1 — Phase 2` summarizing the concrete deliverables and metrics of `S1.P2.SP3`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S1.P2.SP3` changes (`git reset --hard` to the `S1.P2.SP2` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S1.P2.SP3` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-01/S1_P2_SP3_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
