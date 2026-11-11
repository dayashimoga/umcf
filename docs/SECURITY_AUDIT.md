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

## 2. Threat Model & Attack Surface Invariants

1. **Zero Unchecked Panics**: No malformed, truncated, or random byte sequence may cause an unhandled panic (`unwrap()`, `expect()`, or out-of-bounds indexing) in any non-test path.
2. **Deterministic Error Responses**: All protocol violations return structured error variants (`FuzzError::MalformedInput`, `FuzzError::AllocationExceeded`) with constant-time or sub-millisecond execution.
3. **Memory Safety**: Pure Rust codebases enforce `#![forbid(unsafe_code)]`. FFI bindings in `umcf-llama-sys` and GPU wrappers must isolate `unsafe` blocks with complete `// SAFETY:` justifications and catch all unwinding boundaries (`std::panic::catch_unwind`).

