//! Heterogeneous capability-aware workload scheduler and distributed AI inference layer partitioner.
//! Never assumes transparent memory/GPU pooling across nodes (`ENGINEERING_CONTRACT.md` §1.1.2).

use std::fmt;
use umcf_coordinator::RegisteredNode;
use umcf_core::{ComputeBackend, NodeId, NodeRole};

/// Privacy policy for AI inference tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferencePrivacyMode {
    /// Prompt and activations must never leave the local coordinator node.
    PrivateLocalOnly,
    /// Layers may be partitioned across enrolled, attested cluster workers over mTLS P2P QUIC.
    ClusterDistributed,
}

/// Request to partition and schedule a `llama.cpp` GGUF model across available nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributedInferenceRequest {
    pub model_name: String,
    pub total_model_weights_mb: u64,
    pub kv_cache_overhead_mb: u64,
    pub total_layers: u32,
    pub preferred_backend: Option<ComputeBackend>,
    pub privacy_mode: InferencePrivacyMode,
}

/// A contiguous slice of transformer layers assigned to a single node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerStageAssignment {
    pub stage_index: usize,
    pub node_id: NodeId,
    pub layer_start: u32,
    pub layer_end_inclusive: u32,
    pub allocated_memory_mb: u64,
    pub selected_backend: ComputeBackend,
}

/// Resulting pipeline execution plan across 1 or more nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributedInferencePlan {
    pub model_name: String,
    pub total_layers: u32,
    pub privacy_mode: InferencePrivacyMode,
    pub stages: Vec<LayerStageAssignment>,
}

/// Capability-aware scheduler for UMCF.
#[derive(Debug, Default)]
pub struct HeterogeneousScheduler;

