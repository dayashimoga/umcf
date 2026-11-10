# Executable Subphase Prompt — `S3.P3.SP3`: Real Compute/Memory/Network Micro-Benchmarks & Signed Attestation

> **Stage:** Stage 3 — LAN/Remote Networking, Security, Hardware Discovery and Benchmarking (`STAGE_03_PROMPT.md`)  
> **Phase:** Phase `S3.P3` — Real Hardware Capability Discovery & Deterministic Benchmarking (`S3_P3_PROMPT.md`)  
> **Subphase:** `S3.P3.SP3` — Real Compute/Memory/Network Micro-Benchmarks & Signed Attestation  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S3.P3.SP3` (Real Compute/Memory/Network Micro-Benchmarks & Signed Attestation)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement deterministic, time-bounded (`<= 500 ms` total) real micro-benchmarks in `crates/umcf-hwprobe/src/benchmark.rs` measuring actual CPU GEMM GFLOPS (FP32/INT8), memory copy bandwidth (GB/s), storage sequential write/read MB/s, GPU shader dispatch GFLOPS, and peer RTT/bandwidth, wrapped in an Ed25519-signed `SignedHardwareAttestation`.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S3.P3.SP2` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S3.P3.SP2` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Implement `ComputeMicroBench` executing a deterministic `256x256` tiled matrix multiplication (FP32 & quantized INT8 dot-product) and STREAM Triad memory bandwidth kernel (`16 MiB` buffer) to measure real effective GFLOPS and GB/s.
2. Implement `NetworkMicroBench` measuring QUIC RTT (`min`, `avg`, `p95`, jitter) and burst bandwidth between peer nodes using a `256 KiB` probe train.
3. Implement `SignedHardwareAttestation` binding `NodeDid`, `HlcTimestamp`, `HardwareCapabilityDescriptor`, measured `BenchmarkScores`, and deterministic execution seed check, signed by `NodeKeypair`, plus `scripts/verify-stage-03.sh`.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S3.P3.SP3` complete:

- [ ] Full micro-benchmark suite (`CPU GEMM + Memory STREAM + Storage I/O`) completes in `< 500 ms` and uses `<= 32 MiB` peak transient RAM.
- [ ] Measured `BenchmarkScores` are strictly positive, reproducible within `±15%` variance on idle hardware, and attached to `ClusterRegistryState` via Stage 2 CRDT sync.
- [ ] `scripts/verify-stage-03.sh` exits `0`, certifying all 9 Stage 3 subphases (`S3.P1.SP1` .. `S3.P3.SP3`) with Podman disk usage `< 29.0 GiB`.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test FP32/INT8 GEMM reference output correctness against golden matrix product and `SignedHardwareAttestation` signature verification.
- **Integration Tests:**
  - Run full `SystemProber` -> `AcceleratorProber` -> `ComputeMicroBench` -> `SignedHardwareAttestation` -> `ClusterRegistryState` sync across a 2-node QUIC mTLS cluster.
- **Simulation / Fault-Injection Tests:**
  - Simulate a dishonest worker node submitting an inflated `BenchmarkScores` struct with an invalid execution proof or forged signature and verify rejection.
- **Hardware-Required Tests:**
  - Executes real CPU GEMM and memory bandwidth instructions on host processor.
- **Manually Verified Tests:**
  - Inspect `STAGE_03_CERTIFICATION.json`, measured GFLOPS/GB/s numbers, and `TODO.md` / `CHANGELOG.md` updates.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S3.P3.SP3`:

- `artifacts/evidence/stage-03/S3_P3_SP3_evidence.json`
- `artifacts/evidence/stage-03/signed_hardware_attestation_sample.json`
- `artifacts/evidence/stage-03/STAGE_03_CERTIFICATION.json`
- Standard Evidence Schema (`artifacts/evidence/stage-03/S3_P3_SP3_evidence.json`):
  ```json
  {
    "subphase": "S3.P3.SP3",
    "title": "Real Compute/Memory/Network Micro-Benchmarks & Signed Attestation",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S3.P3.SP3`.
2. **`TODO.md` Update:** Flip `- [ ] S3.P3.SP3` to `- [x] S3.P3.SP3` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 3 — Phase 3` summarizing the concrete deliverables and metrics of `S3.P3.SP3`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S3.P3.SP3` changes (`git reset --hard` to the `S3.P3.SP2` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S3.P3.SP3` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-03/S3_P3_SP3_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
