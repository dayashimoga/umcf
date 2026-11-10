# Executable Subphase Prompt — `S2.P1.SP2`: Platform Keychain/Keystore & Encrypted Local Vault

> **Stage:** Stage 2 — Rust Core, Identity, Enrollment, Device Registry and Cluster Management (`STAGE_02_PROMPT.md`)  
> **Phase:** Phase `S2.P1` — Cryptographic Device Identity & Secure Key Storage (`S2_P1_PROMPT.md`)  
> **Subphase:** `S2.P1.SP2` — Platform Keychain/Keystore & Encrypted Local Vault  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S2.P1.SP2` (Platform Keychain/Keystore & Encrypted Local Vault)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement the `KeyStorageBackend` trait supporting native OS secret stores (Linux SecretService/Keyutils, Windows DPAPI/Credential Manager, macOS/iOS Keychain, Android Keystore) and a portable Argon2id + XChaCha20-Poly1305 encrypted file vault (`LocalEncryptedVault`).
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S2.P1.SP1` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S2.P1.SP1` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Implement `crates/umcf-identity/src/storage.rs` defining async trait `KeyStorageBackend` (`store_identity`, `load_identity`, `rotate_identity`, `revoke_identity`).
2. Implement `crates/umcf-identity/src/vault.rs` providing `LocalEncryptedVault` using Argon2id (`m=64 MiB, t=3, p=4`) key derivation, random 192-bit nonces, XChaCha20-Poly1305 AEAD encryption, and atomic file replacement (`0600` permissions on Unix).
3. Implement `crates/umcf-identity/src/os_keystore.rs` wrapping platform credential stores with automatic fallback to `LocalEncryptedVault` in headless container environments.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S2.P1.SP2` complete:

- [ ] Private key material is never written to disk in plaintext under any code path.
- [ ] `LocalEncryptedVault` detects 1-bit ciphertext or AEAD tag tampering with `100%` reliability (`ErrorCode::E2004_VAULT_INTEGRITY_FAILURE`).
- [ ] Key rotation preserves prior public key history for historical receipt verification while activating the new signing key immediately.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test `LocalEncryptedVault` create, unlock, wrong passphrase rejection, key rotation, and file permission checks.
- **Integration Tests:**
  - Persist `NodeKeypair` into vault, reload across simulated process restart, and verify `did:key` identity continuity.
- **Simulation / Fault-Injection Tests:**
  - Corrupt random bytes in `.umcf_vault.enc` file across 500 fuzz iterations and confirm zero panics and clean AEAD error return.
- **Hardware-Required Tests:**
  - OS Keychain/Credential Manager on host OS (tested on host or mocked in headless Podman container).
- **Manually Verified Tests:**
  - Verify file permissions (`chmod 0600` / Windows ACL) of generated vault file.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S2.P1.SP2`:

- `artifacts/evidence/stage-02/S2_P1_SP2_evidence.json`
- `artifacts/evidence/stage-02/vault_security_audit.json`
- Standard Evidence Schema (`artifacts/evidence/stage-02/S2_P1_SP2_evidence.json`):
  ```json
  {
    "subphase": "S2.P1.SP2",
    "title": "Platform Keychain/Keystore & Encrypted Local Vault",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S2.P1.SP2`.
2. **`TODO.md` Update:** Flip `- [ ] S2.P1.SP2` to `- [x] S2.P1.SP2` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 2 — Phase 1` summarizing the concrete deliverables and metrics of `S2.P1.SP2`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S2.P1.SP2` changes (`git reset --hard` to the `S2.P1.SP1` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S2.P1.SP2` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-02/S2_P1_SP2_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
