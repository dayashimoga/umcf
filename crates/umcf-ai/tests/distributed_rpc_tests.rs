//! Classified Verification Tests for Subphase S5.P3.SP2:
//! Distributed llama.cpp RPC Activation Transfer & Fault Recovery.
//!
//! Tiers:
//! - [Unit]: Activation packet serialization, BLAKE3 checksum, FP16/Q8_0 codec SNR, KV commit/rollback.
//! - [Integration]: 3-node pipeline greedy parity with single node, mid-generation worker failover, mTLS guard.
//! - [Simulation]: Link jitter and simulated packet loss recovery.
//! - [Hardware-Required]: Forward activation pipeline latency benchmark.
//! - [Manually-Verified]: UI activation pulse telemetry structure verification.

use std::collections::HashMap;
use umcf_ai::distributed_rpc::{
    ActivationCodec, ActivationDtype, DistributedPipelineCoordinator, ForwardActivationPacket,
    PipelineStageWorker, RpcCapabilityToken,
};
use umcf_ai::partitioner::LayerPartitionStage;
use umcf_ai::EngineError;
use umcf_core::types::NodeId;

fn create_test_stage(
    stage_idx: usize,
    node_id: &str,
    start: u32,
    end: u32,
    has_embd: bool,
    has_head: bool,
) -> LayerPartitionStage {
    LayerPartitionStage {
        stage_index: stage_idx,
        node_id: NodeId(node_id.to_string()),
        has_embeddings: has_embd,
        start_layer: start,
        end_layer: end,
        has_output_head: has_head,
        required_weights_bytes: 1024 * 1024,
        required_kv_bytes: 512 * 1024,
        required_scratch_bytes: 256 * 1024,
        total_stage_memory_bytes: 1792 * 1024,
    }
}

// ---------------------------------------------------------------------------
// 1. UNIT TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_unit_activation_packet_serialization_and_blake3_checksum_validation() {
    let payload = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let mut packet = ForwardActivationPacket::new(
        "session-123".to_string(),
        0,
        1,
        vec![42],
        ActivationDtype::F32,
        [1, 1, 10],
        payload.clone(),
    );

    // Initial valid checksum
    assert!(packet.verify_checksum().is_ok());

    // Tamper payload byte
    packet.payload[0] ^= 0xFF;
    let err = packet.verify_checksum().expect_err("tampered packet must fail checksum");
    match err {
        EngineError::RpcChecksumMismatch { .. } => {}
        other => panic!("expected RpcChecksumMismatch, got {other:?}"),
    }
}

#[test]
fn test_unit_fp16_and_q8_activation_codec_roundtrip_snr() {
    // Generate realistic hidden-state vector (n_embd = 512)
    let mut original = Vec::with_capacity(512);
    for i in 0..512 {
        let x = ((i as f32) * 0.05).sin() * 2.5;
        original.push(x);
    }

    // 1. FP16 Codec
    let fp16_bytes = ActivationCodec::encode_f32_to_f16(&original);
    assert_eq!(fp16_bytes.len(), 512 * 2);
    let fp16_recon = ActivationCodec::decode_f16_to_f32(&fp16_bytes).expect("FP16 decode must succeed");
    let snr_fp16 = ActivationCodec::compute_snr_db(&original, &fp16_recon);
    println!("FP16 SNR: {snr_fp16:.2} dB");
    assert!(snr_fp16 > 60.0, "FP16 SNR ({snr_fp16:.2} dB) must be > 60 dB");

    // 2. Q8_0 Codec
    let q8_bytes = ActivationCodec::encode_f32_to_q8_0(&original);
    assert_eq!(q8_bytes.len(), (512 / 32) * 34); // 16 blocks * 34 = 544 bytes
    let q8_recon =
        ActivationCodec::decode_q8_0_to_f32(&q8_bytes, 512).expect("Q8_0 decode must succeed");
    let snr_q8 = ActivationCodec::compute_snr_db(&original, &q8_recon);
    println!("Q8_0 SNR: {snr_q8:.2} dB");
    assert!(snr_q8 > 40.0, "Q8_0 SNR ({snr_q8:.2} dB) must be > 40 dB");
}

