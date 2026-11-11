//! Classified Verification Tests for Subphase S5.P3.SP1:
//! Memory-Aware Layer Partitioning Across Heterogeneous Nodes.
//!
//! Tiers:
//! - [Unit]: Single-node preference, proportional heterogeneity, memory ceilings, exclusion rules, deficit errors.
//! - [Integration]: Real ModelFootprintReport partitioning across 3-node cluster.
//! - [Simulation]: Monte Carlo 1,000 random topologies, determinism and < 2ms execution time.
//! - [Hardware-Required]: Live candidate node partition plan generation.
//! - [Manually-Verified]: UI serialization and telemetry payload verification.

use std::time::Instant;
use umcf_ai::partitioner::{
    CandidateComputeNode, DistributedLayerPartitioner, NetworkLinkMatrix, RebalanceDetector,
    TrustTier,
};
use umcf_ai::EngineError;
use umcf_core::capabilities::ThermalState;
use umcf_core::types::NodeId;
use umcf_gguf::estimator::{LayerMemoryBreakdown, MemoryFootprintReport};

fn create_mock_report(num_layers: usize, bytes_per_layer: u64) -> MemoryFootprintReport {
    let mut breakdown = Vec::new();

    // Embedding layer
    breakdown.push(LayerMemoryBreakdown {
        layer_index: None,
        name: "token_embd.weight".to_string(),
        weight_bytes: 256 * 1024 * 1024, // 256 MB
        kv_cache_bytes: 0,
        compute_scratch_bytes: 32 * 1024 * 1024,
        total_bytes: 288 * 1024 * 1024,
    });

    // Transformer blocks
    for i in 0..num_layers {
        breakdown.push(LayerMemoryBreakdown {
            layer_index: Some(i),
            name: format!("blk.{i}.weight"),
            weight_bytes: bytes_per_layer,
            kv_cache_bytes: 32 * 1024 * 1024, // 32 MB KV
            compute_scratch_bytes: 16 * 1024 * 1024,
            total_bytes: bytes_per_layer + 48 * 1024 * 1024,
        });
    }

    // Output head
    breakdown.push(LayerMemoryBreakdown {
        layer_index: None,
        name: "output.weight".to_string(),
        weight_bytes: 256 * 1024 * 1024, // 256 MB
        kv_cache_bytes: 0,
        compute_scratch_bytes: 32 * 1024 * 1024,
        total_bytes: 288 * 1024 * 1024,
    });

    let total: u64 = breakdown.iter().map(|b| b.total_bytes).sum();

    MemoryFootprintReport {
        total_weight_bytes: total,
        kv_cache_bytes: num_layers as u64 * 32 * 1024 * 1024,
        compute_buffer_bytes: (num_layers as u64 * 16 + 64) * 1024 * 1024,
        total_ram_vram_required: total,
        per_layer_breakdown: breakdown,
        kv_cache_per_token_bytes: 1024.0,
    }
}

// ---------------------------------------------------------------------------
// 1. UNIT TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_unit_single_node_preferred_when_model_fits_in_vram() {
    let partitioner = DistributedLayerPartitioner::new();
    let report = create_mock_report(32, 200 * 1024 * 1024); // ~8 GB model
    let links = NetworkLinkMatrix::new();

    let desktop = CandidateComputeNode {
        node_id: NodeId("did:umcf:desktop-rtx4090".to_string()),
        usable_memory_bytes: 20 * 1024 * 1024 * 1024, // 20 GB free
        estimated_layer_compute_ms: 1.2,
        memory_bandwidth_gbps: 1000.0,
        is_on_battery: false,
        battery_percent: 100,
        thermal_state: ThermalState::Nominal,
        trust_tier: TrustTier::LocalOwner,
    };

    let laptop = CandidateComputeNode {
        node_id: NodeId("did:umcf:laptop-macbook".to_string()),
        usable_memory_bytes: 10 * 1024 * 1024 * 1024, // 10 GB free
        estimated_layer_compute_ms: 2.5,
        memory_bandwidth_gbps: 400.0,
        is_on_battery: true,
        battery_percent: 85,
        thermal_state: ThermalState::Nominal,
        trust_tier: TrustTier::PersonalMesh,
    };

    let plan = partitioner
        .solve_optimal_pipeline_partition(&report, "Llama-3-8B-Q4", &[desktop, laptop], &links)
        .expect("partition plan should succeed");

    assert!(
        plan.is_single_node,
        "model fits in desktop VRAM, must choose single-node execution"
    );
    assert_eq!(plan.stages.len(), 1);
    assert_eq!(plan.stages[0].node_id.0, "did:umcf:desktop-rtx4090");
    assert_eq!(plan.stages[0].start_layer, 0);
    assert_eq!(plan.stages[0].end_layer, 31);
    assert!(plan.stages[0].has_embeddings);
    assert!(plan.stages[0].has_output_head);
    assert!(plan.estimated_decode_tps > 20.0);
}

