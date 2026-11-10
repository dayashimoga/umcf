# Subphase S5.P1.SP2 Execution Prompt: GGUF Metadata Inspector, Memory Footprint Estimator & Compatibility Validator

## 1. Role & Context
You are the AI Runtime & Model Formats Engineer for UMCF. In Subphase `S5.P1.SP2`, you will implement a safe, zero-copy GGUF v2/v3 binary format inspector, a deterministic RAM/VRAM + KV-cache footprint estimator, and a hardware compatibility validator in `crates/umcf-gguf`. Before scheduling any inference job, UMCF must know the exact tensor layout, architecture hyperparameters, quantization types, and per-layer memory requirements without invoking unsafe C++ code.

## 2. Prerequisites & Dependencies
- `S5.P1.SP1` (`ContentAddressedModelStore` and `VerifiedModelMmap`).
- `crates/umcf-hw` (`DeviceCapabilities`, available RAM/VRAM, SIMD/GPU feature flags).

## 3. Implementation Tasks
1. **Implement Safe Zero-Copy GGUF v2/v3 Parser (`crates/umcf-gguf/src/parser.rs`)**:
   - Parse magic `0x46554747` (`"GGUF"`), version (`2` or `3`), `tensor_count: u64`, and `metadata_kv_count: u64` with strict bounds and overflow checks (`checked_add`, `checked_mul`, max string/array caps).
   - Parse all 13 GGUF metadata value types (`UInt8`, `Int8`, `UInt16`, `Int16`, `UInt32`, `Int32`, `Float32`, `Bool`, `String`, `Array`, `UInt64`, `Int64`, `Float64`).
   - Extract normalized `ModelArchitectureMetadata`:
     - `general.architecture` (`llama`, `mistral`, `qwen2`, `gemma2`, `phi3`, `deepseek2`, `nomic-bert`, etc.)
     - `block_count` (layers), `context_length`, `embedding_length`, `feed_forward_length`, `attention.head_count`, `attention.head_count_kv`, `rope.freq_base`, `rope.dimension_count`, `tokenizer.ggml.model`, and vocabulary size.
   - Parse `GgufTensorInfo` entries: tensor name, dimensions (`n_dims <= 4`), `GgmlType` (F32, F16, BF16, Q4_0, Q4_1, Q5_0, Q5_1, Q8_0, Q8_1, Q2_K..Q8_K, IQ1_S..IQ4_XS), and relative byte offset aligned to `general.alignment` (default 32 bytes).
   - Group tensors by transformer block index (`blk.{i}.*`) plus token embedding (`token_embd.weight`) and output head (`output.weight`, `output_norm.weight`) into `LayerWeightProfile`.
2. **Implement Memory Footprint Estimator (`crates/umcf-gguf/src/estimator.rs`)**:
   - Implement `ModelFootprintEstimator::estimate(&ModelInspection, &InferenceParams) -> MemoryFootprintReport`:
     - **Static Weight Bytes**: Exact sum of tensor byte sizes (`(nelements / block_size) * type_size`) per layer and globally.
     - **KV-Cache Bytes**:
       $$\text{KV}_{\text{bytes}} = 2 \times n_{\text{layers}} \times n_{\text{ctx}} \times n_{\text{embd\_gqa}} \times \text{sizeof}(\text{KV\_dtype})$$
       where $n_{\text{embd\_gqa}} = (\text{embedding\_length} / n_{\text{head}}) \times n_{\text{head\_kv}}$.
     - **Compute & Scratch Buffer Bytes**: Estimate activation & flash-attention scratch overhead based on `n_batch`, `n_ubatch`, and `n_ctx`.
   - Implement `HardwareCompatibilityValidator::validate(&MemoryFootprintReport, &DeviceCapabilities) -> CompatibilityVerdict`:
     - Returns `Verdict::FitsSingleNode { backend, recommended_gpu_layers, max_safe_ctx }`, `Verdict::RequiresClusterSplit { min_nodes_needed, bottleneck_reason }`, or `Verdict::IncompatibleQuantization { unsupported_types }`.
     - Enforce OS-specific safety headroom (reserve >= 1.5 GiB on Android/iOS to prevent mobile LMK/jetsam kills; reserve >= 15% VRAM on desktop GPUs).
3. **UI Integration**:
   - Flutter: `GgufInspectorSheet` displaying architecture parameters, layer-by-layer weight chart, context-length slider dynamically updating KV-cache RAM/VRAM bar, and green/amber/red compatibility badge.
   - Svelte 5: `ModelInspectorModal.svelte` with interactive context window & batch size calculator.

## 4. Acceptance Criteria
- >= 95% line/branch coverage in `parser.rs` and `estimator.rs`; 100% test pass rate.
- Zero `unsafe` blocks in `parser.rs` and `estimator.rs` (`#![forbid(unsafe_code)]` at module level).
- Parses metadata and tensor directory of a 70B GGUF header (320+ tensors, 150k vocab tokens) in < 15 ms.
- Estimated total memory (`weights + kv_cache + compute_buffer`) matches actual `llama.cpp` memory allocation within <= 2.5% across 10 reference GGUF architectures/quantizations.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_gguf_v2_and_v3_header_and_all_metadata_types`
  - `test_ggml_quantization_block_size_and_byte_math_all_types`
  - `test_gqa_and_mha_kv_cache_memory_calculation`
  - `test_malformed_gguf_overflow_tensor_dimensions_rejected`
- **`[INT]`**:
  - Compare `ModelFootprintEstimator` output against live `llama.cpp` context allocation telemetry on Q4_K_M and Q8_0 test models.
- **`[SIM]`**:
  - Property-based (`proptest`) fuzzing with 10,000 mutated GGUF byte buffers verifying zero panics and bounded allocations.
- **`[HW]`**:
  - Validate `HardwareCompatibilityValidator` verdicts against real device capabilities (8 GB Android phone, 16 GB Apple Silicon Mac, 24 GB RTX GPU desktop).
- **`[MANUAL]`**:
  - Adjust context length slider (512 to 131,072 tokens) in Flutter/Svelte inspector and verify real-time KV-cache memory bar transitions.

## 6. Required Evidence & Artifacts
- `artifacts/stage-05/p1/sp2_estimator_vs_actual_table.json`
- `artifacts/stage-05/p1/sp2_proptest_fuzz_report.txt`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document GGUF tensor grouping rules and KV-cache estimation equations in `docs/architecture/MODEL_REGISTRY.md` (Section 2).
- Mark `S5.P1.SP2` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: Any panic or unbounded memory allocation when parsing untrusted/corrupted GGUF headers, or memory underestimation that results in an OS OOM kill during `S5.P2`.
- **Completion Gate**: 100% test pass rate and >= 95% coverage verified before `S5.P1.SP3`.