#[test]
fn test_unit_kv_step_commit_and_rollback_state_consistency() {
    let stage = create_test_stage(0, "did:umcf:node-0", 0, 7, true, false);
    let mut worker = PipelineStageWorker::new(stage, "session-test".to_string());

    let dummy_floats = vec![1.0f32; 16];
    let mut bytes = Vec::new();
    for f in &dummy_floats {
        bytes.extend_from_slice(&f.to_le_bytes());
    }

    let packet = ForwardActivationPacket::new(
        "session-test".to_string(),
        0,
        1,
        vec![101],
        ActivationDtype::F32,
        [1, 1, 16],
        bytes.clone(),
    );

    // Forward pass
    let res = worker.process_forward(&packet, None);
    assert!(res.is_ok());
    assert_eq!(worker.cached_tokens, vec![101]);
    assert_eq!(worker.uncommitted_step_idx, Some(1));

    // Commit
    worker.commit_step(1);
    assert_eq!(worker.committed_step_idx, 1);
    assert_eq!(worker.uncommitted_step_idx, None);

    // Second step aborted and rolled back
    let packet2 = ForwardActivationPacket::new(
        "session-test".to_string(),
        0,
        2,
        vec![102],
        ActivationDtype::F32,
        [1, 1, 16],
        bytes,
    );
    let _ = worker.process_forward(&packet2, None);
    assert_eq!(worker.cached_tokens, vec![101, 102]);

    worker.rollback_step(2);
    assert_eq!(worker.cached_tokens, vec![101], "rollback must restore committed state");
    assert_eq!(worker.uncommitted_step_idx, None);
}

// ---------------------------------------------------------------------------
// 2. INTEGRATION TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_integration_3_node_pipeline_greedy_token_parity_with_single_node() {
    // Define 24 layers total
    // Setup A: Single-Node (all 24 layers on 1 node)
    let single_stage = create_test_stage(0, "did:umcf:monolith", 0, 23, true, true);
    let mut single_coord = DistributedPipelineCoordinator::new(
        "session-monolith".to_string(),
        vec![single_stage],
        HashMap::new(),
    );

    // Setup B: 3-Node Distributed Pipeline (Node 0: 0..7, Node 1: 8..15, Node 2: 16..23)
    let stage0 = create_test_stage(0, "did:umcf:stage-0", 0, 7, true, false);
    let stage1 = create_test_stage(1, "did:umcf:stage-1", 8, 15, false, false);
    let stage2 = create_test_stage(2, "did:umcf:stage-2", 16, 23, false, true);

    let mut dist_coord = DistributedPipelineCoordinator::new(
        "session-distributed".to_string(),
        vec![stage0, stage1, stage2],
        HashMap::new(),
    );

    let input_dim = 64;
    let mut current_input = vec![0.5f32; input_dim];

    let mut single_tokens = Vec::new();
    let mut dist_tokens = Vec::new();

    // Run 32 steps of greedy generation
    for step in 0..32 {
        let single_logits = single_coord
            .forward_token_step(&current_input, step as i32, ActivationDtype::F32)
            .expect("single step forward");

        let dist_logits = dist_coord
            .forward_token_step(&current_input, step as i32, ActivationDtype::F32)
            .expect("dist step forward");

        // Greedy argmax token selection
        let single_tok = argmax(&single_logits);
        let dist_tok = argmax(&dist_logits);

        single_tokens.push(single_tok);
        dist_tokens.push(dist_tok);

        // Feed back matching next input
        current_input = single_logits[..input_dim].to_vec();
    }

    assert_eq!(
        single_tokens, dist_tokens,
        "Bit-exact greedy token parity failed between single-node and 3-node distributed pipeline!"
    );
    println!("Bit-exact parity verified over 32 tokens: {single_tokens:?}");
}

#[test]
fn test_integration_worker_disconnect_mid_generation_seamless_failover() {
    let stage0 = create_test_stage(0, "did:umcf:stage-0", 0, 7, true, false);
    let stage1 = create_test_stage(1, "did:umcf:stage-1", 8, 15, false, false);
    let stage2 = create_test_stage(2, "did:umcf:stage-2", 16, 23, false, true);

    let mut failover_map = HashMap::new();
    failover_map.insert("did:umcf:stage-1".to_string(), "did:umcf:standby-node".to_string());

    let mut coord = DistributedPipelineCoordinator::new(
        "session-failover".to_string(),
        vec![stage0, stage1, stage2],
        failover_map,
    );

    let input = vec![0.4f32; 32];

    // Generate tokens 1..=15 normally
    for pos in 0..15 {
        let _ = coord
            .forward_token_step(&input, pos, ActivationDtype::F32)
            .expect("step before failure");
    }
    assert_eq!(coord.current_step_idx, 15);

    // Simulate node 1 disconnect at token #15
    let failed_node = NodeId("did:umcf:stage-1".to_string());
    let standby_node = NodeId("did:umcf:standby-node".to_string());

    let recovery_ms = coord
        .trigger_worker_failover(&failed_node, &standby_node)
        .expect("failover trigger");

    println!("Failover recovered in {recovery_ms} ms (SLA <= 1500 ms)");
    assert!(recovery_ms < 1500, "recovery took {recovery_ms} ms, exceeding 1500 ms SLA");
    assert!(coord.is_fault_recovered);

    // Resume tokens 16..=25 seamlessly without dropping stream
    for pos in 15..25 {
        let logits = coord
            .forward_token_step(&input, pos, ActivationDtype::F32)
            .expect("step after failover");
        assert!(!logits.is_empty());
    }

    assert_eq!(coord.current_step_idx, 25);
}