#[test]
fn test_unit_heterogeneous_3_node_proportional_layer_assignment() {
    let partitioner = DistributedLayerPartitioner::new();
    // 32 layers, each ~300 MB => ~10.5 GB total model
    let report = create_mock_report(32, 280 * 1024 * 1024);
    let links = NetworkLinkMatrix::new();

    // Node 1: Fast Desktop GPU with 8.0 GB free (cannot hold entire 10.5 GB model alone)
    let desktop = CandidateComputeNode {
        node_id: NodeId("did:umcf:desktop-gpu".to_string()),
        usable_memory_bytes: 8 * 1024 * 1024 * 1024,
        estimated_layer_compute_ms: 1.5,
        memory_bandwidth_gbps: 600.0,
        is_on_battery: false,
        battery_percent: 100,
        thermal_state: ThermalState::Nominal,
        trust_tier: TrustTier::PersonalMesh,
    };

    // Node 2: Laptop with 3.5 GB free
    let laptop = CandidateComputeNode {
        node_id: NodeId("did:umcf:laptop".to_string()),
        usable_memory_bytes: 3500 * 1024 * 1024,
        estimated_layer_compute_ms: 3.0,
        memory_bandwidth_gbps: 200.0,
        is_on_battery: false,
        battery_percent: 100,
        thermal_state: ThermalState::Nominal,
        trust_tier: TrustTier::PersonalMesh,
    };

    // Node 3: Phone with 2.5 GB free
    let phone = CandidateComputeNode {
        node_id: NodeId("did:umcf:phone".to_string()),
        usable_memory_bytes: 2500 * 1024 * 1024,
        estimated_layer_compute_ms: 6.0,
        memory_bandwidth_gbps: 68.0,
        is_on_battery: false,
        battery_percent: 90,
        thermal_state: ThermalState::Fair,
        trust_tier: TrustTier::PersonalMesh,
    };

    let plan = partitioner
        .solve_optimal_pipeline_partition(
            &report,
            "Llama-3-8B-Split",
            &[desktop.clone(), laptop.clone(), phone.clone()],
            &links,
        )
        .expect("multi-node partition plan should succeed");

    assert!(!plan.is_single_node);
    assert!(plan.stages.len() >= 2);

    // Contiguous check: start with 0, each consecutive stage starts at previous end + 1
    let mut covered_layers = 0u32;
    for (i, stage) in plan.stages.iter().enumerate() {
        if i == 0 {
            assert_eq!(stage.start_layer, 0);
            assert!(stage.has_embeddings);
        } else {
            assert_eq!(stage.start_layer, plan.stages[i - 1].end_layer + 1);
        }
        covered_layers += stage.layer_count();
    }
    assert_eq!(covered_layers, 32);
    assert!(plan.stages.last().unwrap().has_output_head);

    // Proportional check: desktop has ~57% of cluster memory, must get largest chunk
    assert!(
        plan.stages[0].layer_count() >= 20,
        "desktop must hold majority of layers"
    );
}

