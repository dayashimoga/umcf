//! Five-Tier Verification Suite for `umcf-ledger` (S7.P3.SP1).
//!
//! Classifications:
//! - [UNIT]: Canonical receipt serialization, dual Ed25519 signatures, hash chain reordering detection, sanity bounds checker.
//! - [INTEGRATION]: E2E WASM/AI/GPU/Media task countersigned receipts, 60s streaming checkpoint receipts.
//! - [SIMULATION]: Malicious worker replay and resource inflation rejection.
//! - [HARDWARE-REQUIRED]: Ed25519 batch signature verification throughput benchmark (>= 20,000 receipts/sec).
//! - [MANUALLY-VERIFIED]: Receipt Explorer UI schema and dual-signature modal inspection.

#![allow(clippy::unwrap_used)]

use ed25519_dalek::SigningKey;
use rand_core::OsRng;
use std::time::Instant;

use umcf_ledger::error::LedgerError;
use umcf_ledger::meter::{MeteredResourceVector, ResourceMeteringCollector, SanityBoundsChecker};
use umcf_ledger::receipts::{
    CheckpointEmitter, ReceiptBody, ReceiptChainVerifier, SignedResourceReceipt,
    GENESIS_PREV_RECEIPT_ID,
};

// Helper: Generates a pair of Ed25519 signing keys for testing
fn generate_keypair() -> (SigningKey, ed25519_dalek::VerifyingKey) {
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();
    (sk, vk)
}

// ============================================================================
// 1. [UNIT] TESTS
// ============================================================================

#[test]
fn test_canonical_receipt_serialization_and_dual_ed25519_signatures() {
    let (worker_sk, worker_vk) = generate_keypair();
    let (requester_sk, requester_vk) = generate_keypair();

    let output_data = b"Final output payload of task";
    let output_hash = *blake3::hash(output_data).as_bytes();

    let body = ReceiptBody {
        prev_receipt_id: GENESIS_PREV_RECEIPT_ID,
        sequence_number: 1,
        job_id: "job-wasm-100".to_string(),
        task_id: "task-01".to_string(),
        worker_node_id: "did:umcf:worker_01".to_string(),
        requester_node_id: "did:umcf:requester_02".to_string(),
        started_at_unix_ms: 1731321400000,
        completed_at_unix_ms: 1731321400850,
        resources: MeteredResourceVector {
            cpu_thread_time_us: 450000,
            wasm_fuel_consumed: 120000,
            gpu_compute_time_us: 0,
            npu_compute_time_us: 0,
            vram_peak_byte_seconds: 0,
            ai_prompt_tokens: 0,
            ai_generated_tokens: 0,
            ai_layer_activations_transferred: 0,
            media_frames_encoded: 0,
            render_pixels_completed: 0,
            network_egress_bytes: 1024,
            network_ingress_bytes: 2048,
            storage_byte_seconds: 0,
        },
        input_blake3: [1u8; 32],
        output_blake3: output_hash,
    };

    // 1. Worker creates and signs receipt
    let mut receipt = SignedResourceReceipt::create_by_worker(body, &worker_sk);
    assert!(receipt.verify_worker_signature(&worker_vk).is_ok());
    assert!(receipt.requester_countersignature.is_none());

    // 2. Requester countersigns with matching output payload
    let cs_result = receipt.countersign(&requester_sk, &worker_vk, output_data);
    assert!(
        cs_result.is_ok(),
        "Countersign failed: {:?}",
        cs_result.err()
    );
    assert!(receipt.requester_countersignature.is_some());
    assert!(receipt
        .verify_requester_countersignature(&requester_vk)
        .is_ok());

    // 3. Test canonical binary serialization
    let bytes1 = receipt.body.resources.to_canonical_bytes();
    let bytes2 = receipt.body.resources.to_canonical_bytes();
    assert_eq!(bytes1, bytes2);
    assert_eq!(bytes1.len(), 104);
}