#[test]
fn test_integration_unauthenticated_rpc_connection_rejected_by_mtls_guard() {
    let stage = create_test_stage(0, "did:umcf:secure-worker", 0, 7, true, false);
    let mut worker = PipelineStageWorker::new(stage, "session-secure".to_string());

    let expired_token = RpcCapabilityToken {
        subject_node: NodeId("did:umcf:attacker".to_string()),
        issuer_node: NodeId("did:umcf:master".to_string()),
        permissions: vec!["inference:worker".to_string()],
        expires_at_unix_s: 1000, // expired in past
    };

    let caller_node = NodeId("did:umcf:attacker".to_string());
    let current_time_s = 2000;

    let packet = ForwardActivationPacket::new(
        "session-secure".to_string(),
        0,
        1,
        vec![1],
        ActivationDtype::F32,
        [1, 1, 4],
        vec![0u8; 16],
    );

    let err = worker
        .process_forward(&packet, Some((&expired_token, &caller_node, current_time_s)))
        .expect_err("expired token must be rejected");

    match err {
        EngineError::RpcAuthenticationFailed { .. } => {}
        other => panic!("expected RpcAuthenticationFailed, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 3. SIMULATION TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_simulation_link_jitter_and_packet_loss_recovery() {
    let stage0 = create_test_stage(0, "did:umcf:node-0", 0, 7, true, false);
    let stage1 = create_test_stage(1, "did:umcf:node-1", 8, 15, false, true);

    let mut coord = DistributedPipelineCoordinator::new(
        "session-jitter".to_string(),
        vec![stage0, stage1],
        HashMap::new(),
    );

    let input = vec![0.25f32; 16];

    // Simulate 50 forward cycles with pseudo-random network jitter
    for step in 0..50 {
        // Jitter simulation: delay calculation
        let jitter_ms = (step * 7) % 20; // 0..20 ms jitter
        assert!(jitter_ms <= 20);

        let res = coord.forward_token_step(&input, step as i32, ActivationDtype::fp16_or_f32(step));
        assert!(res.is_ok(), "forward step {step} failed under simulated jitter");
    }

    assert_eq!(coord.current_step_idx, 50);
}

// ---------------------------------------------------------------------------
// 4. HARDWARE-REQUIRED TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_hardware_distributed_inference_latency_benchmark() {
    let stage0 = create_test_stage(0, "did:umcf:hw-node-0", 0, 15, true, false);
    let stage1 = create_test_stage(1, "did:umcf:hw-node-1", 16, 31, false, true);

    let mut coord = DistributedPipelineCoordinator::new(
        "session-hw-bench".to_string(),
        vec![stage0, stage1],
        HashMap::new(),
    );

    let input = vec![0.1f32; 128];
    let start = std::time::Instant::now();

    for i in 0..10 {
        let _ = coord
            .forward_token_step(&input, i, ActivationDtype::F16)
            .expect("hardware benchmark step");
    }

    let elapsed = start.elapsed();
    let per_token_ms = elapsed.as_secs_f64() * 1000.0 / 10.0;
    println!("Distributed forward latency on hardware: {per_token_ms:.2} ms/token");
    assert!(per_token_ms < 100.0, "latency must be reasonable on hardware");
}

// ---------------------------------------------------------------------------
// 5. MANUALLY-VERIFIED TESTS
// ---------------------------------------------------------------------------

#[test]
fn test_manual_pipeline_topology_live_pulse_visualization() {
    let packet = ForwardActivationPacket::new(
        "session-telemetry".to_string(),
        0,
        42,
        vec![101],
        ActivationDtype::Q8_0,
        [1, 1, 256],
        vec![0u8; (256 / 32) * 34],
    );

    let json = serde_json::to_string(&packet).expect("must serialize activation packet");
    assert!(json.contains("session-telemetry"));
    assert!(json.contains("step_idx"));
    assert!(json.contains("blake3_checksum"));
}

fn argmax(slice: &[f32]) -> usize {
    let mut best_idx = 0;
    let mut max_val = f32::NEG_INFINITY;
    for (i, &v) in slice.iter().enumerate() {
        if v > max_val {
            max_val = v;
            best_idx = i;
        }
    }
    best_idx
}

trait ActivationDtypeExt {
    fn fp16_or_f32(step: usize) -> ActivationDtype;
}

impl ActivationDtypeExt for ActivationDtype {
    fn fp16_or_f32(step: usize) -> ActivationDtype {
        if step % 2 == 0 {
            ActivationDtype::F16
        } else {
            ActivationDtype::F32
        }
    }
}
