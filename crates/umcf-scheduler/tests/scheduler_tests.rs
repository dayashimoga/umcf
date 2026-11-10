//! Multi-tier classified verification test suite for `umcf-scheduler` Subphase `S4.P2.SP1`.
//! Enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Instant;
use umcf_core::{ComputeBackend, NodeId, NodeRole};
use umcf_guard::GuardRegime;
use umcf_scheduler::cost::{LinkProfile, NodeCandidateProfile, RooflineCostModel};
use umcf_scheduler::dag::{
    HardRequirements, JobDag, PlacementConstraint, SoftPreferences, TaskNode, TaskSpec,
    WorkloadKind,
};
use umcf_scheduler::error::{SchedulingError, UnschedulableReason};
use umcf_scheduler::solver::ConstraintSolver;

fn make_node_id(seed_byte: u8) -> NodeId {
    let mut bytes = [0u8; 32];
    bytes[0] = seed_byte;
    bytes[31] = seed_byte.wrapping_mul(7);
    NodeId::from_public_key_bytes(&bytes).expect("valid node id")
}

#[allow(clippy::too_many_arguments)]
fn create_synthetic_node(
    id_seed: u8,
    role: NodeRole,
    ram_mb: u64,
    vram_mb: u64,
    effective_gflops: f64,
    mem_bw_gbps: f64,
    thermal_regime: GuardRegime,
    battery_percent: Option<u8>,
    is_charging: bool,
    backends: Vec<ComputeBackend>,
    simd: Vec<&str>,
    link: LinkProfile,
) -> NodeCandidateProfile {
    let node_id = make_node_id(id_seed);
    NodeCandidateProfile {
        node_id,
        role,
        available_ram_bytes: ram_mb * 1024 * 1024,
        available_vram_bytes: vram_mb * 1024 * 1024,
        effective_gflops,
        memory_bandwidth_bytes_per_sec: mem_bw_gbps * 1_000_000_000.0,
        thermal_regime,
        battery_percent,
        is_charging,
        supported_backends: backends,
        supported_simd: simd.into_iter().map(String::from).collect(),
        link_to_coordinator: link,
        link_to_nodes: HashMap::new(),
    }
}

