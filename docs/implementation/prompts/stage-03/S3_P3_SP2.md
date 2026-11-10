# Executable Subphase Prompt — `S3.P3.SP2`: GPU & NPU Compute Enumeration (Vulkan/Metal/DX12/OpenCL/NNAPI/CoreML)

> **Stage:** Stage 3 — LAN/Remote Networking, Security, Hardware Discovery and Benchmarking (`STAGE_03_PROMPT.md`)  
> **Phase:** Phase `S3.P3` — Real Hardware Capability Discovery & Deterministic Benchmarking (`S3_P3_PROMPT.md`)  
> **Subphase:** `S3.P3.SP2` — GPU & NPU Compute Enumeration (Vulkan/Metal/DX12/OpenCL/NNAPI/CoreML)  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S3.P3.SP2` (GPU & NPU Compute Enumeration (Vulkan/Metal/DX12/OpenCL/NNAPI/CoreML))** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement `crates/umcf-hwprobe/src/accelerator_probe.rs` enumerating discrete and integrated GPUs and neural accelerators (NPUs/TPUs) via `wgpu`/Vulkan/Metal/DX12 adapter introspection, CUDA/ROCm dynamic runtime detection, and Android NNAPI / Apple CoreML / Qualcomm QNN / Intel OpenVINO device descriptors.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S3.P3.SP1` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S3.P3.SP1` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Implement `GpuEnumerator` using `wgpu::Instance::enumerate_adapters` (`Backends::VULKAN | Backends::METAL | Backends::DX12 | Backends::GL`) extracting vendor/device PCI IDs, driver version, `DeviceType` (`DiscreteGpu`, `IntegratedGpu`, `VirtualGpu`, `Cpu`), max compute workgroup sizes, shader-f16 support, and dedicated/shared VRAM budgets.
2. Implement dynamic `dlopen`/`LoadLibrary` probers (`CudaRocmProber`, `NpuEnumerator`) that safely query `libcuda.so`/`nvcuda.dll`, `libamdhip64.so`, Android NNAPI (`ANeuralNetworks_getDeviceCount`), and Apple CoreML/ANE availability without hard link-time dependencies.
3. Provide graceful CPU fallback (`ComputeBackend::CpuSimd`) when running on a headless machine without a physical GPU.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S3.P3.SP2` complete:

- [ ] `AcceleratorProber::enumerate()` never crashes or fails to link when GPU drivers (CUDA/Vulkan/ROCm) are absent; it cleanly returns the available subset plus `ComputeBackend::CpuSimd`.
- [ ] Accurately classifies unified-memory architectures (Apple Silicon, Mobile SoCs, AMD APUs) vs discrete PCIe VRAM GPUs.
- [ ] Completes enumeration across all backends in `< 150 ms`.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test PCI Vendor ID classification (`0x10DE` NVIDIA, `0x1002` AMD, `0x8086` Intel, `0x106B` Apple, `0x5143` Qualcomm) and unified memory heuristics.
- **Integration Tests:**
  - Run `AcceleratorProber::enumerate()` in Podman (verifying Vulkan lavapipe/GPU passthrough or clean `CpuSimd` fallback).
- **Simulation / Fault-Injection Tests:**
  - Simulate corrupted/missing `libcuda.so` symbol table and verify safe `Err` handling with zero segfaults.
- **Hardware-Required Tests:**
  - Host GPU/iGPU via `/dev/dri` or Vulkan/DX12/Metal adapter if present; software Vulkan (`llvmpipe`/`lavapipe`) or CPU fallback in headless container.
- **Manually Verified Tests:**
  - Review enumerated accelerator list and feature flags in probe output.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S3.P3.SP2`:

- `artifacts/evidence/stage-03/S3_P3_SP2_evidence.json`
- `artifacts/evidence/stage-03/accelerator_enumeration_report.json`
- Standard Evidence Schema (`artifacts/evidence/stage-03/S3_P3_SP2_evidence.json`):
  ```json
  {
    "subphase": "S3.P3.SP2",
    "title": "GPU & NPU Compute Enumeration (Vulkan/Metal/DX12/OpenCL/NNAPI/CoreML)",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S3.P3.SP2`.
2. **`TODO.md` Update:** Flip `- [ ] S3.P3.SP2` to `- [x] S3.P3.SP2` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 3 — Phase 3` summarizing the concrete deliverables and metrics of `S3.P3.SP2`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S3.P3.SP2` changes (`git reset --hard` to the `S3.P3.SP1` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S3.P3.SP2` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-03/S3_P3_SP2_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
