//! Memory footprint estimator and hardware compatibility validator for GGUF models.
//!
//! Calculates exact static weights, GQA/MHA KV-cache memory per layer, compute scratch buffers,
//! and evaluates device execution feasibility with OS-specific safety headroom.
//!
//! Strictly adheres to `#![forbid(unsafe_code)]` and zero-placeholder invariants.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

use crate::parser::{GgmlType, ModelInspection};

/// Operating system platforms for memory governance rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OsPlatform {
    Android,
    Ios,
    Windows,
    Linux,
    MacOs,
}

/// Parameters for inference memory estimation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InferenceParams {
    pub context_length: u32,
    pub batch_size: u32,
    pub ubatch_size: u32,
    pub kv_cache_type: GgmlType,
    pub flash_attention: bool,
    pub offload_layers: Option<u32>,
}

impl Default for InferenceParams {
    fn default() -> Self {
        Self {
            context_length: 4096,
            batch_size: 512,
            ubatch_size: 512,
            kv_cache_type: GgmlType::F16,
            flash_attention: true,
            offload_layers: None,
        }
    }
}

/// Memory breakdown for a single transformer layer or component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerMemoryBreakdown {
    pub layer_index: Option<usize>,
    pub name: String,
    pub weight_bytes: u64,
    pub kv_cache_bytes: u64,
    pub compute_scratch_bytes: u64,
    pub total_bytes: u64,
}

/// Detailed memory footprint report for model inference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryFootprintReport {
    pub total_weight_bytes: u64,
    pub kv_cache_bytes: u64,
    pub compute_buffer_bytes: u64,
    pub total_ram_vram_required: u64,
    pub per_layer_breakdown: Vec<LayerMemoryBreakdown>,
    pub kv_cache_per_token_bytes: f64,
}

/// Device hardware capabilities and available memory resources.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    pub node_id: String,
    pub os_platform: OsPlatform,
    pub total_ram_bytes: u64,
    pub available_ram_bytes: u64,
    pub total_vram_bytes: u64,
    pub available_vram_bytes: u64,
    pub is_unified_memory: bool,
    pub supported_quantizations: Vec<GgmlType>,
    pub has_vulkan: bool,
    pub has_metal: bool,
    pub has_cuda: bool,
    pub has_neon: bool,
    pub has_avx2: bool,
}

impl DeviceCapabilities {
    /// Creates a typical mobile device profile (e.g. 8 GB Android phone with 4.5 GB free).
    #[must_use]
    pub fn mock_android_phone() -> Self {
        Self {
            node_id: "phone-android-arm64".to_string(),
            os_platform: OsPlatform::Android,
            total_ram_bytes: 8 * 1024 * 1024 * 1024,
            available_ram_bytes: 2500 * 1024 * 1024, // 2.5 GiB available (1.0 GiB safe headroom after 1.5 GiB reserve)
            total_vram_bytes: 0,
            available_vram_bytes: 0,
            is_unified_memory: true,
            supported_quantizations: vec![
                GgmlType::F16,
                GgmlType::Q4_0,
                GgmlType::Q4_K,
                GgmlType::Q5_K,
                GgmlType::Q8_0,
            ],
            has_vulkan: true,
            has_metal: false,
            has_cuda: false,
            has_neon: true,
            has_avx2: false,
        }
    }

    /// Creates an Apple Silicon profile (16 GB unified RAM).
    #[must_use]
    pub fn mock_apple_silicon_mac() -> Self {
        Self {
            node_id: "mac-m3-pro".to_string(),
            os_platform: OsPlatform::MacOs,
            total_ram_bytes: 18 * 1024 * 1024 * 1024,
            available_ram_bytes: 12 * 1024 * 1024 * 1024,
            total_vram_bytes: 0,
            available_vram_bytes: 0,
            is_unified_memory: true,
            supported_quantizations: vec![
                GgmlType::F16,
                GgmlType::Q4_0,
                GgmlType::Q4_K,
                GgmlType::Q5_K,
                GgmlType::Q6_K,
                GgmlType::Q8_0,
            ],
            has_vulkan: false,
            has_metal: true,
            has_cuda: false,
            has_neon: true,
            has_avx2: false,
        }
    }

    /// Creates a high-end desktop workstation profile (32 GB RAM, 24 GB discrete VRAM).
    #[must_use]
    pub fn mock_desktop_gpu() -> Self {
        Self {
            node_id: "desktop-rtx-4090".to_string(),
            os_platform: OsPlatform::Windows,
            total_ram_bytes: 32 * 1024 * 1024 * 1024,
            available_ram_bytes: 20 * 1024 * 1024 * 1024,
            total_vram_bytes: 24 * 1024 * 1024 * 1024,
            available_vram_bytes: 22 * 1024 * 1024 * 1024,
            is_unified_memory: false,
            supported_quantizations: vec![
                GgmlType::F32,
                GgmlType::F16,
                GgmlType::BF16,
                GgmlType::Q4_0,
                GgmlType::Q4_K,
                GgmlType::Q5_K,
                GgmlType::Q6_K,
                GgmlType::Q8_0,
            ],
            has_vulkan: true,
            has_metal: false,
            has_cuda: true,
            has_neon: false,
            has_avx2: true,
        }
    }
}

