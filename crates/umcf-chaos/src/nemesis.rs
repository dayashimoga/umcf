//! Composable Chaos Nemesis Injectors driven by seedable deterministic PRNG.

use std::collections::{HashMap, HashSet};
use crate::error::ChaosError;

/// Deterministic 64-bit PRNG for 100% reproducible chaos schedules.
#[derive(Debug, Clone)]
pub struct ChaosRng {
    state: u64,
}

impl ChaosRng {
    pub fn new(seed: u64) -> Self {
        let mut rng = Self { state: seed };
        if rng.state == 0 {
            rng.state = 0x853c_49e6_748f_aa9b;
        }
        // Warm up state
        rng.next_u64();
        rng
    }

    /// SplitMix64 step
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    pub fn gen_range(&mut self, min: u64, max: u64) -> u64 {
        if min >= max {
            return min;
        }
        min + (self.next_u64() % (max - min))
    }

    pub fn gen_bool(&mut self, probability_percent: u8) -> bool {
        let val = self.next_u64() % 100;
        val < probability_percent as u64
    }

    pub fn choose<'a, T>(&mut self, slice: &'a [T]) -> Option<&'a T> {
        if slice.is_empty() {
            None
        } else {
            let idx = (self.next_u64() as usize) % slice.len();
            Some(&slice[idx])
        }
    }
}

/// Routing Matrix tracking directed link connectivity between nodes.
#[derive(Debug, Clone)]
pub struct RoutingMatrix {
    /// (src, dst) -> can_reach
    links: HashMap<(String, String), bool>,
}

impl RoutingMatrix {
    pub fn new_all_connected(nodes: &[String]) -> Self {
        let mut links = HashMap::new();
        for a in nodes {
            for b in nodes {
                links.insert((a.clone(), b.clone()), true);
            }
        }
        Self { links }
    }

    pub fn can_route(&self, src: &str, dst: &str) -> bool {
        if src == dst {
            return true;
        }
        self.links.get(&(src.to_string(), dst.to_string())).copied().unwrap_or(false)
    }

    pub fn set_link(&mut self, src: &str, dst: &str, connected: bool) {
        self.links.insert((src.to_string(), dst.to_string()), connected);
    }

    pub fn disconnect_bidirectional(&mut self, node_a: &str, node_b: &str) {
        self.set_link(node_a, node_b, false);
        self.set_link(node_b, node_a, false);
    }

    pub fn heal_all(&mut self, nodes: &[String]) {
        for a in nodes {
            for b in nodes {
                self.links.insert((a.clone(), b.clone()), true);
            }
        }
    }
}

/// 1. Network Partition Nemesis.
#[derive(Debug, Clone)]
pub struct NetworkPartitionNemesis {
    pub routing: RoutingMatrix,
    pub active_splits: Vec<String>,
}

impl NetworkPartitionNemesis {
    pub fn new(nodes: &[String]) -> Self {
        Self {
            routing: RoutingMatrix::new_all_connected(nodes),
            active_splits: Vec::new(),
        }
    }

    /// Injects a symmetric partition: group A and group B cannot communicate with each other.
    pub fn inject_symmetric_split(&mut self, group_a: &[String], group_b: &[String]) {
        for a in group_a {
            for b in group_b {
                self.routing.disconnect_bidirectional(a, b);
            }
        }
        self.active_splits.push(format!("SymmetricSplit({:?} | {:?})", group_a, group_b));
    }

    /// Injects an asymmetric/bridge partition: A can reach B, B can reach C, but A cannot reach C.
    pub fn inject_asymmetric_bridge(&mut self, a: &str, b: &str, c: &str) {
        self.routing.set_link(a, b, true);
        self.routing.set_link(b, a, true);
        self.routing.set_link(b, c, true);
        self.routing.set_link(c, b, true);
        // Break direct link between A and C
        self.routing.disconnect_bidirectional(a, c);
        self.active_splits.push(format!("AsymmetricBridge({a} <-> {b} <-> {c}, {a} -/- {c})"));
    }

    /// Simulates link flapping (rapid alternating disconnect/reconnect).
    pub fn simulate_link_flapping(&mut self, a: &str, b: &str, cycles: usize) -> Vec<(bool, u64)> {
        let mut trace = Vec::new();
        for i in 0..cycles {
            let connected = i % 2 == 0;
            self.routing.set_link(a, b, connected);
            self.routing.set_link(b, a, connected);
            trace.push((connected, 500)); // 500 ms per phase
        }
        // Restore connection at the end of flapping
        self.routing.set_link(a, b, true);
        self.routing.set_link(b, a, true);
        trace
    }

    /// Heals all active partitions.
    pub fn heal_all(&mut self, nodes: &[String]) {
        self.routing.heal_all(nodes);
        self.active_splits.clear();
    }
}

/// 2. Packet Impairment Nemesis.
#[derive(Debug, Clone)]
pub struct PacketImpairmentNemesis {
    pub loss_rate_percent: u8,
    pub rtt_spike_ms: u64,
    pub bandwidth_throttle_kbps: u64,
}

impl PacketImpairmentNemesis {
    pub fn new(loss_rate_percent: u8, rtt_spike_ms: u64, bandwidth_throttle_kbps: u64) -> Self {
        Self {
            loss_rate_percent,
            rtt_spike_ms,
            bandwidth_throttle_kbps,
        }
    }

