# Executable Subphase Prompt — `S2.P1.SP3`: Mutual Attestation & Signed Identity Certificates

> **Stage:** Stage 2 — Rust Core, Identity, Enrollment, Device Registry and Cluster Management (`STAGE_02_PROMPT.md`)  
> **Phase:** Phase `S2.P1` — Cryptographic Device Identity & Secure Key Storage (`S2_P1_PROMPT.md`)  
> **Subphase:** `S2.P1.SP3` — Mutual Attestation & Signed Identity Certificates  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S2.P1.SP3` (Mutual Attestation & Signed Identity Certificates)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement cluster-scoped X.509v3 (for QUIC mTLS) and compact CBOR/JSON Cluster Membership Certificates (`NodeCertificate`), ephemeral challenge-response mutual attestation, and Certificate Revocation Lists (`ClusterCrl`).
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S2.P1.SP2` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S2.P1.SP2` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Implement `crates/umcf-identity/src/cert.rs` generating self-signed root Cluster CA certificates and leaf `NodeCertificate` (containing custom X.509 SAN extension with node `did:key`, `ClusterId`, assigned `NodeRole`, and expiration timestamp) using `rcgen` and `rustls-pki-types`.
2. Implement `crates/umcf-identity/src/attestation.rs` defining the 2-RTT `MutualAttestationProtocol` (`AttestationChallenge` with 32-byte nonce -> `AttestationProof` binding TLS exporter keying material + hardware fingerprint + Ed25519 signature).
3. Implement `crates/umcf-identity/src/crl.rs` for signed `ClusterCrl` distribution and O(1) revoked certificate serial/DID lookup.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S2.P1.SP3` complete:

- [ ] Generated X.509 leaf certificates validate cleanly against the Cluster CA using `rustls::client::WebPkiServerVerifier` and `rustls::server::WebPkiClientVerifier`.
- [ ] `MutualAttestationProtocol` binds to RFC 8446 TLS 1.3 exporter (`EXPORTER-UMCF-ATTEST-v1`) preventing MITM relay attacks.
- [ ] Revoked `NodeId` / `SenderDid` entries in a signed `ClusterCrl` are rejected within `< 10 µs`.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test Cluster CA generation, leaf certificate signing, SAN `did:key` extraction, expiry rejection, and CRL verification.
- **Integration Tests:**
  - Establish an in-memory `rustls` TLS 1.3 mutual-auth handshake between two nodes using generated `NodeCertificate`s and complete `MutualAttestationProtocol`.
- **Simulation / Fault-Injection Tests:**
  - Simulate MITM nonce replay, expired certificate (`now + 366 days`), and revoked serial number in `ClusterCrl`.
- **Hardware-Required Tests:**
  - Not Required.
- **Manually Verified Tests:**
  - Inspect decoded X.509 certificate extensions via `openssl x509 -text -noout` or parser dump.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S2.P1.SP3`:

- `artifacts/evidence/stage-02/S2_P1_SP3_evidence.json`
- `artifacts/evidence/stage-02/mtls_attestation_transcript.json`
- Standard Evidence Schema (`artifacts/evidence/stage-02/S2_P1_SP3_evidence.json`):
  ```json
  {
    "subphase": "S2.P1.SP3",
    "title": "Mutual Attestation & Signed Identity Certificates",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S2.P1.SP3`.
2. **`TODO.md` Update:** Flip `- [ ] S2.P1.SP3` to `- [x] S2.P1.SP3` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 2 — Phase 1` summarizing the concrete deliverables and metrics of `S2.P1.SP3`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S2.P1.SP3` changes (`git reset --hard` to the `S2.P1.SP2` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S2.P1.SP3` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-02/S2_P1_SP3_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