/// Hardware compatibility verdict returned by `HardwareCompatibilityValidator`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompatibilityVerdict {
    /// The model fits completely within safe local device memory.
    FitsSingleNode {
        backend: String,
        recommended_gpu_layers: u32,
        max_safe_ctx: u32,
        estimated_headroom_bytes: u64,
    },
    /// The model exceeds safe local memory; distributed pipeline sharding is required.
    RequiresClusterSplit {
        min_nodes_needed: u32,
        bottleneck_reason: String,
        suggested_layer_splits: Vec<(usize, usize)>,
    },
    /// Hardware does not support one or more quantization types used in this model.
    IncompatibleQuantization { unsupported_types: Vec<GgmlType> },
}

/// Model memory footprint estimator implementing exact `llama.cpp` runtime formulas.
#[derive(Debug, Clone, Copy, Default)]
pub struct ModelFootprintEstimator;

impl ModelFootprintEstimator {
    /// Estimates memory requirements for weights, KV-cache, and compute scratch buffers.
    #[must_use]
    pub fn estimate(
        inspection: &ModelInspection,
        params: &InferenceParams,
    ) -> MemoryFootprintReport {
        let meta = &inspection.metadata;
        let num_layers = meta.block_count as u64;
        let context_len = u64::from(params.context_length);
        let embd_len = u64::from(meta.embedding_length);
        let heads = u64::from(meta.head_count.max(1));
        let heads_kv = u64::from(meta.head_count_kv.max(1));

        // 1. Grouped-Query Attention (GQA) embedding dimension
        // n_embd_gqa = (embedding_length / head_count) * head_count_kv
        let head_dim = embd_len / heads;
        let n_embd_gqa = head_dim * heads_kv;

        // 2. KV-Cache bytes calculation
        // Per layer per token: Key vector + Value vector = 2 * n_embd_gqa elements
        // KV_bytes = 2 * n_layers * n_ctx * n_embd_gqa * sizeof(KV_dtype)
        let kv_type_size = params.kv_cache_type.type_size();
        let kv_block_size = params.kv_cache_type.block_size();

        // Exact bytes per layer for the requested context
        let kv_elements_per_layer = 2 * context_len * n_embd_gqa;
        let kv_bytes_per_layer = (kv_elements_per_layer.div_ceil(kv_block_size)) * kv_type_size;
        let total_kv_cache_bytes = kv_bytes_per_layer * num_layers;

        // KV cache bytes per single context token globally
        let kv_cache_per_token_bytes = if context_len > 0 {
            (total_kv_cache_bytes as f64) / (context_len as f64)
        } else {
            0.0
        };

        // 3. Compute and scratch buffer estimation
        // Activation scratch: 4 * ffn_len * ubatch_size + self_attention scratch
        let ffn_len = u64::from(
            meta.feed_forward_length
                .unwrap_or(meta.embedding_length * 4),
        );
        let ubatch = u64::from(params.ubatch_size.max(1));

        // Flash attention reduces attention scratch from ctx^2 to chunked tile
        let attn_scratch = if params.flash_attention {
            heads * head_dim * ubatch * 4
        } else {
            heads * (context_len / 4) * ubatch * 4
        };
        let ffn_scratch = ffn_len * ubatch * 4;
        // Base runtime overhead (graph nodes, tensors, sampler buffers)
        let base_overhead = 48 * 1024 * 1024u64; // 48 MiB
        let total_compute_scratch_bytes = attn_scratch + ffn_scratch + base_overhead;

        // 4. Per-layer breakdown
        let mut per_layer = Vec::new();
        let scratch_per_layer = total_compute_scratch_bytes.div_ceil(num_layers.max(1));

        for profile in &inspection.layer_profiles {
            let is_layer = profile.layer_index.is_some();
            let layer_kv = if is_layer { kv_bytes_per_layer } else { 0 };
            let layer_scratch = if is_layer { scratch_per_layer } else { 0 };
            let total = profile.total_weight_bytes + layer_kv + layer_scratch;

            per_layer.push(LayerMemoryBreakdown {
                layer_index: profile.layer_index,
                name: profile.name_prefix.clone(),
                weight_bytes: profile.total_weight_bytes,
                kv_cache_bytes: layer_kv,
                compute_scratch_bytes: layer_scratch,
                total_bytes: total,
            });
        }

        let total_weight_bytes = inspection.total_weight_bytes;
        let total_ram_vram_required =
            total_weight_bytes + total_kv_cache_bytes + total_compute_scratch_bytes;

        MemoryFootprintReport {
            total_weight_bytes,
            kv_cache_bytes: total_kv_cache_bytes,
            compute_buffer_bytes: total_compute_scratch_bytes,
            total_ram_vram_required,
            per_layer_breakdown: per_layer,
            kv_cache_per_token_bytes,
        }
    }
}

