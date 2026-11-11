# UMCF Security Audit & Cryptographic Hardening Guide

This document defines the formal security audit reports, parser defense mechanisms, pre-allocation ceilings, and fuzzing verification evidence for the Universal Mobile Compute Fabric (UMCF).

---

## 1. Protocol Parser & FFI Fuzzing Architecture (S8.P2.SP1)

Untrusted network datagrams, model weight files, WGSL shaders, and WASM bytecodes enter UMCF across variable peer networks. To prevent denial-of-service, remote code execution, and kernel panics, all entrypoint parsers are protected by pre-allocation bounds, recursion caps, and continuous fuzzing suites.

### 1.1 Pre-Allocation Length Bounds (`BoundedSliceReader`)
Corrupted or malicious payloads often specify astronomical length prefixes (e.g. $2^{62}$ bytes) designed to trigger process termination via heap allocation failure (`OOM bomb`).
- **Safety Ceiling**: `MAX_ALLOCATION_BYTES = 16 * 1024 * 1024` (16 MiB).
- **Enforcement Rule**: No buffer or vector may be allocated via `Vec::with_capacity(claimed_len)` before verifying that:
  1. `claimed_len <= MAX_ALLOCATION_BYTES`.
  2. `claimed_len <= remaining_unparsed_bytes`.
- Any input violating these bounds is immediately rejected with `FuzzError::AllocationExceeded` with **zero heap memory allocated**.

### 1.2 Recursion Depth Ceilings
Nested JSON objects, GGUF array metadata, and AST shaders can cause stack overflow if unbounded:
- **Maximum Recursion Depth**: `MAX_RECURSION_DEPTH = 32`.
- `check_nesting_depth` parses opening/closing delimiters (`{`, `[`, `(`) and rejects deeper nestings with `FuzzError::RecursionDepthExceeded` before passing payloads to secondary parsers.

### 1.3 The 15 Coverage-Guided Fuzz Targets
| Target ID | Target Name | Primary Subsystem | Invariant Enforced |
|:---|:---|:---|:---|
| `01` | `fuzz_p2p_wire_frame` | `umcf-net` | Rejects corrupt magic, truncated frames, and oversized envelopes. |
| `02` | `fuzz_capability_token` | `umcf-identity` | Validates Ed25519 signatures, DID formats, and token expiration. |
| `03` | `fuzz_wasm_bytecode` | WASM Engine | Validates `\0asm` magic, section sizes, and fuel limits. |
| `04` | `fuzz_scheduler_dag` | `umcf-scheduler` | Detects cycle bombs, bounds node and edge counts <= 10,000. |
| `05` | `fuzz_gguf_header` | `umcf-gguf` | Validates `GGUF` magic, version 2/3, and bounds metadata counts. |
| `06` | `fuzz_llama_tokenizer_grammar` | `umcf-ai` | Enforces incremental UTF-8 parsing, rejects invalid Unicode. |
| `07` | `fuzz_ai_activation_rpc` | `umcf-ai` | Bounds sequence length <= 32,768 and FP16/Q8 tensor sizes. |
| `08` | `fuzz_openai_json_body` | `umcf-ai` | Enforces recursion depth <= 32, schema validation. |
| `09` | `fuzz_wgsl_shader_sandbox` | `umcf-gpu` | AST parsing, loop-fuel watchdog injection, bounds checks. |
| `10` | `fuzz_render_tile_packet` | `umcf-gpu` | Bounds tile dimensions width/height <= 8,192 px. |
| `11` | `fuzz_media_segment_metadata` | `umcf-media` | Enforces GOP index integrity, PTS/DTS continuity. |
| `12` | `fuzz_stream_rtp_fec_and_input` | `umcf-stream` | Reassembles RS FEC, bounds input event packets <= 36 bytes. |
| `13` | `fuzz_storage_dag_and_erasure_shard` | `umcf-storage` | Validates CID, systematic RS(k,m) where $k+m \le 255$. |
| `14` | `fuzz_workload_manifest_yaml_toml` | `umcf-manifest` | Validates YAML/TOML within 16 MiB and recursion <= 32. |
| `15` | `fuzz_resource_receipt_and_market_order` | `umcf-ledger` / `umcf-marketplace` | Validates 104-byte resource vector and order constraints. |

---

## 2. Red-Team Penetration Testing, Sandbox Escape Containment & Endpoint RBAC Audit (S8.P2.SP2)

In Subphase `S8.P2.SP2`, automated adversarial offensive suites (`tests/security-redteam`) execute active penetration attacks against UMCF's compute sandboxes, credential stores, peer negotiation channels, and authorization barriers.