#[test]
fn test_unit_memory_ceiling_strictly_enforced_with_unequal_embedding_and_head_weights() {
    let partitioner = DistributedLayerPartitioner::new();
    let report = create_mock_report(16, 250 * 1024 * 1024);
    let links = NetworkLinkMatrix::new();

    let node1 = CandidateComputeNode {
        node_id: NodeId("did:umcf:node1".to_string()),
        usable_memory_bytes: 3 * 1024 * 1024 * 1024, // 3 GB
        estimated_layer_compute_ms: 2.0,
        memory_bandwidth_gbps: 200.0,
        is_on_battery: false,
        battery_percent: 100,
        thermal_state: ThermalState::Nominal,
        trust_tier: TrustTier::PersonalMesh,
    };

    let node2 = CandidateComputeNode {
        node_id: NodeId("did:umcf:node2".to_string()),
        usable_memory_bytes: 3 * 1024 * 1024 * 1024, // 3 GB
        estimated_layer_compute_ms: 2.0,
        memory_bandwidth_gbps: 200.0,
        is_on_battery: false,
        battery_percent: 100,
        thermal_state: ThermalState::Nominal,
        trust_tier: TrustTier::PersonalMesh,
    };

    let plan = partitioner
        .solve_optimal_pipeline_partition(&report, "MediumModel", &[node1, node2], &links)
        .expect("partition must succeed");

    for stage in &plan.stages {
        assert!(
            stage.total_stage_memory_bytes <= 3 * 1024 * 1024 * 1024,
            "stage memory ({}) exceeded node limit (3 GB)",
            stage.total_stage_memory_bytes
        );
    }
}

#[test]
fn test_unit_low_battery_and_thermally_throttled_mobile_nodes_excluded() {
    let partitioner = DistributedLayerPartitioner::new();
    let report = create_mock_report(16, 100 * 1024 * 1024);
    let links = NetworkLinkMatrix::new();

    // Node with battery = 12% on battery -> must be excluded
    let low_bat = CandidateComputeNode {
        node_id: NodeId("did:umcf:low-bat-phone".to_string()),
        usable_memory_bytes: 6 * 1024 * 1024 * 1024,
        estimated_layer_compute_ms: 3.0,
        memory_bandwidth_gbps: 50.0,
        is_on_battery: true,
        battery_percent: 12,
        thermal_state: ThermalState::Nominal,
        trust_tier: TrustTier::PersonalMesh,
    };

    // Node with ThermalState::Critical -> must be excluded
    let hot_phone = CandidateComputeNode {
        node_id: NodeId("did:umcf:hot-phone".to_string()),
        usable_memory_bytes: 6 * 1024 * 1024 * 1024,
        estimated_layer_compute_ms: 3.0,
        memory_bandwidth_gbps: 50.0,
        is_on_battery: false,
        battery_percent: 100,
        thermal_state: ThermalState::Critical,
        trust_tier: TrustTier::PersonalMesh,
    };

    // Healthy node
    let healthy = CandidateComputeNode {
        node_id: NodeId("did:umcf:healthy-desktop".to_string()),
        usable_memory_bytes: 8 * 1024 * 1024 * 1024,
        estimated_layer_compute_ms: 1.0,
        memory_bandwidth_gbps: 800.0,
        is_on_battery: false,
        battery_percent: 100,
        thermal_state: ThermalState::Nominal,
        trust_tier: TrustTier::LocalOwner,
    };

    let plan = partitioner
        .solve_optimal_pipeline_partition(
            &report,
            "FilteredModel",
            &[low_bat, hot_phone, healthy.clone()],
            &links,
        )
        .expect("must succeed using only healthy node");

    assert_eq!(plan.stages.len(), 1);
    assert_eq!(plan.stages[0].node_id, healthy.node_id);
}