impl HeterogeneousScheduler {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Computes a deterministic pipeline-parallel layer assignment across eligible nodes.
    /// Respects each node's `ResourceConsentPolicy`, role, and verified hardware accelerators.
    pub fn plan_distributed_inference(
        &self,
        req: &DistributedInferenceRequest,
        local_node_id: &NodeId,
        active_nodes: &[&RegisteredNode],
    ) -> Result<DistributedInferencePlan, SchedulerError> {
        if req.total_layers == 0 {
            return Err(SchedulerError::InvalidRequest("total_layers must be > 0"));
        }
        let total_required_mb = req
            .total_model_weights_mb
            .saturating_add(req.kv_cache_overhead_mb);
        if total_required_mb == 0 {
            return Err(SchedulerError::InvalidRequest(
                "total required memory must be > 0",
            ));
        }

        // Filter eligible nodes: must be Worker or Hybrid, have sharing_enabled, and match privacy policy.
        let mut candidates: Vec<(&RegisteredNode, u64, ComputeBackend)> = Vec::new();
        for node in active_nodes {
            if !matches!(node.role, NodeRole::Worker | NodeRole::Hybrid) {
                continue;
            }
            if !node.consent.sharing_enabled {
                continue;
            }
            if req.privacy_mode == InferencePrivacyMode::PrivateLocalOnly
                && &node.node_id != local_node_id
            {
                continue;
            }

            let usable_ram = node
                .consent
                .max_memory_mb
                .min(node.hardware.available_ram_mb);
            if usable_ram < 128 {
                continue;
            }

            // Select verified hardware accelerator if available; otherwise fall back to CpuSimd.
            let verified = node.hardware.verified_accelerators();
            let backend = if let Some(pref) = req.preferred_backend {
                verified
                    .iter()
                    .find(|a| a.backend == pref)
                    .map(|a| a.backend)
                    .or_else(|| verified.first().map(|a| a.backend))
                    .unwrap_or(ComputeBackend::CpuSimd)
            } else {
                verified
                    .first()
                    .map(|a| a.backend)
                    .unwrap_or(ComputeBackend::CpuSimd)
            };

            candidates.push((*node, usable_ram, backend));
        }

        // Sort deterministically: local node first, then highest usable memory, then NodeId.
        candidates.sort_by(|a, b| {
            let a_local = &a.0.node_id == local_node_id;
            let b_local = &b.0.node_id == local_node_id;
            b_local
                .cmp(&a_local)
                .then_with(|| b.1.cmp(&a.1))
                .then_with(|| a.0.node_id.cmp(&b.0.node_id))
        });

        let cluster_usable_mb: u64 = candidates.iter().map(|(_, mem, _)| *mem).sum();
        if cluster_usable_mb < total_required_mb {
            return Err(SchedulerError::InsufficientClusterMemory {
                required_mb: total_required_mb,
                available_mb: cluster_usable_mb,
            });
        }

        let mb_per_layer = (total_required_mb as f64 / f64::from(req.total_layers)).ceil() as u64;
        let mut remaining_layers = req.total_layers;
        let mut current_layer = 0u32;
        let mut stages = Vec::new();

        for (idx, (node, node_mem_mb, backend)) in candidates.into_iter().enumerate() {
            if remaining_layers == 0 {
                break;
            }
            let max_layers_on_node = u32::try_from((node_mem_mb / mb_per_layer.max(1)).max(1))
                .unwrap_or(remaining_layers);
            let assigned_layers = remaining_layers.min(max_layers_on_node);
            let allocated_mb = u64::from(assigned_layers)
                .saturating_mul(mb_per_layer)
                .min(node_mem_mb);

            stages.push(LayerStageAssignment {
                stage_index: idx,
                node_id: node.node_id.clone(),
                layer_start: current_layer,
                layer_end_inclusive: current_layer + assigned_layers - 1,
                allocated_memory_mb: allocated_mb,
                selected_backend: backend,
            });

            current_layer += assigned_layers;
            remaining_layers -= assigned_layers;
        }

        if remaining_layers > 0 {
            return Err(SchedulerError::InsufficientClusterMemory {
                required_mb: total_required_mb,
                available_mb: cluster_usable_mb,
            });
        }

        Ok(DistributedInferencePlan {
            model_name: req.model_name.clone(),
            total_layers: req.total_layers,
            privacy_mode: req.privacy_mode,
            stages,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulerError {
    InvalidRequest(&'static str),
    InsufficientClusterMemory { required_mb: u64, available_mb: u64 },
}

impl fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(msg) => write!(f, "invalid scheduler request: {msg}"),
            Self::InsufficientClusterMemory {
                required_mb,
                available_mb,
            } => write!(
                f,
                "insufficient consented cluster memory: required {required_mb} MiB, available {available_mb} MiB"
            ),
        }
    }
}

impl std::error::Error for SchedulerError {}

#[cfg(test)]
mod tests {
    use super::*;
    use umcf_core::{
        AcceleratorCapability, AcceleratorKind, CpuArch, HardwareProfile, PlatformKind,
        ResourceConsentPolicy,
    };

    fn make_node(seed: &[u8], ram_mb: u64, verified_vulkan: bool) -> RegisteredNode {
        let node_id = NodeId::from_public_key_bytes(seed).expect("id");
        let consent = ResourceConsentPolicy {
            sharing_enabled: true,
            max_memory_mb: ram_mb,
            ..ResourceConsentPolicy::default()
        };

        let accelerators = if verified_vulkan {
            vec![AcceleratorCapability {
                name: "GPU-Vulkan".into(),
                kind: AcceleratorKind::DiscreteGpu,
                backend: ComputeBackend::Vulkan,
                vram_bytes: umcf_core::ByteSize::from_mib(ram_mb),
                dedicated_memory_mb: ram_mb,
                shared_memory_mb: 0,
                compute_units: 32,
                fp32_tflops: 10.0,
                fp16_tflops: umcf_core::TeraFlopsFp16::from_tflops(20),
                int8_tops: 40,
                supports_fp16: true,
                supports_int8: true,
                supports_int4: true,
                verified_on_hardware: true,
            }]
        } else {
            vec![]
        };

        RegisteredNode {
            node_id: node_id.clone(),
            role: NodeRole::Hybrid,
            hardware: HardwareProfile {
                node_id,
                platform: PlatformKind::Linux,
                cpu_arch: CpuArch::X86_64,
                cpu_model: "CPU".into(),
                physical_cores: 8,
                logical_cores: 16,
                total_ram_mb: ram_mb * 2,
                available_ram_mb: ram_mb,
                available_storage_mb: 50_000,
                network_bandwidth_mbps: 1000,
                accelerators,
            },
            consent,
            last_heartbeat_unix_secs: 100,
            lease_expires_unix_secs: 200,
        }
    }

    #[test]
    fn simulation_heterogeneous_layer_partitioning_across_nodes() {
        let n1 = make_node(b"node-1-public-key-bytes-00001", 4096, true);
        let n2 = make_node(b"node-2-public-key-bytes-00002", 4096, false);
        let nodes = vec![&n1, &n2];

        let scheduler = HeterogeneousScheduler::new();
        let req = DistributedInferenceRequest {
            model_name: "Llama-3-8B-Q4_K_M.gguf".into(),
            total_model_weights_mb: 5120,
            kv_cache_overhead_mb: 1024,
            total_layers: 32,
            preferred_backend: Some(ComputeBackend::Vulkan),
            privacy_mode: InferencePrivacyMode::ClusterDistributed,
        };

        let plan = scheduler
            .plan_distributed_inference(&req, &n1.node_id, &nodes)
            .expect("distributed plan succeeds");

        assert_eq!(plan.stages.len(), 2);
        assert_eq!(plan.stages[0].node_id, n1.node_id);
        assert_eq!(plan.stages[0].selected_backend, ComputeBackend::Vulkan);
        assert_eq!(plan.stages[0].layer_start, 0);
        assert_eq!(plan.stages[1].node_id, n2.node_id);
        assert_eq!(plan.stages[1].selected_backend, ComputeBackend::CpuSimd);
        assert_eq!(plan.stages[1].layer_end_inclusive, 31);
    }

    #[test]
    fn unit_private_local_only_mode_blocks_remote_workers() {
        let n1 = make_node(b"node-1-public-key-bytes-00001", 2048, false);
        let n2 = make_node(b"node-2-public-key-bytes-00002", 8192, true);
        let nodes = vec![&n1, &n2];

        let scheduler = HeterogeneousScheduler::new();
        let req = DistributedInferenceRequest {
            model_name: "Private-Model-7B.gguf".into(),
            total_model_weights_mb: 4096,
            kv_cache_overhead_mb: 512,
            total_layers: 32,
            preferred_backend: None,
            privacy_mode: InferencePrivacyMode::PrivateLocalOnly,
        };

        // Local node n1 only has 2048 MB and PrivateLocalOnly forbids using remote node n2
        let res = scheduler.plan_distributed_inference(&req, &n1.node_id, &nodes);
        assert!(matches!(
            res,
            Err(SchedulerError::InsufficientClusterMemory { .. })
        ));
    }
}
