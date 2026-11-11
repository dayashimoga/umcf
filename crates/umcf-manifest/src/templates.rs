//! Built-in reproducible job templates catalog for UMCF.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::ManifestError;
use crate::schema::WorkloadManifest;

/// Summary information for a built-in workflow template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateSummary {
    /// Template identifier (e.g., `hybrid-media-ai-pipeline`).
    pub name: String,
    /// Category / execution profile.
    pub category: String,
    /// Human-readable description.
    pub description: String,
    /// Expected parameter keys.
    pub parameters: Vec<String>,
}

/// Catalog of all 6 standard built-in reproducible templates.
#[derive(Debug, Default)]
pub struct TemplateCatalog;

impl TemplateCatalog {
    /// Lists all 6 available templates with metadata.
    pub fn list() -> Vec<TemplateSummary> {
        vec![
            TemplateSummary {
                name: "wasm-parallel-map-reduce".to_string(),
                category: "Data Processing".to_string(),
                description: "Distributed MapReduce data crunching across heterogeneous cluster nodes using sandboxed WASM.".to_string(),
                parameters: vec!["dataset_path".to_string(), "reducers_count".to_string()],
            },
            TemplateSummary {
                name: "ai-batch-rag-embeddings".to_string(),
                category: "AI & Search".to_string(),
                description: "Distributed document chunking, batched vector embeddings, and index generation.".to_string(),
                parameters: vec!["corpus_dir".to_string(), "embedding_model".to_string(), "chunk_size".to_string()],
            },
            TemplateSummary {
                name: "ai-private-document-qa".to_string(),
                category: "Private AI".to_string(),
                description: "StrictLocalOnly confidential document QA with local on-device GGUF LLM synthesis.".to_string(),
                parameters: vec!["private_doc".to_string(), "question".to_string(), "local_model".to_string()],
            },
            TemplateSummary {
                name: "gpu-distributed-raytracer".to_string(),
                category: "GPU Compute".to_string(),
                description: "Tile-parallel 4K raytracing / path tracing distributed across cluster GPU shaders.".to_string(),
                parameters: vec!["scene_file".to_string(), "resolution".to_string(), "samples_per_pixel".to_string()],
            },
            TemplateSummary {
                name: "media-4k-av1-transcode".to_string(),
                category: "Media Transcoding".to_string(),
                description: "GOP-segmented hardware-accelerated 4K AV1 video transcoding with Annex-B restitching.".to_string(),
                parameters: vec!["input_video".to_string(), "target_bitrate_kbps".to_string(), "crf".to_string()],
            },
            TemplateSummary {
                name: "hybrid-media-ai-pipeline".to_string(),
                category: "Multi-Modal Hybrid".to_string(),
                description: "Multi-stage pipeline: Video -> Keyframe Extraction -> WASM Visual Filter -> LLM Synthesis.".to_string(),
                parameters: vec!["input_video".to_string(), "summary_language".to_string(), "llm_model".to_string()],
            },
        ]
    }

    /// Retrieves the raw YAML template definition by template name.
    pub fn get_template(name: &str) -> Option<&'static str> {
        match name {
            "wasm-parallel-map-reduce" => Some(WASM_PARALLEL_MAP_REDUCE_YAML),
            "ai-batch-rag-embeddings" => Some(AI_BATCH_RAG_EMBEDDINGS_YAML),
            "ai-private-document-qa" => Some(AI_PRIVATE_DOCUMENT_QA_YAML),
            "gpu-distributed-raytracer" => Some(GPU_DISTRIBUTED_RAYTRACER_YAML),
            "media-4k-av1-transcode" => Some(MEDIA_4K_AV1_TRANSCODE_YAML),
            "hybrid-media-ai-pipeline" => Some(HYBRID_MEDIA_AI_PIPELINE_YAML),
            _ => None,
        }
    }

    /// Initializes a template into the specified target directory.
    pub fn init_template(name: &str, target_dir: &Path) -> Result<PathBuf, ManifestError> {
        let yaml_content =
            Self::get_template(name).ok_or_else(|| ManifestError::TemplateNotFound {
                name: name.to_string(),
            })?;

        // Verify that template parses and validates cleanly before writing
        let manifest = WorkloadManifest::from_yaml_str(yaml_content)?;
        crate::validator::ManifestValidator::validate(&manifest)?;

        fs::create_dir_all(target_dir)?;
        let file_path = target_dir.join("umcf.job.yaml");
        fs::write(&file_path, yaml_content)?;

        Ok(file_path)
    }
}

// ============================================================================
// 1. wasm-parallel-map-reduce
// ============================================================================
pub const WASM_PARALLEL_MAP_REDUCE_YAML: &str = r#"apiVersion: umcf.io/v1
kind: DistributedWorkflow
metadata:
  name: wasm-parallel-map-reduce
  version: "1.0.0"
  description: "Distributed MapReduce data crunching across heterogeneous cluster nodes"