/// Generates 15 heterogeneous synthetic node profiles for comprehensive coverage testing.
fn create_15_synthetic_profiles(local_id: &NodeId) -> Vec<NodeCandidateProfile> {
    let mut profiles = Vec::with_capacity(15);

    // Node 0: Local Phone (ARM64)
    let mut local_phone = create_synthetic_node(
        1,
        NodeRole::Hybrid,
        4096,
        0,
        50.0,
        20.0,
        GuardRegime::FullCapacity,
        Some(80),
        false,
        vec![ComputeBackend::CpuSimd],
        vec!["neon", "fp16"],
        LinkProfile::local(),
    );
    local_phone.node_id = local_id.clone();
    profiles.push(local_phone);

    // Node 1: Remote Phone Low Battery (ARM64)
    profiles.push(create_synthetic_node(
        2,
        NodeRole::Worker,
        4096,
        0,
        55.0,
        22.0,
        GuardRegime::FullCapacity,
        Some(18),
        false,
        vec![ComputeBackend::CpuSimd],
        vec!["neon"],
        LinkProfile::lan_wifi6(),
    ));

    // Node 2: Remote Phone Thermally Paused (ARM64)
    profiles.push(create_synthetic_node(
        3,
        NodeRole::Worker,
        6144,
        0,
        60.0,
        25.0,
        GuardRegime::PausedWaitCoolOrCharge,
        Some(75),
        true,
        vec![ComputeBackend::CpuSimd],
        vec!["neon"],
        LinkProfile::lan_wifi6(),
    ));

    // Node 3: LAN High-End GPU Workstation (x86_64, CUDA + Vulkan, 24 GB VRAM)
    profiles.push(create_synthetic_node(
        4,
        NodeRole::Worker,
        65536,
        24576,
        15_000.0,
        800.0,
        GuardRegime::FullCapacity,
        None,
        true,
        vec![
            ComputeBackend::CpuSimd,
            ComputeBackend::Cuda,
            ComputeBackend::Vulkan,
        ],
        vec!["avx2", "avx512", "fma"],
        LinkProfile::lan_ethernet(),
    ));

    // Node 4: LAN Gaming Laptop (x86_64, Vulkan, 8 GB VRAM)
    profiles.push(create_synthetic_node(
        5,
        NodeRole::Worker,
        16384,
        8192,
        3_500.0,
        300.0,
        GuardRegime::FullCapacity,
        Some(95),
        true,
        vec![ComputeBackend::CpuSimd, ComputeBackend::Vulkan],
        vec!["avx2", "fma"],
        LinkProfile::lan_wifi6(),
    ));

    // Node 5: LAN Office Desktop (x86_64, CPU only, 16 GB RAM)
    profiles.push(create_synthetic_node(
        6,
        NodeRole::Worker,
        16384,
        0,
        200.0,
        45.0,
        GuardRegime::FullCapacity,
        None,
        true,
        vec![ComputeBackend::CpuSimd],
        vec!["avx2"],
        LinkProfile::lan_ethernet(),
    ));

    // Node 6: LAN Tablet (ARM64, CoreML, 6 GB RAM)
    profiles.push(create_synthetic_node(
        7,
        NodeRole::Worker,
        6144,
        0,
        90.0,
        30.0,
        GuardRegime::FullCapacity,
        Some(88),
        false,
        vec![ComputeBackend::CpuSimd, ComputeBackend::CoreMl],
        vec!["neon", "fp16"],
        LinkProfile::lan_wifi6(),
    ));

    // Node 7: LAN Node Thermally Critical (EmergencyEvict)
    profiles.push(create_synthetic_node(
        8,
        NodeRole::Worker,
        8192,
        4096,
        2_000.0,
        150.0,
        GuardRegime::EmergencyEvict,
        Some(90),
        true,
        vec![ComputeBackend::CpuSimd, ComputeBackend::Vulkan],
        vec!["avx2"],
        LinkProfile::lan_wifi6(),
    ));

    // Node 8: WAN Broadband Server (x86_64, CUDA, 128 GB RAM, 32 GB VRAM)
    profiles.push(create_synthetic_node(
        9,
        NodeRole::Worker,
        131072,
        32768,
        20_000.0,
        950.0,
        GuardRegime::FullCapacity,
        None,
        true,
        vec![ComputeBackend::CpuSimd, ComputeBackend::Cuda],
        vec!["avx2", "avx512"],
        LinkProfile::wan_broadband(),
    ));

    // Node 9: WAN Slow Mobile Node (ARM64, Relayed + Metered)
    profiles.push(create_synthetic_node(
        10,
        NodeRole::Worker,
        3072,
        0,
        40.0,
        15.0,
        GuardRegime::FullCapacity,
        Some(35),
        false,
        vec![ComputeBackend::CpuSimd],
        vec!["neon"],
        LinkProfile::wan_slow_mobile(),
    ));

    // Node 10: LAN Node Missing AVX2 (Old x86_64)
    profiles.push(create_synthetic_node(
        11,
        NodeRole::Worker,
        8192,
        0,
        80.0,
        25.0,
        GuardRegime::FullCapacity,
        None,
        true,
        vec![ComputeBackend::CpuSimd],
        vec!["sse4.2"],
        LinkProfile::lan_ethernet(),
    ));

    // Node 11: LAN Low-Memory Micro Node (512 MB RAM)
    profiles.push(create_synthetic_node(
        12,
        NodeRole::Worker,
        512,
        0,
        30.0,
        10.0,
        GuardRegime::FullCapacity,
        None,
        true,
        vec![ComputeBackend::CpuSimd],
        vec!["neon"],
        LinkProfile::lan_ethernet(),
    ));

    // Node 12: LAN Apple Silicon Desktop (Metal, 16 GB Unified VRAM)
    profiles.push(create_synthetic_node(
        13,
        NodeRole::Worker,
        16384,
        16384,
        5_500.0,
        250.0,
        GuardRegime::FullCapacity,
        None,
        true,
        vec![ComputeBackend::CpuSimd, ComputeBackend::Metal],
        vec!["neon", "fp16"],
        LinkProfile::lan_wifi6(),
    ));

    // Node 13: LAN Android QNN Device (NNAPI, 8 GB RAM)
    profiles.push(create_synthetic_node(
        14,
        NodeRole::Worker,
        8192,
        2048,
        1_200.0,
        60.0,
        GuardRegime::FullCapacity,
        Some(70),
        true,
        vec![
            ComputeBackend::CpuSimd,
            ComputeBackend::Nnapi,
            ComputeBackend::Vulkan,
        ],
        vec!["neon", "dotprod"],
        LinkProfile::lan_wifi6(),
    ));

    // Node 14: LAN Dedicated Master (Disallowed for worker jobs)
    profiles.push(create_synthetic_node(
        15,
        NodeRole::Master,
        16384,
        0,
        150.0,
        40.0,
        GuardRegime::FullCapacity,
        None,
        true,
        vec![ComputeBackend::CpuSimd],
        vec!["avx2"],
        LinkProfile::lan_ethernet(),
    ));

    profiles
}

