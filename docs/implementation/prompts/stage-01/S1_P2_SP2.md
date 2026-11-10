# Executable Subphase Prompt — `S1.P2.SP2`: `umcf-core` Hardware & Resource Descriptors

> **Stage:** Stage 1 — Engineering Foundation, Repository and Development Infrastructure (`STAGE_01_PROMPT.md`)  
> **Phase:** Phase `S1.P2` — Versioned Wire Protocol, Core Domain Types & Capability Schemas (`S1_P2_PROMPT.md`)  
> **Subphase:** `S1.P2.SP2` — `umcf-core` Hardware & Resource Descriptors  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S1.P2.SP2` (`umcf-core` Hardware & Resource Descriptors)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Define the canonical domain types in `crates/umcf-core` for node identities, device classes, CPU/GPU/NPU/RAM/storage/network/battery/thermal capability descriptors, and resource units.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S1.P2.SP1` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S1.P2.SP1` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Implement `crates/umcf-core/src/types.rs` defining `NodeId`, `ClusterId`, `JobId`, `TaskId`, `DeviceClass` (`Smartphone`, `Tablet`, `Laptop`, `Desktop`, `Workstation`, `Server`, `BrowserWasm`), and `OperatingSystem` (`Android`, `Ios`, `Windows`, `Linux`, `MacOs`, `Web`).
2. Implement `crates/umcf-core/src/capabilities.rs` defining `HardwareCapabilityDescriptor` containing `CpuCapability` (architecture, physical/performance/efficiency cores, AVX2/AVX512/NEON/SVE flags, base/boost MHz), `MemoryCapability` (total/available RAM bytes, unified memory flag, bandwidth MB/s), `AcceleratorCapability` (`ComputeBackend`: `Vulkan`, `Metal`, `Dx12`, `Cuda`, `Rocm`, `OpenCl`, `Nnapi`, `CoreMl`, `WebGpu`; VRAM bytes, FP32/FP16/INT8 TFLOPS, compute units), `StorageCapability`, `NetworkCapability`, and `PowerThermalState` (battery %, charging state, thermal state `Nominal | Fair | Serious | Critical`, temperature milli-Celsius).
3. Implement `crates/umcf-core/src/units.rs` providing checked arithmetic types `ByteSize(u64)`, `MilliCores(u32)`, `TeraFlopsFp16(u32)`, and JSON Schema export (`schemars::JsonSchema`) for TypeScript/Dart codegen.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S1.P2.SP2` complete:

- [ ] `HardwareCapabilityDescriptor` serializes identically across binary (`bincode`) and canonical JSON (`serde_json`) with exported JSON Schema at `schemas/hardware_capability_v1.schema.json`.
- [ ] Checked unit arithmetic (`ByteSize`, `MilliCores`) prevents integer overflow/underflow on all arithmetic operations.
- [ ] `HardwareCapabilityDescriptor::satisfies(&ResourceRequirement)` accurately evaluates multi-dimensional hardware constraints (RAM, VRAM, backend, thermal state).
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test `HardwareCapabilityDescriptor::satisfies` across 25 device profiles (Android phone, iPhone, M3 MacBook, RTX 4090 Desktop, Wasm Browser).
- **Integration Tests:**
  - Embed `HardwareCapabilityDescriptor` inside `umcf_proto::WireEnvelope` and verify cross-crate codec compatibility and JSON Schema validation.
- **Simulation / Fault-Injection Tests:**
  - Simulate boundary overflows (`u64::MAX` bytes, `0` cores, `ThermalState::Critical`) to verify safe rejection.
- **Hardware-Required Tests:**
  - Not Required.
- **Manually Verified Tests:**
  - Review generated `schemas/hardware_capability_v1.schema.json` for TypeScript/Flutter compatibility.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S1.P2.SP2`:

- `artifacts/evidence/stage-01/S1_P2_SP2_evidence.json`
- `schemas/hardware_capability_v1.schema.json`
- Standard Evidence Schema (`artifacts/evidence/stage-01/S1_P2_SP2_evidence.json`):
  ```json
  {
    "subphase": "S1.P2.SP2",
    "title": "`umcf-core` Hardware & Resource Descriptors",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S1.P2.SP2`.
2. **`TODO.md` Update:** Flip `- [ ] S1.P2.SP2` to `- [x] S1.P2.SP2` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 1 — Phase 2` summarizing the concrete deliverables and metrics of `S1.P2.SP2`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S1.P2.SP2` changes (`git reset --hard` to the `S1.P2.SP1` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S1.P2.SP2` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-01/S1_P2_SP2_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