    /// Evaluates if a packet survives loss and computes simulated transit delay.
    pub fn evaluate_transmission(&self, rng: &mut ChaosRng, payload_bytes: usize) -> (bool, u64) {
        let lost = rng.gen_bool(self.loss_rate_percent);
        if lost {
            return (false, 0);
        }

        let base_rtt = rng.gen_range(5, 20);
        let spike = if rng.gen_bool(20) { self.rtt_spike_ms } else { 0 };
        let serialization_ms = (payload_bytes as u64 * 8)
            .checked_div(self.bandwidth_throttle_kbps)
            .unwrap_or(0);

        (true, base_rtt + spike + serialization_ms)
    }
}

/// Node state in cluster churn simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeLifecycleState {
    Active,
    KilledSigkill,
    SuspendedDoze,
    Restarting,
}

/// 3. Node Churn Nemesis.
#[derive(Debug, Clone)]
pub struct NodeChurnNemesis {
    pub node_states: HashMap<String, NodeLifecycleState>,
    pub suspended_deadlines_unix_ms: HashMap<String, u64>,
}

impl NodeChurnNemesis {
    pub fn new(nodes: &[String]) -> Self {
        let mut states = HashMap::new();
        for n in nodes {
            states.insert(n.clone(), NodeLifecycleState::Active);
        }
        Self {
            node_states: states,
            suspended_deadlines_unix_ms: HashMap::new(),
        }
    }

    /// Hard kills a node immediately with zero TCP teardown.
    pub fn sigkill_node(&mut self, node_id: &str) {
        self.node_states.insert(node_id.to_string(), NodeLifecycleState::KilledSigkill);
    }

    /// Simulates mobile OS App Nap or Android Doze freeze for a specified duration.
    pub fn suspend_mobile_app_nap(&mut self, node_id: &str, duration_ms: u64, now_ms: u64) {
        self.node_states.insert(node_id.to_string(), NodeLifecycleState::SuspendedDoze);
        self.suspended_deadlines_unix_ms.insert(node_id.to_string(), now_ms + duration_ms);
    }

    /// Reboots or unsuspends a node back to Active state.
    pub fn restart_or_resume_node(&mut self, node_id: &str) {
        self.node_states.insert(node_id.to_string(), NodeLifecycleState::Active);
        self.suspended_deadlines_unix_ms.remove(node_id);
    }

    pub fn is_node_responding(&self, node_id: &str, now_ms: u64) -> bool {
        match self.node_states.get(node_id) {
            Some(NodeLifecycleState::Active) => true,
            Some(NodeLifecycleState::SuspendedDoze) => {
                if let Some(deadline) = self.suspended_deadlines_unix_ms.get(node_id) {
                    now_ms >= *deadline
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

/// 4. Thermal & Battery Storm Nemesis.
#[derive(Debug, Clone)]
pub struct ThermalAndBatteryStormNemesis {
    pub critical_nodes: HashSet<String>,
    pub low_battery_nodes: HashSet<String>,
}

impl ThermalAndBatteryStormNemesis {
    pub fn new() -> Self {
        Self {
            critical_nodes: HashSet::new(),
            low_battery_nodes: HashSet::new(),
        }
    }

    pub fn trigger_thermal_critical(&mut self, node_id: &str) {
        self.critical_nodes.insert(node_id.to_string());
    }

    pub fn trigger_low_battery_5_percent(&mut self, node_id: &str) {
        self.low_battery_nodes.insert(node_id.to_string());
    }

    pub fn clear(&mut self, node_id: &str) {
        self.critical_nodes.remove(node_id);
        self.low_battery_nodes.remove(node_id);
    }

    pub fn must_evict_or_throttle(&self, node_id: &str) -> bool {
        self.critical_nodes.contains(node_id) || self.low_battery_nodes.contains(node_id)
    }
}

impl Default for ThermalAndBatteryStormNemesis {
    fn default() -> Self {
        Self::new()
    }
}

/// 5. Memory Pressure & OOM Nemesis.
#[derive(Debug, Clone)]
pub struct MemoryPressureOomNemesis {
    pub memory_critical_nodes: HashSet<String>,
    pub evicted_cache_bytes: HashMap<String, u64>,
}

impl MemoryPressureOomNemesis {
    pub fn new() -> Self {
        Self {
            memory_critical_nodes: HashSet::new(),
            evicted_cache_bytes: HashMap::new(),
        }
    }

    /// Simulates Android `onTrimMemory(TRIM_MEMORY_RUNNING_CRITICAL)`.
    /// Immediately triggers unpinned chunk cache shedding to avoid OS SIGKILL.
    pub fn simulate_trim_memory_critical(&mut self, node_id: &str, unpinned_bytes: u64) -> Result<u64, ChaosError> {
        self.memory_critical_nodes.insert(node_id.to_string());
        *self.evicted_cache_bytes.entry(node_id.to_string()).or_insert(0) += unpinned_bytes;
        Ok(unpinned_bytes)
    }

    pub fn is_memory_critical(&self, node_id: &str) -> bool {
        self.memory_critical_nodes.contains(node_id)
    }

    pub fn clear(&mut self, node_id: &str) {
        self.memory_critical_nodes.remove(node_id);
    }
}

impl Default for MemoryPressureOomNemesis {
    fn default() -> Self {
        Self::new()
    }
}
