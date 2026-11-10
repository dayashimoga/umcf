# Subphase S8.P2.SP1 Execution Prompt: Protocol Parser & FFI Fuzzing (`cargo-fuzz` / Proptest)

## 1. Role & Context
You are the Fuzzing & Memory-Safety Verification Engineer for UMCF. In Subphase `S8.P2.SP1`, you will implement `fuzz/` (`cargo-fuzz` LibFuzzer targets) and structured property-based fuzz suites (`proptest`) across every untrusted input parser and FFI boundary in the UMCF workspace. No matter how malformed, truncated, or maliciously crafted a network packet, GGUF model header, WGSL shader, WASM bytecode binary, or YAML manifest is, UMCF must never panic (`unwrap()`, `expect()`, index out-of-bounds, integer overflow), allocate unbounded RAM (OOM bomb), or trigger C++ undefined behavior.

## 2. Prerequisites & Dependencies
- All Stage 1–7 workspace crates.
- `libfuzzer-sys`, `arbitrary` (`#[derive(Arbitrary)]`), and `proptest`.

## 3. Implementation Tasks
1. **Create 15 Coverage-Guided Fuzz Targets (`fuzz/fuzz_targets/*.rs`)**:
   - Implement and wire 15 dedicated fuzz harnesses (plus a deterministic corpus runner in `tests/fuzz_corpus_regression.rs` so standard `cargo test` replays the minimized seed & edge-case corpus on every platform):
     1. `fuzz_p2p_wire_frame`: Fuzzes `umcf-net` QUIC frame decoder, handshake messages, DHT records, and gossip payloads.
     2. `fuzz_capability_token_and_cert`: Fuzzes `umcf-crypto` Ed25519/X.509/DID identity certificates and scoped capability token deserializers.
     3. `fuzz_wasm_module_loader`: Fuzzes `umcf-wasm` Wasmtime module validator, WASI import linker, and resource limiter headers.
     4. `fuzz_scheduler_dag_proto`: Fuzzes `umcf-scheduler` task graph protobuf/bincode messages and work-stealing state deltas.
     5. `fuzz_gguf_parser`: Fuzzes `umcf-gguf` v2/v3 header, metadata KV array nesting, string lengths, and tensor offset table validation (`S5.P1.SP2`).
     6. `fuzz_llama_ffi_tokenizer_and_grammar`: Fuzzes `umcf-ai` `SafeLlamaModel` tokenizer input, incremental UTF-8 piece detokenizer (`Utf8TokenAssembler`), and JSON-Schema-to-GBNF grammar compiler (`S5.P2`).
     7. `fuzz_ai_activation_rpc_packet`: Fuzzes `umcf-ai` distributed `ForwardActivationPacket` & FP16/Q8 activation decompressor (`S5.P3.SP2`).
     8. `fuzz_openai_http_json_body`: Fuzzes `umcf-ai` `/v1/chat/completions` & Jinja2 chat template renderer (`S5.P3.SP3`).
     9. `fuzz_wgsl_and_spirv_shader_sandbox`: Fuzzes `umcf-gpu` `naga` WGSL/SPIR-V parser and AST loop-fuel watchdog injector (`S6.P1.SP1`).
     10. `fuzz_render_tile_packet`: Fuzzes `umcf-gpu` compressed tile decoder and EXR/PNG compositor (`S6.P1.SP2`).
     11. `fuzz_media_segment_metadata`: Fuzzes `umcf-media` GOP keyframe index parser and PTS/DTS continuity normalizer (`S6.P2`).
     12. `fuzz_stream_rtp_fec_and_input`: Fuzzes `umcf-stream` Reed-Solomon FEC packet reassembler and `InputEventPacket` binary decoder (`S6.P3`).
     13. `fuzz_storage_dag_and_erasure_shard`: Fuzzes `umcf-storage` `ObjectDagManifest` CBOR/bincode parser and Reed-Solomon shard header verifier (`S7.P1`).
     14. `fuzz_workload_manifest_yaml_toml`: Fuzzes `umcf-manifest` YAML/TOML parser, variable interpolator, and `umcf.lock` verifier (`S7.P2.SP3`).
     15. `fuzz_resource_receipt_and_market_order`: Fuzzes `umcf-ledger` `SignedResourceReceipt` and `umcf-marketplace` bid/ask order deserializers (`S7.P3`).
2. **Pre-Allocation Length Cap & Recursion Depth Hardening**:
   - Audit every deserializer (`bincode`, `prost`, `serde_json`, `serde_yaml`, `ciborium`, custom binary readers) and enforce explicit `max_allocation_bytes` (e.g., rejecting a corrupt length prefix claiming a `Vec<u8>` has $2^{62}$ elements before `Vec::with_capacity` is called) and `max_recursion_depth <= 32`.

## 4. Acceptance Criteria
- >= 95% line/branch coverage across all parser and validator entrypoints; 100% pass rate.
- **Zero Panics, Zero UB, Zero OOMs**: All 15 fuzz targets run for `>= 500,000` iterations each (and `>= 10,000` structured `proptest` cases each) with **0 crashes, 0 panics, 0 timeouts (> 1s per input), and 0 allocations exceeding 16 MiB on a 64 KiB fuzz input**.
- Static lint check confirms `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::arithmetic_side_effects)]` is enforced across all wire/file parser modules.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_length_prefix_allocation_bomb_2_pow_62_rejected_with_zero_heap_alloc`
  - `test_deeply_nested_gguf_array_and_json_schema_recursion_limit_enforced`
  - `test_minimized_fuzz_corpus_regression_all_15_targets`
- **`[INT]`**:
  - `test_proptest_structured_fuzz_suite_15_targets_10000_cases_each`
- **`[SIM]`**:
  - Live network socket fuzzer blasting malformed QUIC frames and HTTP requests at a running UMCF daemon while verifying legitimate client requests continue with 0 latency degradation.
- **`[HW]`**:
  - Multi-core parallel fuzzing execution report (`cargo fuzz` / `proptest` with ASan enabled).
- **`[MANUAL]`**:
  - Review `artifacts/stage-08/p2/sp1_fuzz_summary.json` verifying coverage percentages and zero crash artifacts in `fuzz/artifacts/`.

## 6. Required Evidence & Artifacts
- `artifacts/stage-08/p2/sp1_fuzz_15_targets_execution_log.json`
- `artifacts/stage-08/p2/sp1_clippy_no_unwrap_parser_audit.txt`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document all 15 fuzz targets, corpus management, and parser allocation-cap rules in `docs/SECURITY_AUDIT.md` (Section 1).
- Mark `S8.P2.SP1` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: Any crash, panic, stack overflow, or OOM discovered in any of the 15 fuzz targets.
- **Completion Gate**: All 15 fuzz targets and corpus regression tests green before proceeding to `S8.P2.SP2`.
