//! `umcf-scheduler::cost`: Roofline execution model, network latency/bandwidth transfer accounting, and reliability penalties.
//! Enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use umcf_core::{ComputeBackend, NodeId, NodeRole};
use umcf_guard::GuardRegime;

use crate::dag::TaskNode;

/// Network interconnect characteristics between two nodes.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LinkProfile {
    pub rtt_ms: f64,
    pub bandwidth_mbps: f64,
    pub is_relayed: bool,
    pub is_metered: bool,
}

impl Default for LinkProfile {
    fn default() -> Self {
        // Default loopback / direct shared bus
        Self {
            rtt_ms: 0.1,
            bandwidth_mbps: 10_000.0, // 10 Gbps loopback / bus
            is_relayed: false,
            is_metered: false,
        }
    }
}

impl LinkProfile {
    #[must_use]
    pub const fn local() -> Self {
        Self {
            rtt_ms: 0.0,
            bandwidth_mbps: 50_000.0, // 50 Gbps shared local RAM/bus
            is_relayed: false,
            is_metered: false,
        }
    }

    #[must_use]
    pub const fn lan_wifi6() -> Self {
        Self {
            rtt_ms: 4.0,
            bandwidth_mbps: 600.0,
            is_relayed: false,
            is_metered: false,
        }
    }

    #[must_use]
    pub const fn lan_ethernet() -> Self {
        Self {
            rtt_ms: 1.0,
            bandwidth_mbps: 1_000.0,
            is_relayed: false,
            is_metered: false,
        }
    }

    #[must_use]
    pub const fn wan_broadband() -> Self {
        Self {
            rtt_ms: 35.0,
            bandwidth_mbps: 100.0,
            is_relayed: false,
            is_metered: false,
        }
    }

    #[must_use]
    pub const fn wan_slow_mobile() -> Self {
        Self {
            rtt_ms: 120.0,
            bandwidth_mbps: 15.0,
            is_relayed: true,
            is_metered: true,
        }
    }
}

/// Dynamic snapshot profile of a candidate node in the cluster.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NodeCandidateProfile {
    pub node_id: NodeId,
    pub role: NodeRole,
    pub available_ram_bytes: u64,
    pub available_vram_bytes: u64,
    pub effective_gflops: f64,
    pub memory_bandwidth_bytes_per_sec: f64,
    pub thermal_regime: GuardRegime,
    pub battery_percent: Option<u8>,
    pub is_charging: bool,
    pub supported_backends: Vec<ComputeBackend>,
    pub supported_simd: Vec<String>,
    pub link_to_coordinator: LinkProfile,
    pub link_to_nodes: HashMap<NodeId, LinkProfile>,
}

impl NodeCandidateProfile {
    #[must_use]
    pub fn get_link_to(&self, other_node_id: &NodeId) -> LinkProfile {
        if self.node_id == *other_node_id {
            LinkProfile::local()
        } else if let Some(link) = self.link_to_nodes.get(other_node_id) {
            link.clone()
        } else {
            self.link_to_coordinator.clone()
        }
    }
}

/// Detailed breakdown of estimated execution, transfer, and reliability costs.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CostBreakdown {
    pub compute_time_ms: f64,
    pub memory_time_ms: f64,
    pub roofline_execution_ms: f64,
    pub transfer_time_ms: f64,
    pub reliability_penalty_ms: f64,
    pub total_cost_score: f64,
}

/// Roofline execution cost model evaluator.
#[derive(Debug)]
pub struct RooflineCostModel;

impl RooflineCostModel {
    /// Estimates execution cost for a task placed on a candidate node.
    /// Combines:
    /// - Compute time: $T_{\text{compute}} = \text{FLOPs} / \text{GFLOPS}$
    /// - Memory time: $T_{\text{mem}} = \text{Bytes} / \text{Bandwidth}$
    /// - Roofline execution time: $\max(T_{\text{compute}}, T_{\text{mem}})$
    /// - Data transfer time: $\text{Dependency Bytes} / \text{Link Bandwidth} + \text{RTT}$
    /// - Reliability penalty (battery, throttling, metered WAN)
    #[must_use]
    pub fn estimate_cost(
        task: &TaskNode,
        node: &NodeCandidateProfile,
        is_local: bool,
        input_transfer_bytes: u64,
        link: &LinkProfile,
    ) -> CostBreakdown {
        // 1. Compute time: (FLOPs / (GFLOPS * 10^9)) * 1000.0 ms
        let gflops = node.effective_gflops.max(0.001);
        let compute_time_ms = (task.spec.estimated_flops as f64 / (gflops * 1_000_000_000.0)) * 1000.0;

        // 2. Memory time: (Bytes / Bandwidth_bytes_per_sec) * 1000.0 ms
        let mem_bw = node.memory_bandwidth_bytes_per_sec.max(1.0);
        let memory_time_ms = (task.spec.memory_bytes as f64 / mem_bw) * 1000.0;

        // 3. Roofline bound: execution is bounded by the slower of compute or memory
        let roofline_execution_ms = compute_time_ms.max(memory_time_ms);

        // 4. Data transfer time across network interconnect
        let transfer_time_ms = if is_local || input_transfer_bytes == 0 {
            0.0
        } else {
            let bw_bytes_sec = (link.bandwidth_mbps * 1_000_000.0) / 8.0;
            let transfer_dur_ms = (input_transfer_bytes as f64 / bw_bytes_sec.max(1.0)) * 1000.0;
            let total_xfer = transfer_dur_ms + link.rtt_ms;
            if link.is_relayed {
                total_xfer * 1.5 // 50% relay hop penalty
            } else {
                total_xfer
            }
        };

        // 5. Reliability penalty: mobile battery level, thermal throttling, metered connection
        let mut reliability_penalty_ms = 0.0;

        // Mobile on battery penalty
        if !node.is_charging {
            if let Some(bat) = node.battery_percent {
                if bat < 50 {
                    let deficit = (50 - bat) as f64 / 50.0;
                    reliability_penalty_ms += 30.0 * deficit;
                }
            }
        }

        // Thermal regime penalty
        if node.thermal_regime == GuardRegime::Throttled50 {
            reliability_penalty_ms += 50.0;
        }

        // Metered link penalty
        if link.is_metered {
            reliability_penalty_ms += 25.0;
        }

        let total_cost_score = roofline_execution_ms + transfer_time_ms + reliability_penalty_ms;

        CostBreakdown {
            compute_time_ms,
            memory_time_ms,
            roofline_execution_ms,
            transfer_time_ms,
            reliability_penalty_ms,
            total_cost_score,
        }
    }
}