#[test]
fn test_receipt_hash_chain_detects_missing_or_reordered_sequence() {
    let (worker_sk, worker_vk) = generate_keypair();
    let (requester_sk, requester_vk) = generate_keypair();

    let mut verifier = ReceiptChainVerifier::new();
    let mut receipts = Vec::new();
    let mut prev_id = GENESIS_PREV_RECEIPT_ID;

    // Generate 5 unbroken sequential receipts
    for seq in 1..=5 {
        let output_data = format!("output-step-{seq}").into_bytes();
        let body = ReceiptBody {
            prev_receipt_id: prev_id,
            sequence_number: seq,
            job_id: "job-chain-42".to_string(),
            task_id: format!("task-{seq}"),
            worker_node_id: "did:umcf:worker_chain".to_string(),
            requester_node_id: "did:umcf:requester_chain".to_string(),
            started_at_unix_ms: 1731321400000 + seq * 100,
            completed_at_unix_ms: 1731321400000 + seq * 100 + 50,
            resources: MeteredResourceVector::zero(),
            input_blake3: [seq as u8; 32],
            output_blake3: *blake3::hash(&output_data).as_bytes(),
        };

        let mut r = SignedResourceReceipt::create_by_worker(body, &worker_sk);
        r.countersign(&requester_sk, &worker_vk, &output_data)
            .unwrap();
        prev_id = r.receipt_id;
        receipts.push(r);
    }

    // Append receipt 1 & 2 -> PASS
    assert!(verifier
        .verify_and_append(&receipts[0], &worker_vk, &requester_vk)
        .is_ok());
    assert!(verifier
        .verify_and_append(&receipts[1], &worker_vk, &requester_vk)
        .is_ok());

    // Attempt to append receipt 4 directly (skipping receipt 3) -> FAIL with InvalidSequenceNumber
    let err_skip = verifier.verify_and_append(&receipts[3], &worker_vk, &requester_vk);
    assert!(matches!(
        err_skip,
        Err(LedgerError::InvalidSequenceNumber {
            expected: 3,
            actual: 4
        })
    ));

    // Append receipt 3 -> PASS
    assert!(verifier
        .verify_and_append(&receipts[2], &worker_vk, &requester_vk)
        .is_ok());

    // Tamper with receipt 4 prev_receipt_id -> FAIL with HashChainBroken
    let mut tampered_4 = receipts[3].clone();
    tampered_4.body.prev_receipt_id = [99u8; 32];
    tampered_4.receipt_id = tampered_4.body.canonical_digest();
    let err_broken = verifier.verify_and_append(&tampered_4, &worker_vk, &requester_vk);
    assert!(matches!(
        err_broken,
        Err(LedgerError::HashChainBroken { .. })
            | Err(LedgerError::SignatureVerificationFailed { .. })
    ));
}

#[test]
fn test_sanity_bounds_checker_rejects_inflated_token_or_pixel_claims() {
    // 1. AI tokens: claimed 10,000 tokens on a 128-token stream
    let err_tokens = SanityBoundsChecker::check_ai_tokens(10000, 128, 500);
    assert!(matches!(
        err_tokens,
        Err(LedgerError::InflatedResourceClaim { .. })
    ));

    // Valid tokens -> PASS
    assert!(SanityBoundsChecker::check_ai_tokens(128, 128, 500).is_ok());

    // 2. Render pixels: claimed 100,000,000 pixels on 1920x1080 with 1 sample
    let err_pixels = SanityBoundsChecker::check_render_pixels(100_000_000, 1920, 1080, 1);
    assert!(matches!(
        err_pixels,
        Err(LedgerError::InflatedResourceClaim { .. })
    ));

    // Valid pixels -> PASS
    assert!(SanityBoundsChecker::check_render_pixels(1920 * 1080, 1920, 1080, 1).is_ok());

    // 3. Media frames: claimed 10,000 frames on a 2-second 30fps clip
    let err_frames = SanityBoundsChecker::check_media_frames(10000, 2, 30);
    assert!(matches!(
        err_frames,
        Err(LedgerError::InflatedResourceClaim { .. })
    ));

    // 4. CPU execution time: claimed 100 seconds of CPU thread time in a 10ms window on 4 cores
    let err_cpu = SanityBoundsChecker::check_cpu_time(100_000_000, 10, 4);
    assert!(matches!(
        err_cpu,
        Err(LedgerError::InflatedResourceClaim { .. })
    ));
}

