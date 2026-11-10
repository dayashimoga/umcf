# Executable Subphase Prompt — `S2.P1.SP1`: Ed25519/X25519 Node Identity & DID Generation

> **Stage:** Stage 2 — Rust Core, Identity, Enrollment, Device Registry and Cluster Management (`STAGE_02_PROMPT.md`)  
> **Phase:** Phase `S2.P1` — Cryptographic Device Identity & Secure Key Storage (`S2_P1_PROMPT.md`)  
> **Subphase:** `S2.P1.SP1` — Ed25519/X25519 Node Identity & DID Generation  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S2.P1.SP1` (Ed25519/X25519 Node Identity & DID Generation)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Create `crates/umcf-identity` with hardware-backed CSPRNG Ed25519 keypair generation, deterministic X25519 birational map derivation, and W3C `did:key` + `did:peer` document generation/resolution.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S1.P3.SP3` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- Stage 1 complete (`S1.P3.SP3`)
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Create `crates/umcf-identity/src/keys.rs` implementing `NodeKeypair` (using `ed25519-dalek` v2 and `x25519-dalek` with `zeroize::ZeroizeOnDrop`), signing/verification helpers, and domain-separated signatures (`UMCF-SIG-v1:`).
2. Implement `crates/umcf-identity/src/did.rs` generating multicodec base58btc `did:key:z6Mk...` identifiers and full W3C `DidDocument` structs containing `Ed25519VerificationKey2020` and `X25519KeyAgreementKey2020` verification methods.
3. Implement `WireEnvelope` signing and verification traits in `crates/umcf-identity/src/envelope_auth.rs` binding `SenderDid` to the Ed25519 public key.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S2.P1.SP1` complete:

- [ ] Keypair memory is zeroized on drop (`ZeroizeOnDrop` verified).
- [ ] `did:key` encoding and decoding conforms strictly to W3C `did:key` multicodec prefix `0xed01` for Ed25519.
- [ ] Signing and verifying a `WireEnvelope` takes `< 80 µs` per operation and rejects any modified envelope header, timestamp, or payload byte.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test Ed25519 generation, X25519 shared secret agreement between Alice and Bob, and `did:key` multibase test vectors.
- **Integration Tests:**
  - Sign `umcf_proto::WireEnvelope` with `NodeKeypair`, resolve `SenderDid` to public key, and verify signature.
- **Simulation / Fault-Injection Tests:**
  - Attempt signature replay with mismatched `SenderDid` or mutated `MessageFlags` and verify `E2001_INVALID_SIGNATURE` error.
- **Hardware-Required Tests:**
  - Not Required (uses OS CSPRNG via `getrandom`).
- **Manually Verified Tests:**
  - Inspect generated `DidDocument` JSON structure against W3C DID Core 1.0 specification.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S2.P1.SP1`:

- `artifacts/evidence/stage-02/S2_P1_SP1_evidence.json`
- `artifacts/evidence/stage-02/did_document_sample.json`
- Standard Evidence Schema (`artifacts/evidence/stage-02/S2_P1_SP1_evidence.json`):
  ```json
  {
    "subphase": "S2.P1.SP1",
    "title": "Ed25519/X25519 Node Identity & DID Generation",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S2.P1.SP1`.
2. **`TODO.md` Update:** Flip `- [ ] S2.P1.SP1` to `- [x] S2.P1.SP1` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 2 — Phase 1` summarizing the concrete deliverables and metrics of `S2.P1.SP1`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S2.P1.SP1` changes (`git reset --hard` to the `S1.P3.SP3` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S2.P1.SP1` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-02/S2_P1_SP1_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