parameters:
  dataset_path:
    type: path
    required: true
    description: "Input dataset file path"
  reducers_count:
    type: int
    required: false
    default: 4
    description: "Number of concurrent reducer partitions"
privacy:
  mode: trusted-devices
steps:
  - id: ingest_dataset
    kind: storage.ingest
    inputs:
      source_file: "${params.dataset_path}"
    config:
      chunking: fastcdc
      min_size_kb: 64
      avg_size_kb: 256
      max_size_kb: 1024
  - id: map_phase
    kind: wasm.mapreduce
    depends_on:
      - ingest_dataset
    module: "cid://blake3:bafkrei-wasm-mapper-v1"
    inputs:
      input_cid: "${steps.ingest_dataset.root_cid}"
    config:
      action: "map"
      partitions: 16
    resources:
      max_memory_mb: 512
      timeout_sec: 120
  - id: reduce_phase
    kind: wasm.mapreduce
    depends_on:
      - map_phase
    module: "cid://blake3:bafkrei-wasm-reducer-v1"
    inputs:
      intermediate_cid: "${steps.map_phase.output_cid}"
    config:
      action: "reduce"
      reducers: "${params.reducers_count}"
    resources:
      max_memory_mb: 1024
      timeout_sec: 180
outputs:
  result_cid: "${steps.reduce_phase.output_cid}"
"#;

// ============================================================================
// 2. ai-batch-rag-embeddings
// ============================================================================
pub const AI_BATCH_RAG_EMBEDDINGS_YAML: &str = r#"apiVersion: umcf.io/v1
kind: DistributedWorkflow
metadata:
  name: ai-batch-rag-embeddings
  version: "1.0.0"
  description: "Distributed document chunking, batched vector embeddings, and index generation"
parameters:
  corpus_dir:
    type: path
    required: true
    description: "Directory containing source markdown/text files"
  embedding_model:
    type: string
    required: false
    default: "nomic-embed-text-v1.5.Q8_0.gguf"
    description: "GGUF embedding model name"
  chunk_size:
    type: int
    required: false
    default: 512
    description: "Token chunk length for document slicing"
privacy:
  mode: trusted-devices
steps:
  - id: ingest_corpus
    kind: storage.ingest
    inputs:
      source_file: "${params.corpus_dir}"
    config:
      recursive: true
  - id: chunk_documents
    kind: wasm.mapreduce
    depends_on:
      - ingest_corpus
    module: "cid://blake3:bafkrei-wasm-text-chunker"
    inputs:
      corpus_cid: "${steps.ingest_corpus.root_cid}"
    config:
      chunk_size: "${params.chunk_size}"
      overlap: 64
  - id: generate_embeddings
    kind: ai.inference
    depends_on:
      - chunk_documents
    model: "${params.embedding_model}"
    inputs:
      chunks_cid: "${steps.chunk_documents.output_cid}"
    config:
      batch_size: 64
      normalize: true
outputs:
  vector_index_cid: "${steps.generate_embeddings.output_cid}"
"#;

// ============================================================================
// 3. ai-private-document-qa
// ============================================================================
pub const AI_PRIVATE_DOCUMENT_QA_YAML: &str = r#"apiVersion: umcf.io/v1
kind: DistributedWorkflow
metadata:
  name: ai-private-document-qa
  version: "1.0.0"
  description: "StrictLocalOnly confidential document QA with local on-device GGUF LLM synthesis"
parameters:
  private_doc:
    type: path
    required: true
    description: "Path to private document file"
  question:
    type: string
    required: true
    description: "User question to answer"
  local_model:
    type: string
    required: false
    default: "llama-3-8b-instruct.Q4_K_M.gguf"
    description: "Locally resident GGUF model"
privacy:
  mode: strict-local
steps:
  - id: extract_local_text
    kind: wasm.mapreduce
    inputs:
      doc_path: "${params.private_doc}"
    config:
      action: "extract_text"
    resources:
      max_memory_mb: 256
      timeout_sec: 30
  - id: synthesize_answer
    kind: ai.inference
    depends_on:
      - extract_local_text
    model: "${params.local_model}"
    inputs:
      context: "${steps.extract_local_text.output_text}"
    prompt_template: "Context:\n${steps.extract_local_text.output_text}\n\nQuestion: ${params.question}\n\nAnswer concisely based strictly on the context above."
    sampling:
      temperature: 0.0
      seed: 42
      top_p: 0.95
    resources:
      max_memory_mb: 6144
      timeout_sec: 60
outputs:
  answer: "${steps.synthesize_answer.output_text}"
"#;

// ============================================================================
// 4. gpu-distributed-raytracer
// ============================================================================
pub const GPU_DISTRIBUTED_RAYTRACER_YAML: &str = r#"apiVersion: umcf.io/v1
kind: DistributedWorkflow
metadata:
  name: gpu-distributed-raytracer
  version: "1.0.0"
  description: "Tile-parallel 4K raytracing distributed across cluster GPU shaders"
