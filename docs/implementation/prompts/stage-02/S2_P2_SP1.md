# Executable Subphase Prompt — `S2.P2.SP1`: Short-Lived QR Enrollment Token Generator & Scanner Flow

> **Stage:** Stage 2 — Rust Core, Identity, Enrollment, Device Registry and Cluster Management (`STAGE_02_PROMPT.md`)  
> **Phase:** Phase `S2.P2` — QR Enrollment, RBAC & Dynamic Master/Worker Role Engine (`S2_P2_PROMPT.md`)  
> **Subphase:** `S2.P2.SP1` — Short-Lived QR Enrollment Token Generator & Scanner Flow  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S2.P2.SP1` (Short-Lived QR Enrollment Token Generator & Scanner Flow)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement compact, cryptographically signed, short-lived enrollment tokens (`umcf://enroll?...`), SVG/PNG/terminal-UTF8 QR code rendering, and one-time SPAKE2+/X25519 enrollment exchange in `crates/umcf-enrollment`.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S2.P1.SP3` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S2.P1.SP3` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Create `crates/umcf-enrollment/src/token.rs` defining `EnrollmentToken` containing `cluster_id`, `issuer_did`, `ephemeral_x25519_pk`, `bootstrap_endpoints: Vec<SocketAddr>`, `one_time_nonce: [u8; 16]`, `expires_at_ms` (default TTL `180s`, max `600s`), `proposed_role`, and `issuer_ed25519_sig`.
2. Implement `EnrollmentToken::to_uri()` (`umcf://enroll/v1/<base64url>`) and QR matrix generator (`to_svg()`, `to_unicode_blocks()`) in `crates/umcf-enrollment/src/qr.rs`.
3. Implement `EnrollmentHandshake` in `crates/umcf-enrollment/src/handshake.rs`: candidate scans token, verifies signature & TTL, establishes X25519 + HKDF-SHA256 encrypted channel, submits `JoinRequest` (with `DidDocument` + `HardwareCapabilityDescriptor`), and receives signed `NodeCertificate`.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S2.P2.SP1` complete:

- [ ] `EnrollmentToken` URI fits within `<= 320` ASCII characters for fast mobile camera recognition at QR Version 10 (Error Correction Level M).
- [ ] Tokens older than `expires_at_ms` or reused a second time (`one_time_nonce` replay cache) are immediately rejected (`ErrorCode::E2101_TOKEN_EXPIRED` / `E2102_TOKEN_REPLAYED`).
- [ ] Complete enrollment handshake executes in `< 50 ms` over local loopback.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test `EnrollmentToken` creation, URI encoding/decoding, SVG/Unicode QR generation, TTL expiration, and signature tampering.
- **Integration Tests:**
  - Execute full Master -> QR Token -> Candidate Scan -> `JoinRequest` -> `NodeCertificate` issuance flow.
- **Simulation / Fault-Injection Tests:**
  - Attempt concurrent replay of the same `EnrollmentToken` by two distinct candidate nodes and verify exactly one succeeds and the second is rejected.
- **Hardware-Required Tests:**
  - Optional mobile camera scan of generated SVG QR code; automated decoder test verifies QR matrix readability.
- **Manually Verified Tests:**
  - Render generated SVG and terminal UTF-8 QR code and verify URI payload.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S2.P2.SP1`:

- `artifacts/evidence/stage-02/S2_P2_SP1_evidence.json`
- `artifacts/evidence/stage-02/sample_enrollment_qr.svg`
- Standard Evidence Schema (`artifacts/evidence/stage-02/S2_P2_SP1_evidence.json`):
  ```json
  {
    "subphase": "S2.P2.SP1",
    "title": "Short-Lived QR Enrollment Token Generator & Scanner Flow",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S2.P2.SP1`.
2. **`TODO.md` Update:** Flip `- [ ] S2.P2.SP1` to `- [x] S2.P2.SP1` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 2 — Phase 2` summarizing the concrete deliverables and metrics of `S2.P2.SP1`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S2.P2.SP1` changes (`git reset --hard` to the `S2.P1.SP3` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S2.P2.SP1` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-02/S2_P2_SP1_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
