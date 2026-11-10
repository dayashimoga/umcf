# Executable Stage Prompt — Stage 2: Rust Core, Identity, Enrollment, Device Registry and Cluster Management

> **Document ID:** `STAGE_02_PROMPT`  
> **Stage Code:** `S2` (`S2.P1.SP1` .. `S2.P3.SP3`)  
> **Master Roadmap Reference:** [`docs/implementation/MASTER_ROADMAP.md`](../MASTER_ROADMAP.md)  
> **Execution Environment:** Rootless Podman (`<= 30 GB` Hard Storage Ceiling)

---

## 1. Role & Context

You are the Chief Systems Architect leading **Stage 2 (Rust Core, Identity, Enrollment, Device Registry and Cluster Management)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Stage Goal:** Build the cryptographic device identity layer (`umcf-identity`), short-lived QR enrollment and fine-grained RBAC/consent engine (`umcf-enrollment`), dynamic Master/Worker/Hybrid election state machine, and the distributed device registry (`umcf-registry`) spanning local autonomous Redb storage, Cloudflare Workers/Durable Objects edge sync, and Google Cloud Run fallback.
- **Primary Crates & Services Owned/Modified:** `crates/umcf-identity`, `crates/umcf-enrollment`, `crates/umcf-registry`, `services/cf_control_plane`, `services/cloudrun_fallback`

---

## 2. Prerequisites & Dependencies

- Stage 1 certified (`STAGE_01_CERTIFICATION.json` present)
- Podman disk usage `< 29.0 GiB`
- Rootless Podman quota enforcer (`scripts/podman-quota-check.sh`) active and reporting `< 29.0 GiB`.

---

## 3. Stage Hierarchy & Executable Prompt Index (`3 Phases × 3 Subphases = 9 Subphases`)

Execute the following phases and subphases in strict sequential order:

### Phase 1 (`S2.P1`): Cryptographic Device Identity & Secure Key Storage
- **Phase Prompt:** [`S2_P1_PROMPT.md`](S2_P1_PROMPT.md)
  1. **`S2.P1.SP1` — Ed25519/X25519 Node Identity & DID Generation:** [`S2_P1_SP1.md`](S2_P1_SP1.md)
  2. **`S2.P1.SP2` — Platform Keychain/Keystore & Encrypted Local Vault:** [`S2_P1_SP2.md`](S2_P1_SP2.md)
  3. **`S2.P1.SP3` — Mutual Attestation & Signed Identity Certificates:** [`S2_P1_SP3.md`](S2_P1_SP3.md)

### Phase 2 (`S2.P2`): QR Enrollment, RBAC & Dynamic Master/Worker Role Engine
- **Phase Prompt:** [`S2_P2_PROMPT.md`](S2_P2_PROMPT.md)
  4. **`S2.P2.SP1` — Short-Lived QR Enrollment Token Generator & Scanner Flow:** [`S2_P2_SP1.md`](S2_P2_SP1.md)
  5. **`S2.P2.SP2` — Fine-Grained RBAC & Resource-Sharing Consent Engine:** [`S2_P2_SP2.md`](S2_P2_SP2.md)
  6. **`S2.P2.SP3` — Dynamic Master/Worker/Hybrid State Machine & Election:** [`S2_P2_SP3.md`](S2_P2_SP3.md)

### Phase 3 (`S2.P3`): Distributed Device Registry & Cluster Coordination
- **Phase Prompt:** [`S2_P3_PROMPT.md`](S2_P3_PROMPT.md)
  7. **`S2.P3.SP1` — Local Autonomous Coordinator & SQLite/Redb State Store:** [`S2_P3_SP1.md`](S2_P3_SP1.md)
  8. **`S2.P3.SP2` — Cloudflare Workers/Durable Objects Control Plane Sync:** [`S2_P3_SP2.md`](S2_P3_SP2.md)
  9. **`S2.P3.SP3` — Google Cloud Run Fallback Control Plane & Split-Brain Reconciliation:** [`S2_P3_SP3.md`](S2_P3_SP3.md)

---

## 4. Implementation Tasks (Stage-Level Orchestration)

1. Execute Phase `S2.P1` (`SP1 -> SP2 -> SP3`) and generate `S2_P1_PHASE_SUMMARY.json`.
2. Execute Phase `S2.P2` (`SP1 -> SP2 -> SP3`) and generate `S2_P2_PHASE_SUMMARY.json`.
3. Execute Phase `S2.P3` (`SP1 -> SP2 -> SP3`) and generate `S2_P3_PHASE_SUMMARY.json`.
4. Run full Stage 2 verification script `scripts/verify-stage-02.sh` to audit all 9 subphase evidence files, SBOMs, coverage reports, and Podman storage usage.

---

## 5. Acceptance Criteria (Stage-Level Measurable Gates)

- [ ] All 9 subphases (`S2.P1.SP1` through `S2.P3.SP3`) pass their individual acceptance criteria.
- [ ] Workspace compiles with `0` warnings and `0` errors across all Stage 2 targets.
- [ ] Combined line coverage across Stage 2 crates (`crates/umcf-identity`, `crates/umcf-enrollment`, `crates/umcf-registry`, `services/cf_control_plane`, `services/cloudrun_fallback`) is `>= 85.0%`.
- [ ] `scripts/check-no-placeholders.sh` confirms `0` `todo!()`, `unimplemented!()`, or mock-only stubs.
- [ ] `scripts/podman-quota-check.sh` confirms total container, volume, and `target/` usage is `<= 29.0 GiB` (`< 30 GB` hard ceiling).

---

## 6. Required Tests (Stage Rollup)

- **Unit Tests:** 100% pass rate across all unit tests in `S2.P1` – `S2.P3`.
- **Integration Tests:** Cross-phase integration tests verifying end-to-end data/control flow across `crates/umcf-identity`, `crates/umcf-enrollment`, `crates/umcf-registry`, `services/cf_control_plane`, `services/cloudrun_fallback`.
- **Simulation Tests:** Chaos, fuzzing (`proptest`), and fault-injection suites for all Stage 2 subsystems.
- **Hardware-Required Tests:** Executed on host hardware or documented with reproducible hardware execution manifests.
- **Manually Verified Tests:** All 9 subphase manual verification checklists signed off.

---

## 7. Required Evidence & Artifacts

- 9 Subphase Evidence Files: `artifacts/evidence/stage-02/S2_P{1..3}_SP{1..3}_evidence.json`
- 3 Phase Summary Files: `artifacts/evidence/stage-02/S2_P{1..3}_PHASE_SUMMARY.json`
- Stage Certification Manifest: `artifacts/evidence/stage-02/STAGE_02_CERTIFICATION.json`
- Stage CycloneDX SBOM & Checksums: `artifacts/sbom/stage-02-cyclonedx.json` and `artifacts/manifests/stage-02-sha256sums.txt`

---

## 8. Documentation, `TODO.md` & `CHANGELOG.md` Updates

- All 9 subphases (`S2.P1.SP1` .. `S2.P3.SP3`) marked `[x]` in `TODO.md`.
- Stage 2 release section finalized in `CHANGELOG.md` under `[0.2.0-stage2]`.

---

## 9. Rollback Criteria & Completion Gate

- **Rollback Criteria:** Any regression in Stage `1..2`, any security/cryptographic invariant failure, or any breach of the `30 GB` Podman disk ceiling triggers rollback to the latest certified phase tag.
- **Completion Gate:** Stage 2 is certified complete when `scripts/verify-stage-02.sh` exits `0` and produces `STAGE_02_CERTIFICATION.json`. Tag repository `v0.2.0-certified`.
