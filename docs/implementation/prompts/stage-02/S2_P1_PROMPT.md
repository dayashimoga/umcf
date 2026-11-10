# Executable Phase Prompt — `S2.P1`: Cryptographic Device Identity & Secure Key Storage

> **Stage:** Stage 2 — Rust Core, Identity, Enrollment, Device Registry and Cluster Management (`STAGE_02_PROMPT.md`)  
> **Phase:** `S2.P1` — Cryptographic Device Identity & Secure Key Storage  
> **Subphases:** `S2.P1.SP1`, `S2.P1.SP2`, `S2.P1.SP3`  
> **Target Environment:** Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are the Lead Systems Architect executing Phase **`S2.P1` (Cryptographic Device Identity & Secure Key Storage)** of Stage 2 in the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Phase Objective:** Implement Ed25519 signing keys, X25519 Diffie-Hellman key agreement, `did:key` / `did:peer` identifiers, cross-platform OS Keychain/Keystore integration with Argon2id + XChaCha20-Poly1305 encrypted local vault fallback, and mutual attestation certificates.
- **Execution Discipline:** Execute `S2.P1.SP1`, `S2.P1.SP2`, and `S2.P1.SP3` sequentially. Do not begin a subsequent subphase until the current subphase's Completion Gate and Evidence JSON are verified.

---

## 2. Prerequisites & Dependencies

- **Upstream Dependency:** `Stage 1` must be 100% complete and certified.
- **Target Crates / Services:** `crates/umcf-identity, crates/umcf-enrollment, crates/umcf-registry, services/cf_control_plane, services/cloudrun_fallback`
- **Quota Enforcement:** `scripts/podman-quota-check.sh` must pass before and after each subphase (`< 29.0 GiB`).

---

## 3. Implementation Tasks & Subphase Execution Sequence

Execute the three constituent subphases in strict sequential order:

### 3.1 Subphase `S2.P1.SP1`: Ed25519/X25519 Node Identity & DID Generation
- **Prompt File:** [`S2_P1_SP1.md`](S2_P1_SP1.md)
- **Scope:** Create `crates/umcf-identity` with hardware-backed CSPRNG Ed25519 keypair generation, deterministic X25519 birational map derivation, and W3C `did:key` + `did:peer` document generation/resolution.
- **Key Deliverables:**
  - Create `crates/umcf-identity/src/keys.rs` implementing `NodeKeypair` (using `ed25519-dalek` v2 and `x25519-dalek` with `zeroize::ZeroizeOnDrop`), signing/verification helpers, and domain-separated signatures (`UMCF-SIG-v1:`).
  - Implement `crates/umcf-identity/src/did.rs` generating multicodec base58btc `did:key:z6Mk...` identifiers and full W3C `DidDocument` structs containing `Ed25519VerificationKey2020` and `X25519KeyAgreementKey2020` verification methods.
  - Implement `WireEnvelope` signing and verification traits in `crates/umcf-identity/src/envelope_auth.rs` binding `SenderDid` to the Ed25519 public key.

### 3.2 Subphase `S2.P1.SP2`: Platform Keychain/Keystore & Encrypted Local Vault
- **Prompt File:** [`S2_P1_SP2.md`](S2_P1_SP2.md)
- **Scope:** Implement the `KeyStorageBackend` trait supporting native OS secret stores (Linux SecretService/Keyutils, Windows DPAPI/Credential Manager, macOS/iOS Keychain, Android Keystore) and a portable Argon2id + XChaCha20-Poly1305 encrypted file vault (`LocalEncryptedVault`).
- **Key Deliverables:**
  - Implement `crates/umcf-identity/src/storage.rs` defining async trait `KeyStorageBackend` (`store_identity`, `load_identity`, `rotate_identity`, `revoke_identity`).
  - Implement `crates/umcf-identity/src/vault.rs` providing `LocalEncryptedVault` using Argon2id (`m=64 MiB, t=3, p=4`) key derivation, random 192-bit nonces, XChaCha20-Poly1305 AEAD encryption, and atomic file replacement (`0600` permissions on Unix).
  - Implement `crates/umcf-identity/src/os_keystore.rs` wrapping platform credential stores with automatic fallback to `LocalEncryptedVault` in headless container environments.