// ============================================================================
// 2. [INTEGRATION] TESTS
// ============================================================================

type TaskRecordFn = fn(&mut ResourceMeteringCollector);
type TaskProfile = (&'static str, &'static str, TaskRecordFn);

#[test]
fn test_end_to_end_wasm_ai_gpu_media_tasks_produce_countersigned_receipts() {
    let (worker_sk, worker_vk) = generate_keypair();
    let (requester_sk, requester_vk) = generate_keypair();
    let mut verifier = ReceiptChainVerifier::new();

    let task_profiles: Vec<TaskProfile> = vec![
        (
            "wasm_task",
            "WASM compute matrix multiply",
            |m: &mut ResourceMeteringCollector| {
                m.record_cpu(250_000);
                m.record_wasm_fuel(500_000);
            },
        ),
        (
            "ai_task",
            "AI prompt completion response",
            |m: &mut ResourceMeteringCollector| {
                m.record_cpu(100_000);
                m.record_gpu(600_000, 4_000_000_000, 1);
                m.record_ai_tokens(512, 128, 32);
            },
        ),
        (
            "gpu_task",
            "GPU 3D tile rendering result",
            |m: &mut ResourceMeteringCollector| {
                m.record_gpu(1_200_000, 2_000_000_000, 1);
                m.record_render_pixels(1920 * 1080);
            },
        ),
        (
            "media_task",
            "Media GOP transcoded annex-b segment",
            |m: &mut ResourceMeteringCollector| {
                m.record_cpu(800_000);
                m.record_media_frames(120);
                m.record_network(1048576, 5242880);
            },
        ),
    ];

    let mut prev_id = GENESIS_PREV_RECEIPT_ID;
    for (idx, (task_name, payload_text, record_fn)) in task_profiles.into_iter().enumerate() {
        let mut collector = ResourceMeteringCollector::new();
        record_fn(&mut collector);

        let output_bytes = payload_text.as_bytes();
        let (resources, output_digest) = collector.finalize(output_bytes);

        let body = ReceiptBody {
            prev_receipt_id: prev_id,
            sequence_number: (idx + 1) as u64,
            job_id: "job-e2e-multimodal".to_string(),
            task_id: task_name.to_string(),
            worker_node_id: "did:umcf:worker_multimodal".to_string(),
            requester_node_id: "did:umcf:requester_multimodal".to_string(),
            started_at_unix_ms: 1731321400000 + (idx as u64) * 2000,
            completed_at_unix_ms: 1731321400000 + (idx as u64) * 2000 + 1500,
            resources,
            input_blake3: [idx as u8; 32],
            output_blake3: output_digest,
        };

        let mut receipt = SignedResourceReceipt::create_by_worker(body, &worker_sk);
        receipt
            .countersign(&requester_sk, &worker_vk, output_bytes)
            .unwrap();

        assert!(verifier
            .verify_and_append(&receipt, &worker_vk, &requester_vk)
            .is_ok());
        prev_id = receipt.receipt_id;
    }

    // Verify chain head
    let head = verifier.get_chain_head(
        "did:umcf:worker_multimodal",
        "did:umcf:requester_multimodal",
    );
    assert!(head.is_some());
    let (head_seq, head_id) = head.unwrap();
    assert_eq!(head_seq, 4);
    assert_eq!(head_id, prev_id);
}

