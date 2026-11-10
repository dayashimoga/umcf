# Subphase S7.P2.SP3 Execution Prompt: Declarative Workload Manifests & Reproducible Job Templates

## 1. Role & Context
You are the Workflow Orchestration & Reproducible Compute Engineer for UMCF. In Subphase `S7.P2.SP3`, you will implement `crates/umcf-manifest`, enabling developers to define multi-step distributed pipelines—combining WASM data preprocessing, GPU shader compute, distributed AI inference, and media transcoding—in a single declarative `umcf.job.yaml` (or `.toml`) manifest backed by a cryptographic `umcf.lock` lockfile.

## 2. Prerequisites & Dependencies
- `S7.P2.SP1` & `S7.P2.SP2` (`umcf-sdk`, `umcf-cli`, `umcf-storage`).
- `crates/umcf-scheduler` (DAG execution engine).

## 3. Implementation Tasks
1. **Declarative Workload Manifest Schema & Parser (`crates/umcf-manifest/src/schema.rs`)**:
   - Support `apiVersion: umcf.io/v1` YAML and TOML workload manifests (`WorkloadManifest`):
     ```yaml
     apiVersion: umcf.io/v1
     kind: DistributedWorkflow
     metadata:
       name: video-ai-summarizer
       version: "1.0.0"
     parameters:
       input_video: { type: path, required: true }
       summary_language: { type: string, default: "en" }
     privacy:
       mode: trusted-devices # strict-local | trusted-devices | cluster-encrypted
     steps:
       - id: extract_keyframes
         kind: media.transcode
         inputs: { source: "${params.input_video}" }
         config: { codec: h264, resolution: "1280x720" }
       - id: preprocess_wasm
         kind: wasm.mapreduce
         depends_on: [extract_keyframes]
         module: "cid://blake3:8f9a..."
         resources: { max_memory_mb: 256, timeout_sec: 60 }
       - id: summarize_llm
         kind: ai.inference
         depends_on: [preprocess_wasm]
         model: "cid://blake3:3c1d..."
         prompt_template: "Summarize in ${params.summary_language}: ${steps.preprocess_wasm.output_text}"
         sampling: { temperature: 0.0, seed: 42 }
     outputs:
       report_cid: "${steps.summarize_llm.output_cid}"
     ```
   - Implement strict static validator (`ManifestValidator`):
     - Detects unknown fields (`#[serde(deny_unknown_fields)]`), validates parameter types, checks step ID uniqueness, verifies topological DAG acyclicity (Kahn's algorithm), and validates inter-step `${steps.<id>.<output>}` type compatibility before runtime.
2. **Cryptographic Lockfile Resolver (`crates/umcf-manifest/src/lockfile.rs`)**:
   - Implement `umcf job lock <MANIFEST>` generating `umcf.lock`:
     - Resolves local file paths or named model/module tags into immutable BLAKE3 `ObjectCid` / `ModelId` hashes, records exact file sizes, quantization types, and redundancy profiles, and signs the lockfile with the developer's Ed25519 key.
   - When executing with `--frozen-lockfile`, reject execution immediately if any referenced module, shader, model, or dataset hash differs from `umcf.lock`.
3. **Built-In Reproducible Job Templates Catalog (`crates/umcf-manifest/src/templates.rs`)**:
   - Ship 6 ready-to-run, tested templates accessible via `umcf job init --template <NAME>`:
     1. `wasm-parallel-map-reduce` (Data analytics across cluster nodes)
     2. `ai-batch-rag-embeddings` (Distributed document chunking + `/v1/embeddings` + vector index build)
     3. `ai-private-document-qa` (`StrictLocalOnly` enclave LLM synthesis)
     4. `gpu-distributed-raytracer` (4K tile-rendered WGSL path tracer)
     5. `media-4k-av1-transcode` (GOP-segmented hardware-accelerated video transcoding)
     6. `hybrid-media-ai-pipeline` (Multi-stage Video -> Audio/Frame Extraction -> WASM Filter -> LLM Summary)
4. **UI Integration**:
   - Flutter & Svelte 5: Visual Workflow DAG Builder & Template Gallery allowing one-click launch of any of the 6 templates with a form auto-generated from the manifest's `parameters:` section.

## 4. Acceptance Criteria
- >= 95% line/branch coverage across `crates/umcf-manifest`; 100% test pass rate.
- `ManifestValidator` catches 100% of cyclic dependencies (`A -> B -> C -> A`), undefined parameter references, and type mismatches between producer and consumer steps with line/column source error reporting.
- Running a deterministic workflow (`temperature: 0.0, seed: 42` for AI, deterministic WASM & integer GPU compute) twice with `--frozen-lockfile` produces an identical `umcf.lock` verification receipt and bit-identical output `ObjectCid`.
- All 6 built-in templates validate, lock, and execute cleanly end-to-end in integration tests.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_manifest_yaml_and_toml_parser_deny_unknown_fields`
  - `test_dag_cycle_detection_and_topological_stage_grouping`
  - `test_parameter_and_step_output_interpolation_type_checker`
  - `test_lockfile_blake3_mismatch_rejected_under_frozen_lockfile`
- **`[INT]`**:
  - `test_all_6_builtin_job_templates_init_lock_and_execute_e2e`
  - `test_multi_step_hybrid_wasm_gpu_ai_dag_pipeline_execution`
- **`[SIM]`**:
  - Mid-workflow failure at step 2 of 4 followed by `umcf job resume <RUN_ID>`, verifying step 1's cached `ObjectCid` output is reused without re-executing step 1.
- **`[HW]`**:
  - Execute the `hybrid-media-ai-pipeline` template on real hardware and verify step-by-step execution timings.
- **`[MANUAL]`**:
  - Browse the Template Gallery in Svelte 5 / Flutter UI, customize parameters via the auto-generated form, and watch the live DAG node graph turn green step by step.

## 6. Required Evidence & Artifacts
- `artifacts/stage-07/p2/sp3_template_execution_receipts.json`
- `artifacts/stage-07/p2/sp3_reproducible_lockfile_verification.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document the `umcf.io/v1` manifest specification, `umcf.lock` format, and built-in templates in `docs/sdk/DEVELOPER_GUIDE.md` (Section 2).
- Mark `S7.P2.SP3` and Phase `S7.P2` complete in `TODO.md`; update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If variable interpolation allows shell command injection or path traversal outside the workspace, or if `--frozen-lockfile` permits executing an unverified/mutated artifact.
- **Completion Gate**: Phase `S7.P2` completion gate passed (`S7_P2_SP1`..`SP3` green, >= 95% coverage). Proceed to Phase `S7.P3`.
