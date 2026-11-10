# Executable Subphase Prompt — `S3.P2.SP3`: Roaming, Network Interface Handover & Connection Migration

> **Stage:** Stage 3 — LAN/Remote Networking, Security, Hardware Discovery and Benchmarking (`STAGE_03_PROMPT.md`)  
> **Phase:** Phase `S3.P2` — NAT Traversal, STUN/ICE Hole Punching & Relay Fallback (`S3_P2_PROMPT.md`)  
> **Subphase:** `S3.P2.SP3` — Roaming, Network Interface Handover & Connection Migration  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S3.P2.SP3` (Roaming, Network Interface Handover & Connection Migration)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement proactive network interface monitoring (Wi-Fi <-> Cellular <-> Ethernet handover, IPv4 <-> IPv6 change) and RFC 9000 QUIC Connection Migration (`PATH_CHALLENGE` / `PATH_RESPONSE`) so active jobs and streams survive mobile roaming without disconnecting.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S3.P2.SP2` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S3.P2.SP2` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Implement `crates/umcf-net/src/roaming.rs` (`NetworkInterfaceWatcher`) polling/listening for local IP address changes, interface up/down events, and Wi-Fi -> Cellular metric changes.
2. Implement active socket rebinding (`Endpoint::rebind`) and RFC 9000 QUIC connection migration in `QuicTransportEngine` so existing `ConnectionId` sessions migrate to the new local IP/port in `< 250 ms`.
3. Publish updated `IceCandidateSet` to the cluster registry upon interface change so newly joining peers also know the node's updated endpoint.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S3.P2.SP3` complete:

- [ ] Active bidirectional QUIC streams survive 5 consecutive local IP/port rebinds (`Wi-Fi -> Cellular -> Wi-Fi`) with `0` broken streams and `< 250 ms` transient pause.
- [ ] If the new interface is behind Symmetric NAT where direct UDP fails, the connection falls back to `UmcfRelayServer` within `< 1000 ms` without resetting the logical peer session.
- [ ] Stale sockets are closed cleanly with zero file descriptor leaks.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test interface diff detector (`AddedAddr`, `RemovedAddr`, `PrimaryInterfaceChanged`) and rebind debounce logic.
- **Integration Tests:**
  - Stream a `50 MiB` payload over QUIC while triggering 3 live socket rebinds to new ephemeral UDP ports/IPs; verify BLAKE3 checksum of received stream matches 100%.
- **Simulation / Fault-Injection Tests:**
  - Simulate abrupt Wi-Fi interface blackhole (`100%` packet loss on old socket) while bringing up secondary socket; verify migration completes before stream idle timeout.
- **Hardware-Required Tests:**
  - Optional smartphone Wi-Fi toggle test; simulated multi-socket rebind executes in Podman.
- **Manually Verified Tests:**
  - Review roaming event telemetry log and path migration counters.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S3.P2.SP3`:

- `artifacts/evidence/stage-03/S3_P2_SP3_evidence.json`
- `artifacts/evidence/stage-03/connection_migration_trace.json`
- Standard Evidence Schema (`artifacts/evidence/stage-03/S3_P2_SP3_evidence.json`):
  ```json
  {
    "subphase": "S3.P2.SP3",
    "title": "Roaming, Network Interface Handover & Connection Migration",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S3.P2.SP3`.
2. **`TODO.md` Update:** Flip `- [ ] S3.P2.SP3` to `- [x] S3.P2.SP3` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 3 — Phase 2` summarizing the concrete deliverables and metrics of `S3.P2.SP3`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S3.P2.SP3` changes (`git reset --hard` to the `S3.P2.SP2` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S3.P2.SP3` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-03/S3_P2_SP3_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
