# Executable Phase Prompt — `S3.P3`: Real Hardware Capability Discovery & Deterministic Benchmarking

> **Stage:** Stage 3 — LAN/Remote Networking, Security, Hardware Discovery and Benchmarking (`STAGE_03_PROMPT.md`)  
> **Phase:** `S3.P3` — Real Hardware Capability Discovery & Deterministic Benchmarking  
> **Subphases:** `S3.P3.SP1`, `S3.P3.SP2`, `S3.P3.SP3`  
> **Target Environment:** Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are the Lead Systems Architect executing Phase **`S3.P3` (Real Hardware Capability Discovery & Deterministic Benchmarking)** of Stage 3 in the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Phase Objective:** Implement `crates/umcf-hwprobe` to discover real physical CPU, RAM, storage, battery, thermal, GPU, and NPU capabilities across platforms and execute deterministic micro-benchmarks producing cryptographically signed hardware capability attestations.
- **Execution Discipline:** Execute `S3.P3.SP1`, `S3.P3.SP2`, and `S3.P3.SP3` sequentially. Do not begin a subsequent subphase until the current subphase's Completion Gate and Evidence JSON are verified.

---

## 2. Prerequisites & Dependencies

- **Upstream Dependency:** `S3.P2` must be 100% complete and certified.
- **Target Crates / Services:** `crates/umcf-net, crates/umcf-hwprobe`
- **Quota Enforcement:** `scripts/podman-quota-check.sh` must pass before and after each subphase (`< 29.0 GiB`).

---

## 3. Implementation Tasks & Subphase Execution Sequence

Execute the three constituent subphases in strict sequential order:

### 3.1 Subphase `S3.P3.SP1`: Cross-Platform CPU/RAM/Storage/Battery/Thermal Probing
- **Prompt File:** [`S3_P3_SP1.md`](S3_P3_SP1.md)
- **Scope:** Implement real OS hardware probing in `crates/umcf-hwprobe/src/system_probe.rs` covering CPU topology (P-cores vs E-cores, cache sizes, SIMD instruction sets AVX2/AVX-512/AMX/NEON/SVE2), physical/available RAM, swap, NVMe/UFS storage free space & IOPS class, battery level/charging state, and thermal sensors.
- **Key Deliverables:**
  - Create `crates/umcf-hwprobe` and implement `CpuProber` using `raw-cpuid` (x86_64), `/proc/cpuinfo` & `getauxval(AT_HWCAP)` (Linux ARM64), `sysctlbyname` (macOS/iOS), and `GetLogicalProcessorInformationEx` (Windows) to populate `umcf_core::CpuCapability`.
  - Implement `MemoryStorageProber` querying total/available physical RAM, cgroup memory limits (when containerized), and workspace filesystem available bytes.
  - Implement `PowerThermalProber` reading `/sys/class/power_supply/`, `/sys/class/thermal/`, Windows `GetSystemPowerStatus`/WMI, macOS `IOPMCopyPowerSourcesInfo`, and Android/iOS FFI battery/thermal bridge hooks.

### 3.2 Subphase `S3.P3.SP2`: GPU & NPU Compute Enumeration (Vulkan/Metal/DX12/OpenCL/NNAPI/CoreML)
- **Prompt File:** [`S3_P3_SP2.md`](S3_P3_SP2.md)
- **Scope:** Implement `crates/umcf-hwprobe/src/accelerator_probe.rs` enumerating discrete and integrated GPUs and neural accelerators (NPUs/TPUs) via `wgpu`/Vulkan/Metal/DX12 adapter introspection, CUDA/ROCm dynamic runtime detection, and Android NNAPI / Apple CoreML / Qualcomm QNN / Intel OpenVINO device descriptors.
- **Key Deliverables:**
  - Implement `GpuEnumerator` using `wgpu::Instance::enumerate_adapters` (`Backends::VULKAN | Backends::METAL | Backends::DX12 | Backends::GL`) extracting vendor/device PCI IDs, driver version, `DeviceType` (`DiscreteGpu`, `IntegratedGpu`, `VirtualGpu`, `Cpu`), max compute workgroup sizes, shader-f16 support, and dedicated/shared VRAM budgets.
  - Implement dynamic `dlopen`/`LoadLibrary` probers (`CudaRocmProber`, `NpuEnumerator`) that safely query `libcuda.so`/`nvcuda.dll`, `libamdhip64.so`, Android NNAPI (`ANeuralNetworks_getDeviceCount`), and Apple CoreML/ANE availability without hard link-time dependencies.
  - Provide graceful CPU fallback (`ComputeBackend::CpuSimd`) when running on a headless machine without a physical GPU.