// ============================================================================
// 1. UNIT TESTS
// ============================================================================

#[test]
fn test_s4_p2_sp1_unit_dag_topological_sort_and_cycle_detection() {
    let mut dag = JobDag::new("job-diamond-01");

    // Add 4 tasks: A -> B, A -> C, B -> D, C -> D (Diamond DAG)
    let spec = TaskSpec::default();
    let constraints = PlacementConstraint::default();

    dag.add_task(TaskNode::new("A", spec.clone(), constraints.clone()))
        .expect("add A");
    dag.add_task(TaskNode::new("B", spec.clone(), constraints.clone()))
        .expect("add B");
    dag.add_task(TaskNode::new("C", spec.clone(), constraints.clone()))
        .expect("add C");
    dag.add_task(TaskNode::new("D", spec, constraints))
        .expect("add D");

    dag.add_edge("A", "B", 1024).expect("A->B");
    dag.add_edge("A", "C", 2048).expect("A->C");
    dag.add_edge("B", "D", 4096).expect("B->D");
    dag.add_edge("C", "D", 8192).expect("C->D");

    let sorted = dag.topological_sort().expect("topological sort");
    assert_eq!(sorted.len(), 4);
    assert_eq!(sorted[0], "A");
    assert_eq!(sorted[3], "D");
    assert!(sorted.contains(&"B".to_string()));
    assert!(sorted.contains(&"C".to_string()));

    // Verify self-loop is rejected immediately
    assert!(dag.add_edge("A", "A", 100).is_err());

    // Inject cycle D -> A and verify cycle detection
    let mut cycle_dag = dag.clone();
    // Force insert back-edge D -> A
    cycle_dag.add_edge("D", "A", 500).expect("edge D->A");
    let cycle_res = cycle_dag.topological_sort();
    assert!(cycle_res.is_err());
    match cycle_res.expect_err("cycle error") {
        SchedulingError::CycleInDag(_) => {}
        other => panic!("Expected CycleInDag, got {other:?}"),
    }
}