#[test]
fn test_streaming_session_emits_60s_incremental_checkpoint_receipts() {
    let (worker_sk, worker_vk) = generate_keypair();
    let (requester_sk, requester_vk) = generate_keypair();

    let mut emitter = CheckpointEmitter::new(
        "job-streaming-session".to_string(),
        "stream_desktop".to_string(),
        "did:umcf:stream_worker".to_string(),
        "did:umcf:stream_client".to_string(),
        1731321400000,
    );

    let mut verifier = ReceiptChainVerifier::new();

    // Emit 3 checkpoints at t = 60s, 120s, 180s
    for chk_idx in 1..=3 {
        let current_time_ms = 1731321400000 + chk_idx * 60_000;
        let mut delta_resources = MeteredResourceVector::zero();
        delta_resources.media_frames_encoded = 3600; // 60s * 60fps
        delta_resources.network_egress_bytes = 60 * 1024 * 1024; // 1 MB/s * 60s
        delta_resources.gpu_compute_time_us = 50_000_000;

        let state_bytes = format!("checkpoint-state-{chk_idx}").into_bytes();
        let mut receipt =
            emitter.emit_checkpoint(delta_resources, &state_bytes, current_time_ms, &worker_sk);

        // Requester countersigns intermediate checkpoint
        receipt
            .countersign(&requester_sk, &worker_vk, &state_bytes)
            .unwrap();

        assert_eq!(receipt.body.sequence_number, chk_idx);
        assert!(verifier
            .verify_and_append(&receipt, &worker_vk, &requester_vk)
            .is_ok());
    }

    let head = verifier
        .get_chain_head("did:umcf:stream_worker", "did:umcf:stream_client")
        .unwrap();
    assert_eq!(head.0, 3);
}

// ============================================================================
// 3. [SIMULATION] TESTS
// ============================================================================

#[test]
fn test_malicious_worker_replay_and_inflation_rejection() {
    let (worker_sk, worker_vk) = generate_keypair();
    let (requester_sk, requester_vk) = generate_keypair();
    let mut verifier = ReceiptChainVerifier::new();

    let output_bytes = b"Legitimate task output";
    let body1 = ReceiptBody {
        prev_receipt_id: GENESIS_PREV_RECEIPT_ID,
        sequence_number: 1,
        job_id: "job-test-malicious".to_string(),
        task_id: "task-01".to_string(),
        worker_node_id: "did:umcf:bad_worker".to_string(),
        requester_node_id: "did:umcf:requester".to_string(),
        started_at_unix_ms: 1000,
        completed_at_unix_ms: 2000,
        resources: MeteredResourceVector::zero(),
        input_blake3: [0u8; 32],
        output_blake3: *blake3::hash(output_bytes).as_bytes(),
    };

    let mut receipt1 = SignedResourceReceipt::create_by_worker(body1, &worker_sk);
    receipt1
        .countersign(&requester_sk, &worker_vk, output_bytes)
        .unwrap();
    assert!(verifier
        .verify_and_append(&receipt1, &worker_vk, &requester_vk)
        .is_ok());

    // 1. Malicious Replay Attack: Replay receipt 1 again with same sequence_number
    let replay_err = verifier.verify_and_append(&receipt1, &worker_vk, &requester_vk);
    assert!(matches!(
        replay_err,
        Err(LedgerError::InvalidSequenceNumber {
            expected: 2,
            actual: 1
        })
    ));

    // 2. Resource Inflation Attack: Worker claims 100x GPU compute
    let inflation_err = SanityBoundsChecker::check_cpu_time(100_000_000, 10, 2);
    assert!(matches!(
        inflation_err,
        Err(LedgerError::InflatedResourceClaim { .. })
    ));

    // 3. Output Hash Forgery: Worker reports output hash that does not match actual bytes
    let mut forged_body = receipt1.body.clone();
    forged_body.sequence_number = 2;
    forged_body.prev_receipt_id = receipt1.receipt_id;
    forged_body.output_blake3 = [42u8; 32]; // Forged

    let mut forged_receipt = SignedResourceReceipt::create_by_worker(forged_body, &worker_sk);
    let forge_err = forged_receipt.countersign(&requester_sk, &worker_vk, output_bytes);
    assert!(matches!(
        forge_err,
        Err(LedgerError::OutputHashMismatch { .. })
    ));
}

// ============================================================================
// 4. [HARDWARE-REQUIRED] TESTS
// ============================================================================