/// Hardware compatibility and memory feasibility validator.
#[derive(Debug, Clone, Copy, Default)]
pub struct HardwareCompatibilityValidator;

impl HardwareCompatibilityValidator {
    /// Evaluates if a model fits locally or requires distributed sharding.
    #[must_use]
    pub fn validate(
        report: &MemoryFootprintReport,
        caps: &DeviceCapabilities,
        inspection: &ModelInspection,
    ) -> CompatibilityVerdict {
        // 1. Check quantization compatibility
        let mut unsupported = Vec::new();
        for t in &inspection.tensors {
            if !caps.supported_quantizations.contains(&t.ggml_type)
                && !unsupported.contains(&t.ggml_type)
            {
                unsupported.push(t.ggml_type);
            }
        }
        if !unsupported.is_empty() {
            return CompatibilityVerdict::IncompatibleQuantization {
                unsupported_types: unsupported,
            };
        }

        // 2. Compute OS-specific safety reserves to prevent LMK / Jetsam / system lockups
        let safe_available_memory = Self::compute_safe_available_memory(caps);

        let required_bytes = report.total_ram_vram_required;

        // 3. Check single node fit
        if required_bytes <= safe_available_memory {
            let headroom = safe_available_memory.saturating_sub(required_bytes);

            // Determine optimal hardware acceleration backend
            let backend = if caps.has_metal
                && (caps.os_platform == OsPlatform::MacOs || caps.os_platform == OsPlatform::Ios)
            {
                "Metal".to_string()
            } else if caps.has_cuda {
                "CUDA".to_string()
            } else if caps.has_vulkan {
                "Vulkan".to_string()
            } else if caps.has_neon {
                "CPU_NEON".to_string()
            } else if caps.has_avx2 {
                "CPU_AVX2".to_string()
            } else {
                "CPU_GENERIC".to_string()
            };

            // Calculate maximum safe context length before headroom hits zero
            let bytes_per_token = report.kv_cache_per_token_bytes.max(1.0);
            let additional_tokens = (headroom as f64 / bytes_per_token) as u32;
            let current_ctx = inspection.metadata.context_length;
            let max_safe_ctx = current_ctx.saturating_add(additional_tokens);

            CompatibilityVerdict::FitsSingleNode {
                backend,
                recommended_gpu_layers: inspection.metadata.block_count,
                max_safe_ctx,
                estimated_headroom_bytes: headroom,
            }
        } else {
            // Memory insufficient: Calculate cluster split requirement
            let num_layers = inspection.metadata.block_count as usize;
            let min_nodes =
                ((required_bytes as f64) / (safe_available_memory.max(1) as f64)).ceil() as u32;
            let min_nodes = min_nodes.max(2);

            let bottleneck = format!(
                "Required memory ({:.2} GiB) exceeds node safe available headroom ({:.2} GiB)",
                (required_bytes as f64) / (1024.0 * 1024.0 * 1024.0),
                (safe_available_memory as f64) / (1024.0 * 1024.0 * 1024.0)
            );

            // Generate suggested layer splits across nodes
            let layers_per_node = num_layers.div_ceil(min_nodes as usize);
            let mut splits = Vec::new();
            let mut start = 0;
            for _ in 0..min_nodes {
                let end = std::cmp::min(start + layers_per_node - 1, num_layers.saturating_sub(1));
                if start <= end {
                    splits.push((start, end));
                    start = end + 1;
                }
            }

            CompatibilityVerdict::RequiresClusterSplit {
                min_nodes_needed: min_nodes,
                bottleneck_reason: bottleneck,
                suggested_layer_splits: splits,
            }
        }
    }

    /// Computes safe available memory reserving appropriate OS headroom.
    #[must_use]
    fn compute_safe_available_memory(caps: &DeviceCapabilities) -> u64 {
        if caps.is_unified_memory {
            // Mobile (Android / iOS): reserve >= 1.5 GiB
            if caps.os_platform == OsPlatform::Android || caps.os_platform == OsPlatform::Ios {
                let reserve = 1536 * 1024 * 1024u64; // 1.5 GiB
                caps.available_ram_bytes.saturating_sub(reserve)
            } else {
                // macOS Apple Silicon: reserve 15%
                let reserve = (caps.total_ram_bytes * 15) / 100;
                caps.available_ram_bytes.saturating_sub(reserve)
            }
        } else {
            // Discrete GPU desktop: evaluate dedicated VRAM (reserve 15%)
            let vram_reserve = (caps.total_vram_bytes * 15) / 100;
            let safe_vram = caps.available_vram_bytes.saturating_sub(vram_reserve);

            let ram_reserve = (caps.total_ram_bytes * 15) / 100;
            let safe_ram = caps.available_ram_bytes.saturating_sub(ram_reserve);

            // Model can use GPU VRAM or spill to host RAM
            safe_vram.max(safe_ram)
        }
    }
}