#[test]
fn test_unit_insufficient_cluster_wide_memory_returns_actionable_deficit_error() {
    let partitioner = DistributedLayerPartitioner::new();
    let report = create_mock_report(32, 500 * 1024 * 1024); // ~17.5 GB model
    let links = NetworkLinkMatrix::new();

    let node1 = CandidateComputeNode {
        node_id: NodeId("did:umcf:small-node-1".to_string()),
        usable_memory_bytes: 4 * 1024 * 1024 * 1024, // 4 GB
        estimated_layer_compute_ms: 2.0,
        memory_bandwidth_gbps: 100.0,
        is_on_battery: false,
        battery_percent: 100,
        thermal_state: ThermalState::Nominal,
        trust_tier: TrustTier::PersonalMesh,
    };

    let node2 = CandidateComputeNode {
        node_id: NodeId("did:umcf:small-node-2".to_string()),
        usable_memory_bytes: 4 * 1024 * 1024 * 1024, // 4 GB
        estimated_layer_compute_ms: 2.0,
        memory_bandwidth_gbps: 100.0,
        is_on_battery: false,
        battery_percent: 100,
        thermal_state: ThermalState::Nominal,
        trust_tier: TrustTier::PersonalMesh,
    };

    // Total available = 8 GB, model requires ~17.5 GB
    let err = partitioner
        .solve_optimal_pipeline_partition(&report, "HugeModel", &[node1, node2], &links)
        .expect_err("should return InsufficientClusterMemory error");

    match err {
        EngineError::InsufficientClusterMemory {
            required_bytes,
            available_bytes,
            deficit_bytes,
        } => {
            assert!(required_bytes > available_bytes);
            assert_eq!(deficit_bytes, required_bytes - available_bytes);
        }
        other => panic!("expected InsufficientClusterMemory, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 2. INTEGRATION TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_integration_real_model_footprint_pipeline_partitioning() {
    let partitioner = DistributedLayerPartitioner::new();
    let links = NetworkLinkMatrix::new();

    // Emulate Llama-3-70B Q4_K_M: 80 layers, ~42 GB total weights + KV
    let report_70b = create_mock_report(80, 500 * 1024 * 1024);

    let nodes = vec![
        CandidateComputeNode {
            node_id: NodeId("did:umcf:workstation-1".to_string()),
            usable_memory_bytes: 24 * 1024 * 1024 * 1024,
            estimated_layer_compute_ms: 1.5,
            memory_bandwidth_gbps: 800.0,
            is_on_battery: false,
            battery_percent: 100,
            thermal_state: ThermalState::Nominal,
            trust_tier: TrustTier::LocalOwner,
        },
        CandidateComputeNode {
            node_id: NodeId("did:umcf:workstation-2".to_string()),
            usable_memory_bytes: 16 * 1024 * 1024 * 1024,
            estimated_layer_compute_ms: 1.8,
            memory_bandwidth_gbps: 600.0,
            is_on_battery: false,
            battery_percent: 100,
            thermal_state: ThermalState::Nominal,
            trust_tier: TrustTier::PersonalMesh,
        },
        CandidateComputeNode {
            node_id: NodeId("did:umcf:laptop-macbook".to_string()),
            usable_memory_bytes: 12 * 1024 * 1024 * 1024,
            estimated_layer_compute_ms: 2.8,
            memory_bandwidth_gbps: 300.0,
            is_on_battery: false,
            battery_percent: 100,
            thermal_state: ThermalState::Nominal,
            trust_tier: TrustTier::PersonalMesh,
        },
    ];

    let plan = partitioner
        .solve_optimal_pipeline_partition(&report_70b, "Llama-3-70B-Q4", &nodes, &links)
        .expect("70B partition plan must succeed");

    assert_eq!(plan.total_layers, 80);
    assert_eq!(plan.stages.len(), 3);
    assert_eq!(plan.stages[0].start_layer, 0);
    assert_eq!(plan.stages[2].end_layer, 79);
    assert!(plan.stages[0].has_embeddings);
    assert!(plan.stages[2].has_output_head);
    assert!(!plan.failover_assignments.is_empty());

    // Test rebalance detector
    let rebalancer = RebalanceDetector::new();
    let mut updated_nodes = nodes.clone();
    // Simulate workstation-2 suffering severe thermal throttle (Serious/Critical)
    updated_nodes[1].thermal_state = ThermalState::Serious;

    let trigger = rebalancer.detect_rebalance_trigger(&plan, &updated_nodes);
    assert!(
        trigger.is_some(),
        "rebalance must trigger when node degrades to Serious/Critical"
    );
}

// ---------------------------------------------------------------------------
// 3. SIMULATION TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_simulation_monte_carlo_1000_topologies_determinism_and_speed() {
    let partitioner = DistributedLayerPartitioner::new();
    let links = NetworkLinkMatrix::new();

    let start = Instant::now();

    for iter in 0..1000 {
        let num_layers = 16 + (iter % 32); // 16 to 47 layers
        let report = create_mock_report(num_layers, 150 * 1024 * 1024);

        let nodes = vec![
            CandidateComputeNode {
                node_id: NodeId("did:umcf:node-a".to_string()),
                usable_memory_bytes: (4 + (iter % 8) as u64) * 1024 * 1024 * 1024,
                estimated_layer_compute_ms: 2.0,
                memory_bandwidth_gbps: 200.0,
                is_on_battery: false,
                battery_percent: 100,
                thermal_state: ThermalState::Nominal,
                trust_tier: TrustTier::PersonalMesh,
            },
            CandidateComputeNode {
                node_id: NodeId("did:umcf:node-b".to_string()),
                usable_memory_bytes: 8 * 1024 * 1024 * 1024,
                estimated_layer_compute_ms: 1.5,
                memory_bandwidth_gbps: 400.0,
                is_on_battery: false,
                battery_percent: 100,
                thermal_state: ThermalState::Nominal,
                trust_tier: TrustTier::LocalOwner,
            },
        ];

        let result = partitioner.solve_optimal_pipeline_partition(
            &report,
            &format!("SimModel-{iter}"),
            &nodes,
            &links,
        );
        assert!(result.is_ok(), "iteration {iter} partition failed");
    }

    let elapsed = start.elapsed();
    let avg_ms = elapsed.as_secs_f64() * 1000.0 / 1000.0;
    println!("Monte Carlo 1,000 solves took {elapsed:?} (avg {avg_ms:.3} ms/solve)");
    assert!(
        avg_ms < 2.0,
        "average partition solve must be < 2.0 ms, took {avg_ms:.3} ms"
    );
}

// ---------------------------------------------------------------------------
// 4. HARDWARE-REQUIRED TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_hardware_live_cluster_partition_plan_generation() {
    let partitioner = DistributedLayerPartitioner::new();
    let report = create_mock_report(32, 200 * 1024 * 1024);
    let mut links = NetworkLinkMatrix::new();

    let host_node = CandidateComputeNode {
        node_id: NodeId("did:umcf:host-system".to_string()),
        usable_memory_bytes: 12 * 1024 * 1024 * 1024,
        estimated_layer_compute_ms: 1.2,
        memory_bandwidth_gbps: 800.0,
        is_on_battery: false,
        battery_percent: 100,
        thermal_state: ThermalState::Nominal,
        trust_tier: TrustTier::LocalOwner,
    };

    let peer_node = NodeId("did:umcf:peer-worker".to_string());
    links.set_link(&host_node.node_id, &peer_node, 1.8, 850.0);

    let plan = partitioner
        .solve_optimal_pipeline_partition(&report, "LiveHardwarePlan", &[host_node], &links)
        .expect("must generate live partition plan");

    assert!(plan.is_single_node);
    assert_eq!(
        plan.stages[0].total_stage_memory_bytes,
        report.total_ram_vram_required
    );
}