#[test]
fn test_s4_p2_sp1_unit_constraint_solver_pruning_matrix() {
    let local_id = make_node_id(1);
    let cluster = create_15_synthetic_profiles(&local_id);
    let solver = ConstraintSolver::new();

    // 1. Privacy Local Only Task: MUST only allow Node 0 (local phone)
    let private_task = TaskNode::new(
        "task-private",
        TaskSpec {
            name: "private-nlp".into(),
            is_private_ai_local_only: true,
            ..Default::default()
        },
        PlacementConstraint {
            hard: HardRequirements {
                privacy_local_only: true,
                ..Default::default()
            },
            ..Default::default()
        },
    );

    let feasible_private = solver
        .filter_feasible_nodes(&private_task, &cluster, &local_id)
        .expect("feasible private");
    assert_eq!(feasible_private.len(), 1);
    assert_eq!(feasible_private[0].node_id, local_id);

    // 2. CUDA required task: only nodes with ComputeBackend::Cuda (Node 3 & Node 8)
    let cuda_task = TaskNode::new(
        "task-cuda",
        TaskSpec::default(),
        PlacementConstraint {
            hard: HardRequirements {
                required_backend: Some(ComputeBackend::Cuda),
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let feasible_cuda = solver
        .filter_feasible_nodes(&cuda_task, &cluster, &local_id)
        .expect("feasible cuda");
    assert_eq!(feasible_cuda.len(), 2);
    for n in feasible_cuda {
        assert!(n.supported_backends.contains(&ComputeBackend::Cuda));
    }

    // 3. High VRAM requirement (20 GB VRAM): only Workstation (24 GB) and WAN server (32 GB)
    let vram_task = TaskNode::new(
        "task-vram",
        TaskSpec::default(),
        PlacementConstraint {
            hard: HardRequirements {
                min_vram_bytes: 20 * 1024 * 1024 * 1024,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let feasible_vram = solver
        .filter_feasible_nodes(&vram_task, &cluster, &local_id)
        .expect("feasible vram");
    assert_eq!(feasible_vram.len(), 2);

    // 4. AVX2 SIMD required: must prune all ARM64 and old SSE4.2 nodes
    let avx2_task = TaskNode::new(
        "task-avx2",
        TaskSpec::default(),
        PlacementConstraint {
            hard: HardRequirements {
                required_simd: vec!["avx2".into()],
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let feasible_avx2 = solver
        .filter_feasible_nodes(&avx2_task, &cluster, &local_id)
        .expect("feasible avx2");
    for n in feasible_avx2 {
        assert!(n.supported_simd.iter().any(|s| s == "avx2"));
        // No thermally paused or evict nodes
        assert_ne!(n.thermal_regime, GuardRegime::PausedWaitCoolOrCharge);
        assert_ne!(n.thermal_regime, GuardRegime::EmergencyEvict);
    }

    // 5. Worker-only role task: must prune Master node
    let worker_only_task = TaskNode::new(
        "task-worker-only",
        TaskSpec::default(),
        PlacementConstraint {
            hard: HardRequirements {
                allowed_node_roles: vec![NodeRole::Worker],
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let feasible_worker = solver
        .filter_feasible_nodes(&worker_only_task, &cluster, &local_id)
        .expect("feasible worker");
    for n in feasible_worker {
        assert_eq!(n.role, NodeRole::Worker);
    }
}

#[test]
fn test_s4_p2_sp1_unit_roofline_cost_model_lan_vs_wan_tradeoff() {
    let local_id = make_node_id(1);
    let cluster = create_15_synthetic_profiles(&local_id);

    let local_phone = &cluster[0];
    let lan_gpu_workstation = &cluster[3];
    let wan_slow_mobile = &cluster[9];

    // Scenario A: Compute-heavy task (10^12 FLOPs, small 2 MB input)
    // Local Phone: 50 GFLOPS -> compute time = 20,000 ms.
    // LAN Workstation: 15,000 GFLOPS -> compute time = 66.6 ms + transfer (2MB / 125MB/s + 1ms = 17ms) = 83.6 ms.
    // Workstation should win by a landslide!
    let compute_heavy_task = TaskNode::new(
        "task-compute-heavy",
        TaskSpec {
            name: "matrix-tiled".into(),
            workload_kind: WorkloadKind::MatrixMultiply,
            estimated_flops: 1_000_000_000_000,
            memory_bytes: 32 * 1024 * 1024,
            output_bytes: 4 * 1024 * 1024,
            is_private_ai_local_only: false,
        },
        PlacementConstraint::default(),
    );

    let cost_local = RooflineCostModel::estimate_cost(
        &compute_heavy_task,
        local_phone,
        true,
        2 * 1024 * 1024,
        &LinkProfile::local(),
    );
    let cost_lan_gpu = RooflineCostModel::estimate_cost(
        &compute_heavy_task,
        lan_gpu_workstation,
        false,
        2 * 1024 * 1024,
        &LinkProfile::lan_ethernet(),
    );

    println!(
        "[Roofline Tradeoff] Compute-Heavy: Local={:.2} ms vs LAN GPU={:.2} ms",
        cost_local.total_cost_score, cost_lan_gpu.total_cost_score
    );
    assert!(
        cost_lan_gpu.total_cost_score < cost_local.total_cost_score / 10.0,
        "LAN GPU workstation must be > 10x faster for compute-heavy task"
    );

    // Scenario B: Data-heavy task (small compute 10^7 FLOPs, huge 300 MB input)
    // Local Phone: compute time = 0.2 ms, transfer = 0 ms -> Total ~0.2 ms.
    // WAN Slow Mobile (15 Mbps, 120 ms RTT, relayed): transfer time = 300 MB / 1.875 MB/s = 160,000 ms (160 s)!
    // Local Phone MUST win by thousands of times!
    let data_heavy_task = TaskNode::new(
        "task-data-heavy",
        TaskSpec {
            name: "bulk-filter".into(),
            workload_kind: WorkloadKind::GeneralCompute,
            estimated_flops: 10_000_000,
            memory_bytes: 300 * 1024 * 1024,
            output_bytes: 1024,
            is_private_ai_local_only: false,
        },
        PlacementConstraint::default(),
    );

    let cost_data_local = RooflineCostModel::estimate_cost(
        &data_heavy_task,
        local_phone,
        true,
        300 * 1024 * 1024,
        &LinkProfile::local(),
    );
    let cost_data_wan = RooflineCostModel::estimate_cost(
        &data_heavy_task,
        wan_slow_mobile,
        false,
        300 * 1024 * 1024,
        &LinkProfile::wan_slow_mobile(),
    );

    println!(
        "[Roofline Tradeoff] Data-Heavy: Local={:.2} ms vs WAN={:.2} ms",
        cost_data_local.total_cost_score, cost_data_wan.total_cost_score
    );
    assert!(
        cost_data_local.total_cost_score < cost_data_wan.total_cost_score / 100.0,
        "Local phone must win by > 100x when transfer cost dominates"
    );
}

// ============================================================================
// 2. INTEGRATION TESTS
// ============================================================================

#[test]
fn test_s4_p2_sp1_integration_100_task_dag_across_32_nodes_under_10ms() {
    let local_id = make_node_id(1);

    // Generate 32 heterogeneous candidate nodes
    let mut cluster = Vec::with_capacity(32);
    for i in 1..=32 {
        let is_gpu = (i % 3) == 0;
        let is_apple = (i % 5) == 0;
        let is_paused = i == 17;
        let role = if i == 32 {
            NodeRole::Master
        } else if (i % 2) == 0 {
            NodeRole::Worker
        } else {
            NodeRole::Hybrid
        };

        let regime = if is_paused {
            GuardRegime::PausedWaitCoolOrCharge
        } else if (i % 7) == 0 {
            GuardRegime::Throttled50
        } else {
            GuardRegime::FullCapacity
        };

        let backends = if is_gpu {
            vec![
                ComputeBackend::CpuSimd,
                ComputeBackend::Cuda,
                ComputeBackend::Vulkan,
            ]
        } else if is_apple {
            vec![ComputeBackend::CpuSimd, ComputeBackend::Metal]
        } else {
            vec![ComputeBackend::CpuSimd]
        };

        let ram = if is_gpu {
            32768
        } else if (i % 2) == 0 {
            16384
        } else {
            4096
        };
        let vram = if is_gpu { 12288 } else { 0 };
        let gflops = if is_gpu { 10_000.0 } else { 150.0 };
        let mem_bw = if is_gpu { 600.0 } else { 35.0 };

        let link = if i == 1 {
            LinkProfile::local()
        } else if (i % 4) == 0 {
            LinkProfile::lan_ethernet()
        } else {
            LinkProfile::lan_wifi6()
        };

        let mut node = create_synthetic_node(
            i as u8,
            role,
            ram,
            vram,
            gflops,
            mem_bw,
            regime,
            Some(85),
            true,
            backends,
            vec!["avx2", "neon"],
            link,
        );
        if i == 1 {
            node.node_id = local_id.clone();
        }
        cluster.push(node);
    }

    // Build 100-task DAG: 10 pipeline stages, 10 parallel tasks per stage
    let mut dag = JobDag::new("job-massive-100-tasks");
    for stage in 0..10 {
        for t in 0..10 {
            let task_id = format!("stage_{stage}_task_{t}");
            let is_private = stage == 0 && t == 0; // Stage 0 Task 0 is private
            let requires_gpu = (stage == 3 || stage == 7) && t < 3; // Some tasks require GPU

            let spec = TaskSpec {
                name: task_id.clone(),
                workload_kind: WorkloadKind::GeneralCompute,
                estimated_flops: 50_000_000 * (stage as u64 + 1),
                memory_bytes: 20 * 1024 * 1024,
                output_bytes: 2 * 1024 * 1024,
                is_private_ai_local_only: is_private,
            };

            let constraints = PlacementConstraint {
                hard: HardRequirements {
                    required_backend: if requires_gpu {
                        Some(ComputeBackend::Vulkan)
                    } else {
                        None
                    },
                    privacy_local_only: is_private,
                    allowed_node_roles: vec![NodeRole::Worker, NodeRole::Hybrid],
                    ..Default::default()
                },
                soft: SoftPreferences::default(),
            };

            dag.add_task(TaskNode::new(&task_id, spec, constraints))
                .expect("add task");

            // Add edges from previous stage tasks
            if stage > 0 {
                let parent_id = format!("stage_{}_task_{t}", stage - 1);
                dag.add_edge(&parent_id, &task_id, 1024 * 1024)
                    .expect("add edge");
            }
        }
    }

    assert_eq!(dag.tasks().len(), 100);

    let solver = ConstraintSolver::new();
    let t_sched_start = Instant::now();
    let plan = solver
        .schedule_dag(&dag, &cluster, &local_id)
        .expect("schedule 100-task DAG");
    let sched_duration = t_sched_start.elapsed();
    let duration_micros = sched_duration.as_micros() as u64;

    println!(
        "[Integration] 100-task DAG across 32 nodes scheduled in {} µs ({:.3} ms) — target < 10,000 µs (< 10 ms)",
        duration_micros,
        duration_micros as f64 / 1000.0
    );

    assert_eq!(plan.assignments.len(), 100);
    assert!(
        duration_micros < 10_000,
        "Constraint solver MUST schedule 100 tasks across 32 nodes in < 10.0 ms (took {} µs)",
        duration_micros
    );

    // 100% constraint satisfaction verification
    for (task_id, decision) in &plan.assignments {
        let task = dag.task(task_id).expect("task");
        let assigned_node = cluster
            .iter()
            .find(|n| n.node_id == decision.assigned_node_id)
            .expect("assigned node exists");

        // Verify privacy
        if task.spec.is_private_ai_local_only {
            assert_eq!(
                decision.assigned_node_id, local_id,
                "Private task must be on local node"
            );
        }

        // Verify thermal regime: never paused or emergency evict
        assert_ne!(
            assigned_node.thermal_regime,
            GuardRegime::PausedWaitCoolOrCharge
        );
        assert_ne!(assigned_node.thermal_regime, GuardRegime::EmergencyEvict);

        // Verify backend requirement
        if let Some(req_b) = task.constraints.hard.required_backend {
            assert!(assigned_node.supported_backends.contains(&req_b));
        }

        // Verify role
        assert!(task
            .constraints
            .hard
            .allowed_node_roles
            .contains(&assigned_node.role));
    }
}

// ============================================================================
// 3. SIMULATION / FAULT-INJECTION TESTS
// ============================================================================

#[test]
fn test_s4_p2_sp1_simulation_randomized_1000_dags_proptest_style() {
    let local_id = make_node_id(1);
    let cluster = create_15_synthetic_profiles(&local_id);
    let solver = ConstraintSolver::new();

    // Deterministic pseudo-random test over 1,000 randomized DAG configurations
    let mut rng_state: u64 = 0xdeadbeefcafebabe;
    let mut prng = move || {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (rng_state >> 33) as u32
    };

    let mut successful_schedules = 0;
    let mut clean_unschedulable_rejections = 0;

    for dag_idx in 0..1000 {
        let task_count = (prng() % 8) + 1; // 1 to 8 tasks
        let mut dag = JobDag::new(format!("rand-job-{dag_idx}"));

        for t in 0..task_count {
            let req_backend = match prng() % 5 {
                0 => None,
                1 => Some(ComputeBackend::CpuSimd),
                2 => Some(ComputeBackend::Cuda),
                3 => Some(ComputeBackend::Metal),
                _ => Some(ComputeBackend::Dx12), // Not present in 15 nodes -> will cleanly fail!
            };

            let min_ram = ((prng() % 100) as u64) * 1024 * 1024 * 1024; // 0 to 100 GB RAM
            let is_private = (prng() % 4) == 0;

            let spec = TaskSpec {
                name: format!("t{t}"),
                workload_kind: WorkloadKind::GeneralCompute,
                estimated_flops: ((prng() % 1000) as u64 + 1) * 1_000_000,
                memory_bytes: 10 * 1024 * 1024,
                output_bytes: 1024,
                is_private_ai_local_only: is_private,
            };

            let constraints = PlacementConstraint {
                hard: HardRequirements {
                    required_backend: req_backend,
                    min_ram_bytes: min_ram,
                    privacy_local_only: is_private,
                    ..Default::default()
                },
                soft: SoftPreferences::default(),
            };

            dag.add_task(TaskNode::new(format!("t{t}"), spec, constraints))
                .expect("add task");
        }

        // Add random forward edges (strictly t_a < t_b to avoid cycles)
        for a in 0..task_count {
            for b in (a + 1)..task_count {
                if (prng() % 3) == 0 {
                    let _ = dag.add_edge(format!("t{a}"), format!("t{b}"), 1024);
                }
            }
        }

        match solver.schedule_dag(&dag, &cluster, &local_id) {
            Ok(plan) => {
                successful_schedules += 1;
                // Verify 100% hard constraint satisfaction
                for (t_id, decision) in &plan.assignments {
                    let task = dag.task(t_id).expect("task");
                    let node = cluster
                        .iter()
                        .find(|n| n.node_id == decision.assigned_node_id)
                        .expect("node");
                    if task.spec.is_private_ai_local_only {
                        assert_eq!(node.node_id, local_id);
                    }
                    if let Some(req_b) = task.constraints.hard.required_backend {
                        assert!(node.supported_backends.contains(&req_b));
                    }
                    assert!(node.available_ram_bytes >= task.constraints.hard.min_ram_bytes);
                }
            }
            Err(SchedulingError::UnschedulableTask { reason, .. }) => {
                clean_unschedulable_rejections += 1;
                // Must be a recognized valid reason
                match reason {
                    UnschedulableReason::MissingBackend
                    | UnschedulableReason::InsufficientRam { .. }
                    | UnschedulableReason::PrivacyViolation(_)
                    | UnschedulableReason::NoFeasibleNodes => {}
                    other => panic!("Unexpected rejection reason: {other:?}"),
                }
            }
            Err(other) => panic!("Unexpected error: {other:?}"),
        }
    }

    println!(
        "[Simulation] 1,000 random DAGs tested: {} scheduled with 100% satisfaction, {} cleanly rejected",
        successful_schedules, clean_unschedulable_rejections
    );
    assert_eq!(successful_schedules + clean_unschedulable_rejections, 1000);
    assert!(successful_schedules > 0);
    assert!(clean_unschedulable_rejections > 0);
}

// ============================================================================
// 4. HARDWARE-REQUIRED TESTS
// ============================================================================

#[test]
fn test_s4_p2_sp1_hardware_monotonic_timer_and_quota() {
    let t0 = Instant::now();
    std::thread::sleep(std::time::Duration::from_millis(5));
    let t1 = Instant::now();
    assert!(t1 > t0);

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.join("../..");
    let quota_script = workspace_root.join("scripts/podman-quota-check.ps1");
    assert!(quota_script.exists());
}

// ============================================================================
// 5. MANUALLY VERIFIED TESTS
// ============================================================================

#[test]
fn test_s4_p2_sp1_manually_verified_explain_json_audit() {
    let local_id = make_node_id(1);
    let cluster = create_15_synthetic_profiles(&local_id);
    let solver = ConstraintSolver::new();

    let mut dag = JobDag::new("job-explainable-pipeline");

    // Task 1: Preprocessing (local)
    dag.add_task(TaskNode::new(
        "preprocess_data",
        TaskSpec {
            name: "preprocess_data".into(),
            workload_kind: WorkloadKind::GeneralCompute,
            estimated_flops: 20_000_000,
            memory_bytes: 50 * 1024 * 1024,
            output_bytes: 10 * 1024 * 1024,
            is_private_ai_local_only: true, // Local only
        },
        PlacementConstraint {
            hard: HardRequirements {
                privacy_local_only: true,
                ..Default::default()
            },
            ..Default::default()
        },
    ))
    .expect("add task 1");

    // Task 2: Heavy neural inference (LAN GPU)
    dag.add_task(TaskNode::new(
        "heavy_neural_inference",
        TaskSpec {
            name: "heavy_neural_inference".into(),
            workload_kind: WorkloadKind::LlamaLayerShard,
            estimated_flops: 2_000_000_000_000, // 2 TFLOPs
            memory_bytes: 4 * 1024 * 1024 * 1024,
            output_bytes: 1024 * 1024,
            is_private_ai_local_only: false,
        },
        PlacementConstraint {
            hard: HardRequirements {
                required_backend: Some(ComputeBackend::Vulkan),
                min_vram_bytes: 4 * 1024 * 1024 * 1024,
                ..Default::default()
            },
            ..Default::default()
        },
    ))
    .expect("add task 2");

    // Task 3: Postprocessing / Render
    dag.add_task(TaskNode::new(
        "render_output",
        TaskSpec {
            name: "render_output".into(),
            workload_kind: WorkloadKind::MediaEncode,
            estimated_flops: 100_000_000,
            memory_bytes: 20 * 1024 * 1024,
            output_bytes: 5 * 1024 * 1024,
            is_private_ai_local_only: false,
        },
        PlacementConstraint::default(),
    ))
    .expect("add task 3");

    dag.add_edge("preprocess_data", "heavy_neural_inference", 10 * 1024 * 1024)
        .expect("edge 1->2");
    dag.add_edge("heavy_neural_inference", "render_output", 1024 * 1024)
        .expect("edge 2->3");

    let plan = solver
        .schedule_dag(&dag, &cluster, &local_id)
        .expect("schedule plan");

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.join("../..");
    let explain_path = workspace_root.join("artifacts/evidence/stage-04/scheduler_placement_explain.json");

    if let Some(parent) = explain_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let explain_json = serde_json::to_vec_pretty(&plan).expect("serialize plan");
    fs::write(&explain_path, &explain_json).expect("write explain file");

    println!(
        "[Manually-Verified] Exported placement plan explainability to {:?}",
        explain_path
    );
    assert!(explain_path.exists());
    assert!(!explain_json.is_empty());
}