parameters:
  scene_file:
    type: path
    required: true
    description: "Path to 3D scene description"
  resolution:
    type: string
    required: false
    default: "3840x2160"
    description: "Target render resolution"
  samples_per_pixel:
    type: int
    required: false
    default: 256
    description: "Monte Carlo samples per pixel"
privacy:
  mode: trusted-devices
steps:
  - id: ingest_scene
    kind: storage.ingest
    inputs:
      scene_path: "${params.scene_file}"
    config:
      compression: zstd
  - id: render_tiles
    kind: gpu.render
    depends_on:
      - ingest_scene
    inputs:
      scene_cid: "${steps.ingest_scene.root_cid}"
    config:
      resolution: "${params.resolution}"
      samples: "${params.samples_per_pixel}"
      tiles: 16
    resources:
      vram_mb: 2048
      timeout_sec: 300
  - id: denoise_and_stitch
    kind: wasm.mapreduce
    depends_on:
      - render_tiles
    module: "cid://blake3:bafkrei-wasm-image-stitcher"
    inputs:
      tiles_cid: "${steps.render_tiles.output_cid}"
    config:
      format: png
      denoise: true
outputs:
  rendered_image_cid: "${steps.denoise_and_stitch.output_cid}"
"#;

// ============================================================================
// 5. media-4k-av1-transcode
// ============================================================================
pub const MEDIA_4K_AV1_TRANSCODE_YAML: &str = r#"apiVersion: umcf.io/v1
kind: DistributedWorkflow
metadata:
  name: media-4k-av1-transcode
  version: "1.0.0"
  description: "GOP-segmented hardware-accelerated 4K AV1 video transcoding with Annex-B restitching"
parameters:
  input_video:
    type: path
    required: true
    description: "Input source video path"
  target_bitrate_kbps:
    type: int
    required: false
    default: 12000
    description: "Target video stream bitrate"
  crf:
    type: int
    required: false
    default: 24
    description: "Constant Rate Factor quality parameter"
privacy:
  mode: trusted-devices
steps:
  - id: ingest_video
    kind: storage.ingest
    inputs:
      video_path: "${params.input_video}"
  - id: transcode_segments
    kind: media.transcode
    depends_on:
      - ingest_video
    inputs:
      source_cid: "${steps.ingest_video.root_cid}"
    config:
      codec: av1
      bitrate_kbps: "${params.target_bitrate_kbps}"
      crf: "${params.crf}"
      segment_seconds: 4
      gop_size: 120
    resources:
      cpu_cores: 8
      vram_mb: 2048
  - id: stitch_annex_b
    kind: wasm.mapreduce
    depends_on:
      - transcode_segments
    module: "cid://blake3:bafkrei-wasm-annexb-stitcher"
    inputs:
      segments_cid: "${steps.transcode_segments.output_cid}"
    config:
      output_container: mp4
outputs:
  transcoded_video_cid: "${steps.stitch_annex_b.output_cid}"
"#;

// ============================================================================
// 6. hybrid-media-ai-pipeline
// ============================================================================
pub const HYBRID_MEDIA_AI_PIPELINE_YAML: &str = r#"apiVersion: umcf.io/v1
kind: DistributedWorkflow
metadata:
  name: hybrid-media-ai-pipeline
  version: "1.0.0"
  description: "Multi-stage pipeline: Video -> Keyframe Extraction -> WASM Visual Filter -> LLM Synthesis"
parameters:
  input_video:
    type: path
    required: true
    description: "Source video file to summarize"
  summary_language:
    type: string
    required: false
    default: "en"
    description: "Language for the generated summary report"
  llm_model:
    type: string
    required: false
    default: "llama-3-8b-instruct.Q4_K_M.gguf"
    description: "GGUF LLM model for synthesis"
privacy:
  mode: trusted-devices
steps:
  - id: extract_keyframes
    kind: media.transcode
    inputs:
      source: "${params.input_video}"
    config:
      codec: h264
      resolution: "1280x720"
      extract_keyframes_interval_sec: 10
  - id: preprocess_wasm
    kind: wasm.mapreduce
    depends_on:
      - extract_keyframes
    module: "cid://blake3:8f9a2b1c3d4e5f60"
    inputs:
      frames_cid: "${steps.extract_keyframes.output_cid}"
    config:
      action: "ocr_and_visual_descriptors"
    resources:
      max_memory_mb: 256
      timeout_sec: 60
  - id: summarize_llm
    kind: ai.inference
    depends_on:
      - preprocess_wasm
    model: "${params.llm_model}"
    inputs:
      descriptors: "${steps.preprocess_wasm.output_text}"
    prompt_template: "Summarize in ${params.summary_language}: ${steps.preprocess_wasm.output_text}"
    sampling:
      temperature: 0.0
      seed: 42
  - id: export_report
    kind: storage.ingest
    depends_on:
      - summarize_llm
    inputs:
      content: "${steps.summarize_llm.output_text}"
    config:
      format: markdown
outputs:
  report_cid: "${steps.export_report.output_cid}"
"#;
