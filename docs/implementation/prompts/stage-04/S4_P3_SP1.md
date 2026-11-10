# Executable Subphase Prompt — `S4.P3.SP1`: Isolated Worker Execution Sandbox & Artifact Staging

> **Stage:** Stage 4 — Distributed Scheduler, Resource Management, Workload Execution and Recovery (`STAGE_04_PROMPT.md`)  
> **Phase:** Phase `S4.P3` — Sandboxed Workload Execution, Checkpointing & Fault Recovery (`S4_P3_PROMPT.md`)  
> **Subphase:** `S4.P3.SP1` — Isolated Worker Execution Sandbox & Artifact Staging  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S4.P3.SP1` (Isolated Worker Execution Sandbox & Artifact Staging)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement `crates/umcf-worker/src/sandbox.rs` executing untrusted/portable compute tasks inside a capability-restricted Wasmtime (`wasm32-wasip1`) sandbox or isolated child process with an ephemeral per-task scratch directory and BLAKE3-verified input/output artifact staging.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S4.P2.SP3` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S4.P2.SP3` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Create `crates/umcf-worker` and implement `WasmSandboxRunner` using `wasmtime` with fuel/epoch interruption (wired to `InstantPreemptor`), bounded linear memory (wired to `QuotaEnforcer`), disabled ambient host filesystem/network access, and explicit preopened scratch dir (`/task/in`, `/task/out`, `/task/checkpoint`).
2. Implement `NativeKernelRunner` for verified built-in native engines (Stage 5 `llama.cpp`, Stage 6 `wgpu`/media) gated strictly by cryptographic engine registry allow-list.
3. Implement `ArtifactStager` streaming input blobs over QUIC, verifying BLAKE3 root hashes before task start, and atomically sealing output artifacts with BLAKE3 digests + execution receipts.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S4.P3.SP1` complete:

- [ ] Sandboxed Wasm tasks cannot read or write any host file outside their ephemeral `/task/*` scratch directory (`100%` path traversal & symlink escape prevention).
- [ ] Corrupted input artifacts (BLAKE3 mismatch) are rejected before task execution begins (`ErrorCode::E4201_ARTIFACT_HASH_MISMATCH`).
- [ ] Ephemeral scratch directories are securely wiped and reclaimed upon task completion or cancellation.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test `ArtifactStager` BLAKE3 verification, quota-bounded scratch directory cleanup, and Wasm fuel exhaustion termination.
- **Integration Tests:**
  - Compile a real Rust map-reduce / matrix-multiply task to `wasm32-wasip1`, stage input tensor via `ArtifactStager`, execute inside `WasmSandboxRunner`, and verify output BLAKE3 hash.
- **Simulation / Fault-Injection Tests:**
  - Execute a malicious Wasm module attempting `../../etc/passwd` traversal, symlink creation, infinite loop, and `4 GiB` allocation bomb; verify all 4 are trapped cleanly within `< 20 ms`.
- **Hardware-Required Tests:**
  - Not Required.
- **Manually Verified Tests:**
  - Review sandbox security policy and resource usage receipt.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S4.P3.SP1`:

- `artifacts/evidence/stage-04/S4_P3_SP1_evidence.json`
- `artifacts/evidence/stage-04/sandbox_escape_audit.json`
- Standard Evidence Schema (`artifacts/evidence/stage-04/S4_P3_SP1_evidence.json`):
  ```json
  {
    "subphase": "S4.P3.SP1",
    "title": "Isolated Worker Execution Sandbox & Artifact Staging",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S4.P3.SP1`.
2. **`TODO.md` Update:** Flip `- [ ] S4.P3.SP1` to `- [x] S4.P3.SP1` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 4 — Phase 3` summarizing the concrete deliverables and metrics of `S4.P3.SP1`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S4.P3.SP1` changes (`git reset --hard` to the `S4.P2.SP3` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S4.P3.SP1` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-04/S4_P3_SP1_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
