# Subphase S8.P2.SP2 Execution Prompt: Penetration Testing, Sandbox Escape Verification & RBAC Bypass Audit

## 1. Role & Context
You are the Offensive Security & Red-Team Penetration Lead for UMCF. In Subphase `S8.P2.SP2`, you will implement `tests/security-redteam/`, an automated adversarial penetration test suite that actively attempts to break out of UMCF's WASM/Container/GPU sandboxes, traverse host filesystems, forge peer identities, escalate RBAC capability scopes, exfiltrate Private AI Enclave prompts, and inject unauthorized remote desktop inputs.

## 2. Prerequisites & Dependencies
- `S8.P2.SP1` (Fuzz-hardened parsers).
- All Stage 1–7 security boundaries (`umcf-crypto`, `umcf-wasm`, `umcf-gguf`, `umcf-ai`, `umcf-gpu`, `umcf-stream`, `umcf-storage`, `umcf-api-gateway`).

## 3. Implementation Tasks
1. **Sandbox Escape Verification Suite (`tests/security-redteam/src/sandbox_escape.rs`)**:
   - Implement hostile payloads and verify 100% containment:
     - **Vector 1 (WASM WASI Filesystem Traversal & Symlink Escape)**: Hostile `.wasm` module attempts `open("../../../../etc/passwd")`, `open("C:\\Windows\\System32\\cmd.exe")`, and symlink creation pointing outside its ephemeral scratch directory; verify `WASI_ERRNO_NOTCAPABLE` / `EACCES`.
     - **Vector 2 (WASM Ambient Network & Subprocess Escape)**: Hostile `.wasm` module attempts `sock_open`, raw socket syscalls, and `proc_exec`; verify hard capability denial.
     - **Vector 3 (WASM Fuel Bomb, Stack Overflow & Zip/Memory Bomb)**: Hostile `.wasm` executes mutual infinite recursion, `memory.grow` past quota, and infinite loops; verify deterministic termination within <= 50 ms without affecting sibling tasks.
     - **Vector 4 (GPU Shader Cross-Tenant VRAM Read & TDR Hang)**: Hostile WGSL compute shader attempts to read uninitialized storage buffers after a sensitive LLM/render task and executes a nested infinite loop; verify zero-scrubbed VRAM (`0x00000000`) and loop-fuel watchdog preemption (`S6.P1.SP1`).
     - **Vector 5 (GGUF Arbitrary Offset Pointer Read)**: Crafted `.gguf` file with tensor offsets pointing past EOF or negative wrap-around (`u64::MAX - 16`); verify rejection by `ModelIntegrityVerifier` before `VerifiedModelGuard` creation (`S5.P1.SP3`).
2. **Authentication, mTLS & RBAC Bypass Audit (`tests/security-redteam/src/rbac_and_crypto_audit.rs`)**:
   - Implement 10 protocol & API attack vectors:
     - **Vector 6 (Unauthenticated / Self-Signed Peer Connection)**: Rogue node with valid TLS syntax but untrusted/unpaired root CA attempts to join cluster or invoke `ActivationRpcService`; verify immediate mTLS handshake rejection.
     - **Vector 7 (Revoked Node Certificate / CRL Replay)**: Node whose trust certificate was revoked 1 millisecond ago attempts 0-RTT QUIC session resumption; verify session ticket cache invalidation and rejection.
     - **Vector 8 (Horizontal & Vertical RBAC Scope Escalation)**: Guest peer with `Role::GuestObserver` or API key with `scope = ["cluster:read"]` calls every mutating gRPC and HTTP `/api/v1/*` endpoint (`POST /jobs`, `DELETE /models`, `POST /storage/pins`, `POST /stream/input`); verify 100% return `403 Forbidden` / `Status::permission_denied`.
     - **Vector 9 (Private AI Enclave Exfiltration Attempt)**: While a session is in `PrivacyEnclaveMode::StrictLocalOnly`, inject a compromised cluster coordinator recommendation requesting remote layer offloading; verify the local enclave circuit breaker aborts the offload and emits a `CRITICAL` security alert.
     - **Vector 10 (Remote Stream `ViewOnly` Input & Clipboard Hijack)**: Client connected with a `ViewOnly` capability token sends crafted `InputEventPacket` (Ctrl+Alt+Del / shell commands) and `ClipboardSyncEvent` over the QUIC datagram channel; verify drop + peer ban.
     - **Vector 11 (Timing Side-Channel on API Keys & Pairing PINs)**: Verify all secret comparisons in `umcf-crypto` and `umcf-api-gateway` use `subtle::ConstantTimeEq`.

## 4. Acceptance Criteria
- >= 95% coverage of all security guard/policy enforcement paths; 100% pass rate across all 11 red-team vector categories (25+ individual exploit test cases).
- Every blocked attack emits a structured, tamper-evident `SecurityAuditEvent { timestamp_ms, source_node_or_ip, vector_category, severity, action_taken }` to the security audit log.
- Zero false negatives (0 successful escapes or unauthorized actions) and zero daemon crashes under red-team assault.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_constant_time_api_key_and_pairing_pin_comparison`
  - `test_path_canonicalization_blocks_dotdot_and_symlink_escapes`
  - `test_revoked_certificate_purges_0rtt_session_tickets`
- **`[INT]`**:
  - `test_redteam_vectors_1_through_5_sandbox_escape_containment`
  - `test_redteam_vectors_6_through_11_rbac_mtls_and_enclave_containment`
- **`[SIM]`**:
  - Automated endpoint scanner hitting all 45+ HTTP/WebSocket/gRPC methods across 5 privilege tiers (`Unauthenticated`, `Revoked`, `ViewOnlyGuest`, `ComputeWorker`, `ClusterAdmin`) verifying the exact authorization matrix.
- **`[HW]`**:
  - Verify GPU VRAM zero-scrubbing isolation between two consecutive tasks on physical GPU hardware.
- **`[MANUAL]`**:
  - Run the "Security Self-Audit & Red-Team Verification" diagnostic in Svelte 5 `/settings` and inspect the green 25/25 Containment Report and Security Audit Log.

## 6. Required Evidence & Artifacts
- `artifacts/stage-08/p2/sp2_redteam_25_vectors_audit_report.json`
- `artifacts/stage-08/p2/sp2_endpoint_rbac_authorization_matrix.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document the 11 Red-Team threat vectors, defense-in-depth controls, and RBAC authorization matrix in `docs/SECURITY_AUDIT.md` (Section 2).
- Mark `S8.P2.SP2` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If any single red-team vector succeeds in reading unauthorized host files/VRAM, executing an unprivileged RPC, or bypassing `StrictLocalOnly` isolation.
- **Completion Gate**: All 25+ red-team penetration tests green before proceeding to `S8.P2.SP3`.
