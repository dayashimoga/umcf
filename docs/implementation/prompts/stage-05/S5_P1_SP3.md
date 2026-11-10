# Subphase S5.P1.SP3 Execution Prompt: Model Integrity Verification, Licensing & Access Control

## 1. Role & Context
You are the AI Security & Governance Engineer for UMCF. In Subphase `S5.P1.SP3`, you will implement cryptographic model provenance verification, pre-load tensor integrity auditing, license compliance gates, and cluster-wide role-based access control (RBAC) for AI models in `crates/umcf-gguf/src/security.rs` and `src/license.rs`.

## 2. Prerequisites & Dependencies
- `S5.P1.SP1` (`ContentAddressedModelStore`, `ModelManifest`) and `S5.P1.SP2` (`GgufParser`).
- `crates/umcf-crypto` (Ed25519 signature verification, X.509/DID peer identity certificates, capability tokens).

## 3. Implementation Tasks
1. **Cryptographic Provenance & Pre-Load Verification (`crates/umcf-gguf/src/security.rs`)**:
   - Define `SignedModelAttestation`:
     ```rust
     pub struct SignedModelAttestation {
         pub model_id: ModelId,
         pub header_blake3: [u8; 32],
         pub tensor_table_blake3: [u8; 32],
         pub license_id: String,
         pub license_text_blake3: [u8; 32],
         pub signer_public_key: Ed25519PublicKey,
         pub signature: Ed25519Signature,
     }
     ```
   - Implement `ModelIntegrityVerifier`:
     - **Fast Header & Sampled Verification (`VerifyMode::FastPreFlight`)**: Always verifies the GGUF header + tensor offset table BLAKE3 hash and checks that every tensor's `[offset, offset + size]` lies strictly within the file bounds and does not overlap metadata before returning a `VerifiedModelGuard`.
     - **Full Scrub (`VerifyMode::FullCryptographicScrub`)**: Verifies all 4 MiB chunk hashes against `ModelManifest` in parallel andquarantines any model whose on-disk bytes mutated since ingestion.
2. **Model Licensing & EULA Gate (`crates/umcf-gguf/src/license.rs`)**:
   - Parse embedded GGUF metadata `general.license`, `general.license.name`, `general.license.link` alongside external `SignedModelAttestation`.
   - Implement `LicensePolicyEngine`:
     - Classifies licenses (`OpenPermissive` e.g. Apache-2.0/MIT, `CommunityRestricted` e.g. Llama-3/Gemma terms requiring explicit user acceptance, `CommercialProhibited` blocking commercial marketplace execution in Stage 7).
     - Stores cryptographically signed user/admin EULA acceptance records (`LicenseAcceptanceReceipt { model_id, license_hash, accepted_by_node, timestamp_ms, signature }`).
3. **Model Access Control (`ModelAccessPolicy`)**:
   - Enforce three visibility scopes per model:
     - `Visibility::PrivateLocalOnly` (never advertised in P2P catalog, never sharded to remote peers).
     - `Visibility::TrustedClusterPeers { allowed_roles: Vec<ClusterRole>, allowed_node_ids: Vec<NodeId> }`.
     - `Visibility::PublicShared` (available for cluster-wide inference and shard distribution).
   - Integrate policy check into both `ResumableShardDownloader` (server-side chunk handler) and `InferenceCoordinator` before dispatching layers.
4. **UI Integration**:
   - Flutter & Svelte 5: License acceptance dialog required before first load of restricted community models; visibility & RBAC selector (`Private Local`, `Trusted Devices`, `Cluster Shared`) with audit log viewer.

## 4. Acceptance Criteria
- >= 95% line/branch coverage in `security.rs` and `license.rs`; 100% test pass rate.
- `VerifiedModelGuard` is a Rust capability type with private constructors—`umcf-llama-sys` in `S5.P2` can *only* accept a `&VerifiedModelGuard`, making it a compile-time error to load an unverified GGUF path.
- Any modification to a single byte of an on-disk GGUF file is detected and moves the model to `Quarantined` status with a structured security alert event.
- Models marked `Visibility::PrivateLocalOnly` return `404 Not Found` (zero metadata leakage) to remote peer catalog or shard queries.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_ed25519_model_attestation_sign_and_verify`
  - `test_overlapping_or_out_of_bounds_tensor_offsets_rejected`
  - `test_license_policy_blocks_unaccepted_restricted_model`
  - `test_private_local_only_policy_denies_remote_peer_access`
- **`[INT]`**:
  - End-to-end test where a peer attempts to pull chunks of a `PrivateLocalOnly` model and a `TrustedClusterPeers` model without a valid capability token, verifying rejection and audit log emission.
  - On-disk silent corruption test (`dd` flip 1 byte in tensor data) verifying automatic quarantine on `FullCryptographicScrub`.
- **`[SIM]`**:
  - Simulated Sybil peer attempting to poison a swarm with a forged `SignedModelAttestation`.
- **`[HW]`**:
  - Measure `FastPreFlight` verification latency on real mobile and desktop storage (< 5 ms target).
- **`[MANUAL]`**:
  - Trigger license acceptance modal in Flutter/Svelte UI, accept terms, and inspect the signed receipt in the Security Audit tab.

## 6. Required Evidence & Artifacts
- `artifacts/stage-05/p1/sp3_security_audit_test_log.json`
- `artifacts/stage-05/p1/sp3_quarantine_recovery_trace.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document `VerifiedModelGuard` compile-time safety pattern, license classification rules, and RBAC visibility modes in `docs/architecture/MODEL_REGISTRY.md` (Section 3).
- Mark `S5.P1.SP3` and Phase `S5.P1` complete in `TODO.md`; update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If any code path allows passing a raw `&Path` to model loading bypassing `VerifiedModelGuard`, or if `PrivateLocalOnly` model hashes appear in P2P gossip messages.
- **Completion Gate**: Phase `S5.P1` completion gate passed (`S5_P1_SP1`..`SP3` green, >= 95% coverage). Proceed to Phase `S5.P2`.