### 3.3 Subphase `S3.P3.SP3`: Real Compute/Memory/Network Micro-Benchmarks & Signed Attestation
- **Prompt File:** [`S3_P3_SP3.md`](S3_P3_SP3.md)
- **Scope:** Implement deterministic, time-bounded (`<= 500 ms` total) real micro-benchmarks in `crates/umcf-hwprobe/src/benchmark.rs` measuring actual CPU GEMM GFLOPS (FP32/INT8), memory copy bandwidth (GB/s), storage sequential write/read MB/s, GPU shader dispatch GFLOPS, and peer RTT/bandwidth, wrapped in an Ed25519-signed `SignedHardwareAttestation`.
- **Key Deliverables:**
  - Implement `ComputeMicroBench` executing a deterministic `256x256` tiled matrix multiplication (FP32 & quantized INT8 dot-product) and STREAM Triad memory bandwidth kernel (`16 MiB` buffer) to measure real effective GFLOPS and GB/s.
  - Implement `NetworkMicroBench` measuring QUIC RTT (`min`, `avg`, `p95`, jitter) and burst bandwidth between peer nodes using a `256 KiB` probe train.
  - Implement `SignedHardwareAttestation` binding `NodeDid`, `HlcTimestamp`, `HardwareCapabilityDescriptor`, measured `BenchmarkScores`, and deterministic execution seed check, signed by `NodeKeypair`, plus `scripts/verify-stage-03.sh`.

---

## 4. Acceptance Criteria (Phase-Level Measurable Gates)

- [ ] All acceptance criteria in `S3_P3_SP1.md`, `S3_P3_SP2.md`, and `S3_P3_SP3.md` are satisfied.
- [ ] Cross-subphase integration between `S3.P3.SP1`, `S3.P3.SP2`, and `S3.P3.SP3` compiles cleanly with zero warnings (`cargo clippy --workspace --all-targets -- -D warnings`).
- [ ] Line coverage across crates modified in `S3.P3` is `>= 85.0%`.
- [ ] Zero `todo!()`, `unimplemented!()`, or placeholder stubs exist in the codebase (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Podman storage utilization remains strictly `< 29.0 GiB`.

---

## 5. Required Tests (Classified Rollup)

- **Unit Tests:** All unit test suites across `S3.P3.SP1`, `S3.P3.SP2`, and `S3.P3.SP3` pass via `cargo nextest run`.
- **Integration Tests:** End-to-end phase integration tests verifying `Cross-Platform CPU/RAM/Storage/Battery/Thermal Probing` + `GPU & NPU Compute Enumeration (Vulkan/Metal/DX12/OpenCL/NNAPI/CoreML)` + `Real Compute/Memory/Network Micro-Benchmarks & Signed Attestation` interoperation.
- **Simulation Tests:** Fault-injection, corrupted input, and concurrency stress tests defined in `S3.P3.SP1..SP3`.
- **Hardware-Required Tests:** Executed on target hardware or recorded in hardware execution manifests where containerized hardware passthrough is unavailable.
- **Manually Verified Tests:** Developer visual/CLI inspection checklists completed for `S3.P3.SP1..SP3`.

---

## 6. Required Evidence & Artifacts

- `artifacts/evidence/stage-03/S3_P3_SP1_evidence.json`
- `artifacts/evidence/stage-03/S3_P3_SP2_evidence.json`
- `artifacts/evidence/stage-03/S3_P3_SP3_evidence.json`
- `artifacts/evidence/stage-03/S3_P3_PHASE_SUMMARY.json` aggregating all 3 subphase evidence bundles and phase-level coverage metrics.

---

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates

- Verify `S3.P3.SP1`, `S3.P3.SP2`, and `S3.P3.SP3` are all checked `[x]` in `TODO.md`.
- Consolidate `S3.P3` capabilities into `CHANGELOG.md` and module-level architecture docs.

---

## 8. Rollback Criteria & Completion Gate

- **Rollback Criteria:** If phase integration tests fail or Podman disk quota exceeds `29.0 GiB` after cache pruning, roll back to the start of `S3.P3` (`v0.3.2-certified`) and isolate the failing subphase.
- **Completion Gate:** Tag repository checkpoint `v0.3.3-certified` once `S3_P3_PHASE_SUMMARY.json` reports `"status": "PASS"`.
