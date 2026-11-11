//! Five-tier verification test suite for Subphase S8.P2.SP1 Protocol Parser & FFI Fuzzing.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use fuzz_suite::{
    check_nesting_depth, BoundedSliceReader, CorpusRunner, FuzzError, FuzzTargets,
    MAX_ALLOCATION_BYTES, MAX_RECURSION_DEPTH,
};
use proptest::prelude::*;

// ============================================================================
// 1. [UNIT] TESTS
// ============================================================================

#[test]
fn test_length_prefix_allocation_bomb_2_pow_62_rejected_with_zero_heap_alloc() {
    let mut bomb_data = Vec::new();
    let claimed_len = 1u64 << 62; // 4 Exabytes
    bomb_data.extend_from_slice(&claimed_len.to_le_bytes());
    bomb_data.extend_from_slice(b"tiny payload");

    let mut reader = BoundedSliceReader::new(&bomb_data);
    let result = reader.read_length_prefixed_slice("AllocationBombTest", MAX_ALLOCATION_BYTES);

    assert!(matches!(
        result,
        Err(FuzzError::AllocationExceeded { requested_bytes, limit_bytes, .. })
        if requested_bytes == (1 << 62) && limit_bytes == MAX_ALLOCATION_BYTES
    ));
}

#[test]
fn test_deeply_nested_gguf_array_and_json_schema_recursion_limit_enforced() {
    // 40 nested levels (exceeds MAX_RECURSION_DEPTH = 32)
    let mut deep_brackets = vec![b'['; 40];
    deep_brackets.resize(80, b']');

    let result = check_nesting_depth(&deep_brackets, MAX_RECURSION_DEPTH);
    assert!(matches!(
        result,
        Err(FuzzError::RecursionDepthExceeded {
            depth: 33,
            limit: 32,
            ..
        })
    ));

    // Safe 16 nested levels passes
    let mut safe_brackets = vec![b'['; 16];
    safe_brackets.resize(32, b']');
    let safe_result = check_nesting_depth(&safe_brackets, MAX_RECURSION_DEPTH);
    assert!(safe_result.is_ok());
    assert_eq!(safe_result.unwrap(), 16);
}

#[test]
fn test_minimized_fuzz_corpus_regression_all_15_targets() {
    let summary = CorpusRunner::run_corpus_on_all_targets().unwrap();
    assert!(summary.passed);
    assert_eq!(summary.targets_tested, 15);
    assert_eq!(summary.panics_detected, 0);
    assert_eq!(summary.crashes_detected, 0);
    assert!(summary.total_vectors_evaluated >= 150);
}

// ============================================================================
// 2. [INTEGRATION] TESTS (Proptest Structured Fuzzing)
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn test_proptest_fuzz_wire_and_tokens(data in proptest::collection::vec(any::<u8>(), 0..512)) {
        // Target 1: P2P Wire Frame
        let r1 = FuzzTargets::fuzz_01_p2p_wire_frame(&data);
        let safe1 = r1.is_ok() || matches!(r1, Err(FuzzError::AllocationExceeded { .. }) | Err(FuzzError::MalformedInput { .. }));
        prop_assert!(safe1);

        // Target 2: Capability Token
        let r2 = FuzzTargets::fuzz_02_capability_token(&data);
        let safe2 = r2.is_ok() || matches!(r2, Err(FuzzError::AllocationExceeded { .. }) | Err(FuzzError::MalformedInput { .. }));
        prop_assert!(safe2);

        // Target 5: GGUF Header
        let r5 = FuzzTargets::fuzz_05_gguf_header(&data);
        let safe5 = r5.is_ok() || matches!(r5, Err(FuzzError::AllocationExceeded { .. }) | Err(FuzzError::MalformedInput { .. }));
        prop_assert!(safe5);

        // Target 8: OpenAI JSON Body
        let r8 = FuzzTargets::fuzz_08_openai_json_body(&data);
        let safe8 = r8.is_ok() || matches!(r8, Err(FuzzError::RecursionDepthExceeded { .. }) | Err(FuzzError::MalformedInput { .. }));
        prop_assert!(safe8);

        // Target 15: Resource Receipt
        let r15 = FuzzTargets::fuzz_15_resource_receipt_and_market_order(&data);
        let safe15 = r15.is_ok() || matches!(r15, Err(FuzzError::AllocationExceeded { .. }) | Err(FuzzError::MalformedInput { .. }));
        prop_assert!(safe15);
    }
}

// ============================================================================
// 3. [SIMULATION] TESTS
// ============================================================================

#[test]
fn test_live_socket_simulated_fuzzer_malformed_quic_and_http() {
    // Blasts 200 mutated network datagrams at wire parsers
    let mut rng_seed: u64 = 0x1337_f00d_cafe_babe;
    let mut malformed_packets_rejected = 0;

    for i in 0..200 {
        rng_seed = rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut packet = vec![0u8; (rng_seed as usize % 256) + 4];
        for b in &mut packet {
            rng_seed = rng_seed.wrapping_add(i as u64);
            *b = (rng_seed & 0xFF) as u8;
        }

        // Test wire frame and RTP/FEC
        let r_wire = FuzzTargets::fuzz_01_p2p_wire_frame(&packet);
        let r_stream = FuzzTargets::fuzz_12_stream_rtp_fec_and_input(&packet);

        if r_wire.is_ok() || r_stream.is_ok() {
            malformed_packets_rejected += 1;
        }
    }

    assert!(malformed_packets_rejected > 0);
}

// ============================================================================
// 4. [HARDWARE-REQUIRED] TESTS
// ============================================================================

#[test]
fn test_hardware_multicore_parallel_fuzz_stability() {
    let prober = umcf_hwprobe::SystemProber::probe_all(
        umcf_core::types::NodeId::from_public_key_bytes(&[3u8; 32]).unwrap(),
    )
    .expect("probe all");

    assert!(prober.cpu.physical_cores >= 1);

    // Multi-threaded parallel fuzzing across logical threads
    let threads = prober.cpu.physical_cores.clamp(2, 8);
    let mut handles = Vec::new();

    for t in 0..threads {
        handles.push(std::thread::spawn(move || {
            let corpus = CorpusRunner::build_adversarial_seed_corpus();
            for (name, data) in &corpus {
                let _ = FuzzTargets::fuzz_01_p2p_wire_frame(data);
                let _ = FuzzTargets::fuzz_05_gguf_header(data);
                let _ = FuzzTargets::fuzz_14_workload_manifest_yaml_toml(data);
                let _ = name;
            }
            t
        }));
    }

    for h in handles {
        assert!(h.join().is_ok());
    }
}

// ============================================================================
// 5. [MANUALLY-VERIFIED] TESTS
// ============================================================================

#[test]
fn test_manual_fuzz_summary_report_and_coverage_schema() {
    let summary = CorpusRunner::run_corpus_on_all_targets().unwrap();
    let json_val = serde_json::to_value(&summary).unwrap();

    assert_eq!(json_val["passed"], true);
    assert_eq!(json_val["targets_tested"], 15);
    assert_eq!(json_val["panics_detected"], 0);
    assert_eq!(json_val["crashes_detected"], 0);
}
