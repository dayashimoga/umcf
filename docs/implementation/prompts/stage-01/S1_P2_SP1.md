# Executable Subphase Prompt — `S1.P2.SP1`: `umcf-proto` Envelope & Semantic Versioning Negotiation

> **Stage:** Stage 1 — Engineering Foundation, Repository and Development Infrastructure (`STAGE_01_PROMPT.md`)  
> **Phase:** Phase `S1.P2` — Versioned Wire Protocol, Core Domain Types & Capability Schemas (`S1_P2_PROMPT.md`)  
> **Subphase:** `S1.P2.SP1` — `umcf-proto` Envelope & Semantic Versioning Negotiation  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S1.P2.SP1` (`umcf-proto` Envelope & Semantic Versioning Negotiation)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement wire framing, magic-byte validation, binary/JSON codec abstraction, Ed25519 signature footer slots, and semantic version capability handshake in `crates/umcf-proto`.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S1.P1.SP3` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S1.P1.SP3` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Implement `crates/umcf-proto/src/envelope.rs` defining `WireEnvelope` with 4-byte magic header `UMCF` (`0x554D4346`), `ProtocolVersion { major: u16, minor: u16, patch: u16 }`, `MessageFlags` bitflags (`COMPRESSED_ZSTD`, `ENCRYPTED_PAYLOAD`, `REQUIRES_ACK`), `MessageId(u128)`, `TimestampHlc`, `SenderDid`, `PayloadType`, `payload: Bytes`, and `signature: [u8; 64]`.
2. Implement `crates/umcf-proto/src/negotiation.rs` defining `HandshakeOffer`, `HandshakeAccept`, `HandshakeReject`, and deterministic `negotiate_version(local_range, remote_range) -> Result<ProtocolVersion, NegotiationError>`.
3. Implement length-delimited async framing codec (`UmcfFrameCodec`) in `crates/umcf-proto/src/codec.rs` with configurable maximum frame size (default `16 MiB` control plane, `64 MiB` data plane) and BLAKE3 payload integrity verification.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S1.P2.SP1` complete:

- [ ] Round-trip serialization/deserialization of `WireEnvelope` takes `< 2.5 µs` for a `1 KiB` payload on standard x86_64/ARM64.
- [ ] Frames exceeding `max_frame_bytes` or bearing invalid magic bytes (`!= 0x554D4346`) or corrupted BLAKE3 checksums are rejected in O(1) header inspection before allocating payload buffers.
- [ ] `negotiate_version` selects the highest mutually compatible `(major, minor)` version and rejects mismatched major versions with `NegotiationError::IncompatibleMajorVersion`.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test `WireEnvelope` round-trip encoding, flag combinations, Zstd compression toggle, and version negotiation matrix.
- **Integration Tests:**
  - Stream 10,000 framed `WireEnvelope` messages over an in-memory duplex `tokio::io::DuplexStream` using `UmcfFrameCodec`.
- **Simulation / Fault-Injection Tests:**
  - Inject bit-flips into frame header, length prefix (`0xFFFFFFFF` OOM attack), and payload bytes via `proptest` (10,000 cases) ensuring zero panics or OOM allocations.
- **Hardware-Required Tests:**
  - Not Required.
- **Manually Verified Tests:**
  - Inspect generated Rustdoc for `umcf-proto` (`cargo doc -p umcf-proto --no-deps`).

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S1.P2.SP1`:

- `artifacts/evidence/stage-01/S1_P2_SP1_evidence.json` containing proptest seed logs and micro-benchmark latency stats.
- `artifacts/evidence/stage-01/umcf_proto_test_report.xml` (JUnit nextest report).
- Standard Evidence Schema (`artifacts/evidence/stage-01/S1_P2_SP1_evidence.json`):
  ```json
  {
    "subphase": "S1.P2.SP1",
    "title": "`umcf-proto` Envelope & Semantic Versioning Negotiation",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S1.P2.SP1`.
2. **`TODO.md` Update:** Flip `- [ ] S1.P2.SP1` to `- [x] S1.P2.SP1` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 1 — Phase 2` summarizing the concrete deliverables and metrics of `S1.P2.SP1`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S1.P2.SP1` changes (`git reset --hard` to the `S1.P1.SP3` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S1.P2.SP1` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-01/S1_P2_SP1_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
