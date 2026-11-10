# Subphase S5.P3.SP1 Execution Prompt: Memory-Aware Layer Partitioning Across Heterogeneous Nodes

## 1. Role & Context
You are the Distributed Scheduling & Graph Partitioning Engineer for UMCF. In Subphase `S5.P3.SP1`, you will implement `DistributedLayerPartitioner` in `crates/umcf-ai/src/partitioner.rs`. Given a model's `MemoryFootprintReport` (per-layer weight bytes + KV-cache bytes) and a set of candidate cluster nodes with heterogeneous free RAM/VRAM, compute throughput, network link latencies, and thermal/battery states, the partitioner computes an optimal contiguous pipeline assignment of transformer blocks (`blk.0 .. blk.L-1`) that minimizes end-to-end token latency while strictly respecting per-node memory ceilings.

## 2. Prerequisites & Dependencies
- `S5.P1.SP2` (`ModelFootprintEstimator`, `LayerWeightProfile`).
- `crates/umcf-scheduler` & `crates/umcf-hw` (`ClusterTopologySnapshot`, `NodeCapabilityScore`, link RTT/bandwidth matrix).

## 3. Implementation Tasks
1. **Define Partitioning Domain Model (`crates/umcf-ai/src/partitioner.rs`)**:
   - Define `CandidateComputeNode`:
     ```rust
     pub struct CandidateComputeNode {
         pub node_id: NodeId,
         pub usable_memory_bytes: u64, // After OS / mobile LMK safety reserve
         pub estimated_layer_compute_ms: f64, // Per-layer forward pass time
         pub memory_bandwidth_gbps: f64,
         pub is_on_battery: bool,
         pub battery_percent: u8,
         pub thermal_state: ThermalState,
         pub trust_tier: TrustTier,
     }
     ```
   - Define `LayerPartitionStage`:
     ```rust
     pub struct LayerPartitionStage {
         pub stage_index: usize,
         pub node_id: NodeId,
         pub has_embeddings: bool,
         pub start_layer: u32, // inclusive
         pub end_layer: u32,   // inclusive
         pub has_output_head: bool,
         pub required_weights_bytes: u64,
         pub required_kv_bytes: u64,
         pub required_scratch_bytes: u64,
     }
     ```
   - Define `DistributedPartitionPlan`: ordered `Vec<LayerPartitionStage>`, estimated TTFT (ms), estimated decode tokens/sec, and standby failover node assignments.
2. **Implement Dynamic Programming / Min-Max Pipeline Solver**:
   - Prefer **single-node execution** whenever a single trusted node can hold 100% of weights + KV cache + scratch buffer without exceeding its `usable_memory_bytes` (avoiding network hop overhead unless multi-node prefill speedup outweighs RTT).
   - When multi-node partitioning is required (or explicitly requested), solve for contiguous layer ranges $[l_0..l_1], [l_1+1..l_2], \dots, [l_{k-1}+1..L-1]$ across an ordered ring of $k$ nodes using Dynamic Programming (`solve_optimal_pipeline_partition`):
     - **Hard Constraint 1 (Memory Ceiling)**: For every stage $i$, $\text{weights}(i) + \text{kv}(i) + \text{scratch}(i) \le \text{usable\_memory}(node_i)$.
     - **Hard Constraint 2 (Battery/Thermal Exclusion)**: Exclude mobile nodes with `battery_percent < 20%` (when unplugged) or `thermal_state >= Severe`.
     - **Objective Function**: Minimize per-token pipeline cycle time:
       $$T_{\text{token}} = \sum_{i=1}^{k} \left( n_{\text{layers}, i} \cdot t_{\text{layer}}(node_i) + \text{RTT}(node_i, node_{(i \bmod k)+1}) + \frac{\text{ActivationBytes}}{\text{BW}(node_i, node_{(i \bmod k)+1})} \right)$$
3. **Dynamic Re-Partitioning Trigger (`RebalanceDetector`)**:
   - Monitor live cluster telemetry; if an active worker node's thermal state degrades or latency spikes > 3x baseline, compute a hot-swap migration delta (`PartitionMigrationPlan`) for the coordinator.
4. **UI Integration**:
   - Flutter `DistributedLayerTopologyWidget` & Svelte 5 `LayerPartitionVisualizer.svelte`: Interactive visual ring showing each device (Phone, Laptop, Desktop), its assigned layer range (e.g., `Layers 0–11`, `Layers 12–23`, `Layers 24–31`), VRAM/RAM fill gauge, and inter-node hop latency in ms.

## 4. Acceptance Criteria
- >= 95% line/branch coverage in `crates/umcf-ai/src/partitioner.rs`; 100% test pass rate.
- Solver computes the optimal partition plan for up to 128 layers across 16 candidate nodes in < 2 ms.
- Zero generated `LayerPartitionStage` ever exceeds a node's `usable_memory_bytes`.
- Proportional heterogeneity verified: in a cluster with a 16 GB desktop GPU (fast) and a 6 GB mobile phone (slower), the solver assigns >= 70% of layers (plus embedding and output head) to the desktop and only the overflow layers to the phone.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_single_node_preferred_when_model_fits_in_vram`
  - `test_heterogeneous_3_node_proportional_layer_assignment`
  - `test_memory_ceiling_strictly_enforced_with_unequal_embedding_and_head_weights`
  - `test_low_battery_and_thermally_throttled_mobile_nodes_excluded`
  - `test_insufficient_cluster_wide_memory_returns_actionable_deficit_error`
- **`[INT]`**:
  - Feed real `ModelFootprintReport` from `S5.P1.SP2` (Llama-3-8B & 70B profiles) and `ClusterTopologySnapshot` into `DistributedLayerPartitioner`, verifying valid non-overlapping contiguous coverage of `0..block_count` and selective shard download requests to `S5.P1.SP1`.
- **`[SIM]`**:
  - Monte Carlo simulation of 1,000 random cluster topologies (1 to 12 nodes) and model sizes verifying 100% solver determinism and < 2 ms execution time.
- **`[HW]`**:
  - Verify live partition plan generation across real connected devices in a 2+ node cluster.
- **`[MANUAL]`**:
  - Inspect `DistributedLayerTopologyWidget` in Flutter & Svelte 5 and verify layer badges and memory gauges match the active `DistributedPartitionPlan`.

## 6. Required Evidence & Artifacts
- `artifacts/stage-05/p3/sp1_partition_solver_benchmarks.json`
- `artifacts/stage-05/p3/sp1_heterogeneous_topology_plans.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document the DP layer partitioning formulation and cost model in `docs/architecture/AI_INFERENCE_ENGINE.md` (Section 4).
- Mark `S5.P3.SP1` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If any partition plan omits a layer, duplicates a layer, or assigns more bytes than a node's safe usable memory limit.
- **Completion Gate**: All tests green with >= 95% coverage before proceeding to `S5.P3.SP2`.
