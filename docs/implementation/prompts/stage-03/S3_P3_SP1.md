# Executable Subphase Prompt — `S3.P3.SP1`: Cross-Platform CPU/RAM/Storage/Battery/Thermal Probing

> **Stage:** Stage 3 — LAN/Remote Networking, Security, Hardware Discovery and Benchmarking (`STAGE_03_PROMPT.md`)  
> **Phase:** Phase `S3.P3` — Real Hardware Capability Discovery & Deterministic Benchmarking (`S3_P3_PROMPT.md`)  
> **Subphase:** `S3.P3.SP1` — Cross-Platform CPU/RAM/Storage/Battery/Thermal Probing  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S3.P3.SP1` (Cross-Platform CPU/RAM/Storage/Battery/Thermal Probing)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement real OS hardware probing in `crates/umcf-hwprobe/src/system_probe.rs` covering CPU topology (P-cores vs E-cores, cache sizes, SIMD instruction sets AVX2/AVX-512/AMX/NEON/SVE2), physical/available RAM, swap, NVMe/UFS storage free space & IOPS class, battery level/charging state, and thermal sensors.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S3.P2.SP3` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S3.P2.SP3` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Create `crates/umcf-hwprobe` and implement `CpuProber` using `raw-cpuid` (x86_64), `/proc/cpuinfo` & `getauxval(AT_HWCAP)` (Linux ARM64), `sysctlbyname` (macOS/iOS), and `GetLogicalProcessorInformationEx` (Windows) to populate `umcf_core::CpuCapability`.
2. Implement `MemoryStorageProber` querying total/available physical RAM, cgroup memory limits (when containerized), and workspace filesystem available bytes.
3. Implement `PowerThermalProber` reading `/sys/class/power_supply/`, `/sys/class/thermal/`, Windows `GetSystemPowerStatus`/WMI, macOS `IOPMCopyPowerSourcesInfo`, and Android/iOS FFI battery/thermal bridge hooks.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S3.P3.SP1` complete:

- [ ] `SystemProber::probe_all()` completes in `< 25 ms` with zero unsafe memory errors and returns a populated `HardwareCapabilityDescriptor`.
- [ ] Correctly detects container cgroup v2 CPU/memory limits when running inside rootless Podman so a constrained container never over-reports host RAM.
- [ ] Accurately reports SIMD flags (`avx2`, `vnni`, `neon`, `fp16`) matching host CPU hardware registers.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test `/proc/cpuinfo`, cgroup v2 `memory.max` / `cpu.max`, and `/sys/class/power_supply` parsers against real fixtures from x86_64 Linux, ARM64 Android, Apple M3, and Windows 11.
- **Integration Tests:**
  - Execute `SystemProber::probe_all()` live inside the rootless Podman container and verify non-zero CPU cores, valid RAM bounds, and cgroup awareness.
- **Simulation / Fault-Injection Tests:**
  - Inject synthetic thermal critical (`95_000` mC) and low battery (`8%` discharging) sysfs fixtures and verify `ThermalState::Critical` mapping.
- **Hardware-Required Tests:**
  - Executes real CPUID and OS sysfs/kernel queries on the host CPU inside Podman.
- **Manually Verified Tests:**
  - Inspect JSON dump of `umcf-hwprobe` live system report on host machine.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S3.P3.SP1`:

- `artifacts/evidence/stage-03/S3_P3_SP1_evidence.json`
- `artifacts/evidence/stage-03/live_system_probe_report.json`
- Standard Evidence Schema (`artifacts/evidence/stage-03/S3_P3_SP1_evidence.json`):
  ```json
  {
    "subphase": "S3.P3.SP1",
    "title": "Cross-Platform CPU/RAM/Storage/Battery/Thermal Probing",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S3.P3.SP1`.
2. **`TODO.md` Update:** Flip `- [ ] S3.P3.SP1` to `- [x] S3.P3.SP1` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 3 — Phase 3` summarizing the concrete deliverables and metrics of `S3.P3.SP1`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S3.P3.SP1` changes (`git reset --hard` to the `S3.P2.SP3` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S3.P3.SP1` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-03/S3_P3_SP1_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