### 2.1 The 11 Hostile Threat Vector Categories & Defense-in-Depth Controls
| Vector ID | Attack Name | Adversarial Payload / Vector | Defense-in-Depth Mechanism | Action Taken & SLA | Containment Status |
|:---|:---|:---|:---|:---|:---|
| `V01` | WASI Filesystem Traversal | `../../../../etc/passwd`, `C:\Windows\System32\cmd.exe`, out-of-scratch symlinks | Path canonicalization verifying canonical target starts with root scratch directory | `WASI_ERRNO_NOTCAPABLE` / `EACCES` (< 1 ms) | **CONTAINED (100%)** |
| `V02` | WASI Ambient Network / Proc | `sock_open`, `connect()`, `proc_exec("/bin/sh")` | Zero ambient syscall capability tokens granted to untrusted WASM runtimes | `HARD_CAPABILITY_DENIAL` (< 1 ms) | **CONTAINED (100%)** |
| `V03` | WASM Fuel Bomb & Recursion | Infinite loop, deep mutual recursion, unbounded `memory.grow` | Fuel metering preemption with stack frame limiter | `FUEL_EXHAUSTED_PREEMPT` (< 50 ms) | **CONTAINED (100%)** |
| `V04` | GPU Shader Cross-Tenant Read | WGSL shader reading uninitialized storage buffer, nested loops | Pre-dispatch zero-scrubbing (`0x00000000`) and GPU loop-fuel watchdog preemption | `ZERO_SCRUBBED_AND_TDR_PREEMPT` (< 5 ms) | **CONTAINED (100%)** |
| `V05` | GGUF Arbitrary Offset Read | Tensor offset past EOF or negative wrap-around (`u64::MAX - 16`) | `ModelIntegrityVerifier` bounds checks before `VerifiedModelGuard` instantiation | `POINTER_REJECTED_BEFORE_LOAD` (< 1 ms) | **CONTAINED (100%)** |
| `V06` | Unauthenticated Rogue Peer | Self-signed or untrusted root CA certificate in mTLS handshake | Peer trust verification rejecting un-enrolled CA before Quic stream allocation | `MTLS_HANDSHAKE_REJECTED` (< 2 ms) | **CONTAINED (100%)** |
| `V07` | Revoked Cert 0-RTT Replay | Revoked certificate attempting 0-RTT QUIC resumption | Immediate session ticket cache invalidation upon CRL / revocation update | `TICKET_INVALIDATED_REJECTED` (< 1 ms) | **CONTAINED (100%)** |
| `V08` | RBAC Privilege Escalation | GuestObserver / `cluster:read` token calling mutating endpoints | Mandatory token scope inspection rejecting unauthorized mutations | `FORBIDDEN_403_REJECTED` (< 1 ms) | **CONTAINED (100%)** |
| `V09` | Private AI Prompt Exfiltration | Compromised coordinator suggesting remote offload in `StrictLocalOnly` | Local enclave circuit breaker aborting offload and raising `CRITICAL` alert | `HARD_CIRCUIT_BREAKER_ABORT` (< 1 ms) | **CONTAINED (100%)** |
| `V10` | Stream ViewOnly Input Hijack | Client with `ViewOnly` capability sending `InputEventPacket` / clipboard events | QUIC datagram filter dropping unprivileged input and quarantining peer | `INPUT_DROPPED_PEER_QUARANTINED` (< 1 ms) | **CONTAINED (100%)** |
| `V11` | Timing Side-Channel on Secrets | High-resolution timing analysis on API keys and 6-digit PIN verification | Cryptographic constant-time comparisons via `subtle::ConstantTimeEq` | `CONSTANT_TIME_VERIFIED` (< 0.1 ms) | **CONTAINED (100%)** |

### 2.2 45-Endpoint x 5-Privilege Tier RBAC Authorization Matrix
Every HTTP, gRPC, and WebSocket endpoint in UMCF is validated against a strict authorization matrix across 5 tiers:
1. `Unauthenticated`: Access limited strictly to `/health` and initial `/auth/enroll`.
2. `Revoked`: Access to 100% of endpoints rejected with `401 Unauthorized` or `403 Forbidden`.
3. `ViewOnlyGuest`: Read-only access to cluster status and stream video; 100% of mutating RPCs rejected with `403 Forbidden`.
4. `ComputeWorker`: Scoped worker RPCs (`claim_job`, `submit_result`, `heartbeat`) permitted; administrative and destructive endpoints rejected.
5. `ClusterAdmin`: Full cluster governance, model registration, node revocation, and emergency halt.

**Audit Result**: 225 out of 225 automated authorization checks passed with zero false negatives and zero privilege leaks (`artifacts/evidence/stage-08/p2/sp2_endpoint_rbac_authorization_matrix.json`).

### 2.3 Tamper-Evident Security Audit Ledger (`SecurityAuditLedger`)
All blocked adversarial attempts emit a structured event into `SecurityAuditLedger` featuring:
- `timestamp_ms`: Deterministic monotonic timestamp.
- `vector_category`: Machine-readable threat classification.
- `source_node_or_ip`: Identified origin of the malicious frame.
- `severity`: Structured `SecuritySeverity` enum (`Critical`, `High`, `Medium`, `Low`).
- `action_taken`: Neutralization action (`BLOCK`, `HARD_ABORT`, `QUARANTINE`, `PREEMPT`, `REJECT`).
- Cryptographic hash chaining preventing retrospective tampering or audit erasure.

---

## 3. Threat Model & Attack Surface Invariants

1. **Zero Unchecked Panics**: No malformed, truncated, or random byte sequence may cause an unhandled panic (`unwrap()`, `expect()`, or out-of-bounds indexing) in any non-test path.
2. **Deterministic Error Responses**: All protocol violations return structured error variants (`FuzzError::MalformedInput`, `FuzzError::AllocationExceeded`) with constant-time or sub-millisecond execution.
3. **Memory Safety**: Pure Rust codebases enforce `#![forbid(unsafe_code)]`. FFI bindings in `umcf-llama-sys` and GPU wrappers must isolate `unsafe` blocks with complete `// SAFETY:` justifications and catch all unwinding boundaries (`std::panic::catch_unwind`).

