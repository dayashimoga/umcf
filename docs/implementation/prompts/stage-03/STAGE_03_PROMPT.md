# Executable Stage Prompt — Stage 3: LAN/Remote Networking, Security, Hardware Discovery and Benchmarking

> **Document ID:** `STAGE_03_PROMPT`  
> **Stage Code:** `S3` (`S3.P1.SP1` .. `S3.P3.SP3`)  
> **Master Roadmap Reference:** [`docs/implementation/MASTER_ROADMAP.md`](../MASTER_ROADMAP.md)  
> **Execution Environment:** Rootless Podman (`<= 30 GB` Hard Storage Ceiling)

---

## 1. Role & Context

You are the Chief Systems Architect leading **Stage 3 (LAN/Remote Networking, Security, Hardware Discovery and Benchmarking)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Stage Goal:** Implement the LAN/WAN peer-to-peer networking stack (`umcf-net`) featuring zero-conf mDNS/DNS-SD + signed UDP broadcast discovery, Quinn/QUIC multiplexed mTLS transport, QoS traffic isolation, STUN/ICE UDP hole punching, encrypted DERP/TURN relay fallback, and QUIC connection migration, alongside real cross-platform hardware capability probing and deterministic signed micro-benchmarking (`umcf-hwprobe`).
- **Primary Crates & Services Owned/Modified:** `crates/umcf-net`, `crates/umcf-hwprobe`

---

## 2. Prerequisites & Dependencies

- Stage 2 certified (`STAGE_02_CERTIFICATION.json` present)
- Podman disk usage `< 29.0 GiB`
- Rootless Podman quota enforcer (`scripts/podman-quota-check.sh`) active and reporting `< 29.0 GiB`.

---

## 3. Stage Hierarchy & Executable Prompt Index (`3 Phases × 3 Subphases = 9 Subphases`)

Execute the following phases and subphases in strict sequential order:

### Phase 1 (`S3.P1`): LAN Discovery & Direct P2P QUIC Data Plane
- **Phase Prompt:** [`S3_P1_PROMPT.md`](S3_P1_PROMPT.md)
  1. **`S3.P1.SP1` — Zero-Conf mDNS/DNS-SD & Signed UDP Broadcast Discovery:** [`S3_P1_SP1.md`](S3_P1_SP1.md)
  2. **`S3.P1.SP2` — Quinn/QUIC Multiplexed Data-Plane Transport with mTLS:** [`S3_P1_SP2.md`](S3_P1_SP2.md)
  3. **`S3.P1.SP3` — Control-Plane vs Data-Plane Traffic Isolation & QoS:** [`S3_P1_SP3.md`](S3_P1_SP3.md)

### Phase 2 (`S3.P2`): NAT Traversal, STUN/ICE Hole Punching & Relay Fallback
- **Phase Prompt:** [`S3_P2_PROMPT.md`](S3_P2_PROMPT.md)
  4. **`S3.P2.SP1` — STUN/ICE Candidate Gathering & UDP Hole Punching:** [`S3_P2_SP1.md`](S3_P2_SP1.md)
  5. **`S3.P2.SP2` — Encrypted TURN/DERP-Style Relay Fallback Protocol:** [`S3_P2_SP2.md`](S3_P2_SP2.md)
  6. **`S3.P2.SP3` — Roaming, Network Interface Handover & Connection Migration:** [`S3_P2_SP3.md`](S3_P2_SP3.md)

### Phase 3 (`S3.P3`): Real Hardware Capability Discovery & Deterministic Benchmarking
- **Phase Prompt:** [`S3_P3_PROMPT.md`](S3_P3_PROMPT.md)
  7. **`S3.P3.SP1` — Cross-Platform CPU/RAM/Storage/Battery/Thermal Probing:** [`S3_P3_SP1.md`](S3_P3_SP1.md)
  8. **`S3.P3.SP2` — GPU & NPU Compute Enumeration (Vulkan/Metal/DX12/OpenCL/NNAPI/CoreML):** [`S3_P3_SP2.md`](S3_P3_SP2.md)
  9. **`S3.P3.SP3` — Real Compute/Memory/Network Micro-Benchmarks & Signed Attestation:** [`S3_P3_SP3.md`](S3_P3_SP3.md)

---

## 4. Implementation Tasks (Stage-Level Orchestration)

1. Execute Phase `S3.P1` (`SP1 -> SP2 -> SP3`) and generate `S3_P1_PHASE_SUMMARY.json`.
2. Execute Phase `S3.P2` (`SP1 -> SP2 -> SP3`) and generate `S3_P2_PHASE_SUMMARY.json`.
3. Execute Phase `S3.P3` (`SP1 -> SP2 -> SP3`) and generate `S3_P3_PHASE_SUMMARY.json`.
4. Run full Stage 3 verification script `scripts/verify-stage-03.sh` to audit all 9 subphase evidence files, SBOMs, coverage reports, and Podman storage usage.

---

## 5. Acceptance Criteria (Stage-Level Measurable Gates)

- [ ] All 9 subphases (`S3.P1.SP1` through `S3.P3.SP3`) pass their individual acceptance criteria.
- [ ] Workspace compiles with `0` warnings and `0` errors across all Stage 3 targets.
- [ ] Combined line coverage across Stage 3 crates (`crates/umcf-net`, `crates/umcf-hwprobe`) is `>= 85.0%`.
- [ ] `scripts/check-no-placeholders.sh` confirms `0` `todo!()`, `unimplemented!()`, or mock-only stubs.
- [ ] `scripts/podman-quota-check.sh` confirms total container, volume, and `target/` usage is `<= 29.0 GiB` (`< 30 GB` hard ceiling).

---

## 6. Required Tests (Stage Rollup)

- **Unit Tests:** 100% pass rate across all unit tests in `S3.P1` – `S3.P3`.
- **Integration Tests:** Cross-phase integration tests verifying end-to-end data/control flow across `crates/umcf-net`, `crates/umcf-hwprobe`.
- **Simulation Tests:** Chaos, fuzzing (`proptest`), and fault-injection suites for all Stage 3 subsystems.
- **Hardware-Required Tests:** Executed on host hardware or documented with reproducible hardware execution manifests.
- **Manually Verified Tests:** All 9 subphase manual verification checklists signed off.

---

## 7. Required Evidence & Artifacts

- 9 Subphase Evidence Files: `artifacts/evidence/stage-03/S3_P{1..3}_SP{1..3}_evidence.json`
- 3 Phase Summary Files: `artifacts/evidence/stage-03/S3_P{1..3}_PHASE_SUMMARY.json`
- Stage Certification Manifest: `artifacts/evidence/stage-03/STAGE_03_CERTIFICATION.json`
- Stage CycloneDX SBOM & Checksums: `artifacts/sbom/stage-03-cyclonedx.json` and `artifacts/manifests/stage-03-sha256sums.txt`

---

## 8. Documentation, `TODO.md` & `CHANGELOG.md` Updates

- All 9 subphases (`S3.P1.SP1` .. `S3.P3.SP3`) marked `[x]` in `TODO.md`.
- Stage 3 release section finalized in `CHANGELOG.md` under `[0.3.0-stage3]`.

---

## 9. Rollback Criteria & Completion Gate

- **Rollback Criteria:** Any regression in Stage `1..3`, any security/cryptographic invariant failure, or any breach of the `30 GB` Podman disk ceiling triggers rollback to the latest certified phase tag.
- **Completion Gate:** Stage 3 is certified complete when `scripts/verify-stage-03.sh` exits `0` and produces `STAGE_03_CERTIFICATION.json`. Tag repository `v0.3.0-certified`.