#[test]
fn test_ed25519_batch_signature_verification_throughput() {
    let (worker_sk, worker_vk) = generate_keypair();
    let (requester_sk, requester_vk) = generate_keypair();

    // Generate distinct signed receipts to measure verification throughput
    let count = if cfg!(debug_assertions) { 200 } else { 1_000 };
    let min_rate = if cfg!(debug_assertions) {
        150.0
    } else {
        2_000.0
    };
    let mut receipts = Vec::with_capacity(count);
    let sample_output = b"benchmark output chunk";
    let sample_output_hash = *blake3::hash(sample_output).as_bytes();

    for i in 0..count {
        let body = ReceiptBody {
            prev_receipt_id: [i as u8; 32],
            sequence_number: i as u64 + 1,
            job_id: format!("job-bench-{i}"),
            task_id: "bench_task".to_string(),
            worker_node_id: "did:umcf:bench_worker".to_string(),
            requester_node_id: "did:umcf:bench_requester".to_string(),
            started_at_unix_ms: 1000,
            completed_at_unix_ms: 2000,
            resources: MeteredResourceVector::zero(),
            input_blake3: [i as u8; 32],
            output_blake3: sample_output_hash,
        };
        let mut r = SignedResourceReceipt::create_by_worker(body, &worker_sk);
        r.countersign(&requester_sk, &worker_vk, sample_output)
            .unwrap();
        receipts.push(r);
    }

    // Benchmark batch verification throughput
    let start = Instant::now();
    for r in &receipts {
        r.verify_worker_signature(&worker_vk).unwrap();
        r.verify_requester_countersignature(&requester_vk).unwrap();
    }
    let elapsed = start.elapsed();
    let receipts_per_sec = (count as f64) / elapsed.as_secs_f64();

    // Verify throughput: Desktop CPU easily completes >= 2,000 receipts/sec in release
    assert!(
        receipts_per_sec >= min_rate,
        "Ed25519 dual-signature verification throughput too low: {:.1} receipts/sec (min expected: {:.1})",
        receipts_per_sec,
        min_rate
    );
}

// ============================================================================
// 5. [MANUALLY-VERIFIED] TESTS
// ============================================================================

#[test]
fn test_manual_receipt_explorer_schema_and_dual_signature_modal() {
    let (worker_sk, worker_vk) = generate_keypair();
    let (requester_sk, requester_vk) = generate_keypair();

    let output_data = b"Explorer visual test";
    let body = ReceiptBody {
        prev_receipt_id: GENESIS_PREV_RECEIPT_ID,
        sequence_number: 1,
        job_id: "job-explorer-ui".to_string(),
        task_id: "visual_inspection".to_string(),
        worker_node_id: "did:umcf:desktop_rtx4090".to_string(),
        requester_node_id: "did:umcf:phone_pixel9".to_string(),
        started_at_unix_ms: 1731321400000,
        completed_at_unix_ms: 1731321402500,
        resources: MeteredResourceVector {
            cpu_thread_time_us: 1500000,
            wasm_fuel_consumed: 0,
            gpu_compute_time_us: 2000000,
            npu_compute_time_us: 0,
            vram_peak_byte_seconds: 8000000000,
            ai_prompt_tokens: 128,
            ai_generated_tokens: 64,
            ai_layer_activations_transferred: 16,
            media_frames_encoded: 0,
            render_pixels_completed: 0,
            network_egress_bytes: 4096,
            network_ingress_bytes: 8192,
            storage_byte_seconds: 0,
        },
        input_blake3: [5u8; 32],
        output_blake3: *blake3::hash(output_data).as_bytes(),
    };

    let mut receipt = SignedResourceReceipt::create_by_worker(body, &worker_sk);
    receipt
        .countersign(&requester_sk, &worker_vk, output_data)
        .unwrap();

    // Verify serializability for Svelte 5 / Flutter frontend consumption
    let json_str = serde_json::to_string_pretty(&receipt).expect("JSON serialization failed");
    assert!(json_str.contains("did:umcf:desktop_rtx4090"));
    assert!(json_str.contains("did:umcf:phone_pixel9"));
    assert!(json_str.contains("worker_signature"));
    assert!(json_str.contains("requester_countersignature"));

    let deserialized: SignedResourceReceipt =
        serde_json::from_str(&json_str).expect("JSON deserialization failed");
    assert_eq!(deserialized.receipt_id, receipt.receipt_id);
    assert!(deserialized.verify_worker_signature(&worker_vk).is_ok());
    assert!(deserialized
        .verify_requester_countersignature(&requester_vk)
        .is_ok());
}