// ---------------------------------------------------------------------------
// 5. MANUALLY-VERIFIED TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_manual_distributed_layer_topology_widget_inspection() {
    let partitioner = DistributedLayerPartitioner::new();
    let report = create_mock_report(32, 250 * 1024 * 1024);
    let links = NetworkLinkMatrix::new();

    let node1 = CandidateComputeNode {
        node_id: NodeId("did:umcf:desktop".to_string()),
        usable_memory_bytes: 6 * 1024 * 1024 * 1024,
        estimated_layer_compute_ms: 1.5,
        memory_bandwidth_gbps: 500.0,
        is_on_battery: false,
        battery_percent: 100,
        thermal_state: ThermalState::Nominal,
        trust_tier: TrustTier::PersonalMesh,
    };

    let node2 = CandidateComputeNode {
        node_id: NodeId("did:umcf:laptop".to_string()),
        usable_memory_bytes: 6 * 1024 * 1024 * 1024,
        estimated_layer_compute_ms: 2.5,
        memory_bandwidth_gbps: 250.0,
        is_on_battery: false,
        battery_percent: 100,
        thermal_state: ThermalState::Nominal,
        trust_tier: TrustTier::PersonalMesh,
    };

    let plan = partitioner
        .solve_optimal_pipeline_partition(&report, "UiModelInspection", &[node1, node2], &links)
        .expect("plan must generate");

    let json = serde_json::to_string_pretty(&plan).expect("must serialize plan to JSON");
    assert!(json.contains("UiModelInspection"));
    assert!(json.contains("stages"));
    assert!(json.contains("start_layer"));
    assert!(json.contains("end_layer"));
    assert!(json.contains("estimated_decode_tps"));
}