### 3.3 Subphase `S2.P1.SP3`: Mutual Attestation & Signed Identity Certificates
- **Prompt File:** [`S2_P1_SP3.md`](S2_P1_SP3.md)
- **Scope:** Implement cluster-scoped X.509v3 (for QUIC mTLS) and compact CBOR/JSON Cluster Membership Certificates (`NodeCertificate`), ephemeral challenge-response mutual attestation, and Certificate Revocation Lists (`ClusterCrl`).
- **Key Deliverables:**
  - Implement `crates/umcf-identity/src/cert.rs` generating self-signed root Cluster CA certificates and leaf `NodeCertificate` (containing custom X.509 SAN extension with node `did:key`, `ClusterId`, assigned `NodeRole`, and expiration timestamp) using `rcgen` and `rustls-pki-types`.
  - Implement `crates/umcf-identity/src/attestation.rs` defining the 2-RTT `MutualAttestationProtocol` (`AttestationChallenge` with 32-byte nonce -> `AttestationProof` binding TLS exporter keying material + hardware fingerprint + Ed25519 signature).
  - Implement `crates/umcf-identity/src/crl.rs` for signed `ClusterCrl` distribution and O(1) revoked certificate serial/DID lookup.

---

## 4. Acceptance Criteria (Phase-Level Measurable Gates)

- [ ] All acceptance criteria in `S2_P1_SP1.md`, `S2_P1_SP2.md`, and `S2_P1_SP3.md` are satisfied.
- [ ] Cross-subphase integration between `S2.P1.SP1`, `S2.P1.SP2`, and `S2.P1.SP3` compiles cleanly with zero warnings (`cargo clippy --workspace --all-targets -- -D warnings`).
- [ ] Line coverage across crates modified in `S2.P1` is `>= 85.0%`.
- [ ] Zero `todo!()`, `unimplemented!()`, or placeholder stubs exist in the codebase (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Podman storage utilization remains strictly `< 29.0 GiB`.

---

## 5. Required Tests (Classified Rollup)

- **Unit Tests:** All unit test suites across `S2.P1.SP1`, `S2.P1.SP2`, and `S2.P1.SP3` pass via `cargo nextest run`.
- **Integration Tests:** End-to-end phase integration tests verifying `Ed25519/X25519 Node Identity & DID Generation` + `Platform Keychain/Keystore & Encrypted Local Vault` + `Mutual Attestation & Signed Identity Certificates` interoperation.
- **Simulation Tests:** Fault-injection, corrupted input, and concurrency stress tests defined in `S2.P1.SP1..SP3`.
- **Hardware-Required Tests:** Executed on target hardware or recorded in hardware execution manifests where containerized hardware passthrough is unavailable.
- **Manually Verified Tests:** Developer visual/CLI inspection checklists completed for `S2.P1.SP1..SP3`.

---

## 6. Required Evidence & Artifacts

- `artifacts/evidence/stage-02/S2_P1_SP1_evidence.json`
- `artifacts/evidence/stage-02/S2_P1_SP2_evidence.json`
- `artifacts/evidence/stage-02/S2_P1_SP3_evidence.json`
- `artifacts/evidence/stage-02/S2_P1_PHASE_SUMMARY.json` aggregating all 3 subphase evidence bundles and phase-level coverage metrics.

---

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates

- Verify `S2.P1.SP1`, `S2.P1.SP2`, and `S2.P1.SP3` are all checked `[x]` in `TODO.md`.
- Consolidate `S2.P1` capabilities into `CHANGELOG.md` and module-level architecture docs.

---

## 8. Rollback Criteria & Completion Gate

- **Rollback Criteria:** If phase integration tests fail or Podman disk quota exceeds `29.0 GiB` after cache pruning, roll back to the start of `S2.P1` (`v0.2.0-certified`) and isolate the failing subphase.
- **Completion Gate:** Tag repository checkpoint `v0.2.1-certified` once `S2_P1_PHASE_SUMMARY.json` reports `"status": "PASS"`.
