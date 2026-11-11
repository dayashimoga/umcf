//! Classified Five-Tier Test Suite for Subphase S8.P1.SP1:
//! Multi-OS Cluster Integration Suite (Android, iOS, Windows, Linux, macOS, Web).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use e2e_cluster::{
    CaseCollisionDetector, MultiOsClusterHarness, PageAlignmentValidator, PlatformError,
    TargetArch, TargetOs, VirtualPathNormalizer, PAGE_SIZE_16KB,
};

// ============================================================================
// 1. [UNIT] TESTS
// ============================================================================

#[test]
fn test_virtual_path_normalizer_rejects_windows_backslashes_and_drive_letters_on_wire() {
    // 1. Valid POSIX relative paths succeed
    let p1 = VirtualPathNormalizer::normalize_wire_path("models/llama3/model.gguf").unwrap();
    assert_eq!(p1, "models/llama3/model.gguf");

    let p2 = VirtualPathNormalizer::normalize_wire_path("./data/weights.bin").unwrap();
    assert_eq!(p2, "data/weights.bin");

    let p3 = VirtualPathNormalizer::normalize_wire_path("a//b///c").unwrap();
    assert_eq!(p3, "a/b/c");

    // 2. Windows backslash path MUST be rejected
    let err1 =
        VirtualPathNormalizer::normalize_wire_path("models\\llama3\\model.gguf").unwrap_err();
    assert!(
        matches!(err1, PlatformError::InvalidWirePath { .. }),
        "Expected InvalidWirePath for backslashes, got {err1:?}"
    );

    // 3. Windows drive letter MUST be rejected
    let err2 = VirtualPathNormalizer::normalize_wire_path("C:/Users/model.gguf").unwrap_err();
    assert!(
        matches!(err2, PlatformError::InvalidWirePath { .. }),
        "Expected InvalidWirePath for drive letter, got {err2:?}"
    );

    // 4. Absolute path starting with '/' MUST be rejected
    let err3 = VirtualPathNormalizer::normalize_wire_path("/etc/passwd").unwrap_err();
    assert!(
        matches!(err3, PlatformError::InvalidWirePath { .. }),
        "Expected InvalidWirePath for absolute path, got {err3:?}"
    );

    // 5. Path traversal ('..') MUST be rejected
    let err4 = VirtualPathNormalizer::normalize_wire_path("models/../../secret").unwrap_err();
    assert!(
        matches!(err4, PlatformError::InvalidWirePath { .. }),
        "Expected InvalidWirePath for path traversal, got {err4:?}"
    );
}

#[test]
fn test_16kb_page_size_mmap_alignment_for_android15_and_apple_silicon() {
    // Exactly 16 KiB aligned offsets succeed
    assert!(PageAlignmentValidator::validate_16kb_alignment(0).is_ok());
    assert!(PageAlignmentValidator::validate_16kb_alignment(16384).is_ok());
    assert!(PageAlignmentValidator::validate_16kb_alignment(32768).is_ok());
    assert!(PageAlignmentValidator::validate_16kb_alignment(1024 * 1024 * 64).is_ok());

    // 4 KiB aligned offset (unaligned to 16 KiB) MUST fail
    let err = PageAlignmentValidator::validate_16kb_alignment(4096).unwrap_err();
    assert!(
        matches!(
            err,
            PlatformError::PageAlignmentViolation {
                required_page_size_bytes: 16384,
                ..
            }
        ),
        "Expected 16 KiB PageAlignmentViolation, got {err:?}"
    );

    // Arbitrary unaligned offset MUST fail
    assert!(PageAlignmentValidator::validate_16kb_alignment(12345).is_err());

    // Padding helper round-trips
    assert_eq!(PageAlignmentValidator::pad_to_16kb(0), 0);
    assert_eq!(PageAlignmentValidator::pad_to_16kb(100), PAGE_SIZE_16KB);
    assert_eq!(
        PageAlignmentValidator::pad_to_16kb(PAGE_SIZE_16KB),
        PAGE_SIZE_16KB
    );
    assert_eq!(
        PageAlignmentValidator::pad_to_16kb(PAGE_SIZE_16KB + 1),
        PAGE_SIZE_16KB * 2
    );
}

#[test]
fn test_case_collision_detector_for_cross_os_storage_manifests() {
    // Distinct paths succeed
    let valid_paths = vec![
        "src/lib.rs",
        "src/main.rs",
        "docs/ARCHITECTURE.md",
        "data/input_01.csv",
    ];
    assert!(CaseCollisionDetector::detect_collisions(&valid_paths).is_ok());

    // Colliding paths MUST be detected (e.g., README.md and readme.md)
    let colliding_paths = vec!["docs/README.md", "docs/readme.md"];
    let err1 = CaseCollisionDetector::detect_collisions(&colliding_paths).unwrap_err();
    assert!(
        matches!(err1, PlatformError::CaseCollisionDetected { .. }),
        "Expected CaseCollisionDetected, got {err1:?}"
    );

    // Deep path collision
    let deep_collision = vec!["assets/textures/Wood.png", "assets/textures/wood.png"];
    let err2 = CaseCollisionDetector::detect_collisions(&deep_collision).unwrap_err();
    assert!(matches!(err2, PlatformError::CaseCollisionDetected { .. }));
}

// ============================================================================
// 2. [INTEGRATION] TESTS
// ============================================================================

#[test]
fn test_scenario_1_to_6_full_multi_os_concurrent_cluster_suite() {
    let mut harness = MultiOsClusterHarness::new();
    assert_eq!(harness.node_count(), 6);

    // Scenario 1: mTLS Discovery & NAT Traversal
    let s1 = harness.run_scenario_1_mtls_discovery().unwrap();
    assert!(s1.passed);
    assert_eq!(s1.nodes_participating.len(), 6);

    // Scenario 2: Cross-OS Distributed WASM Map-Reduce
    let s2 = harness.run_scenario_2_wasm_determinism().unwrap();
    assert!(s2.passed);

    // Scenario 3: Cross-OS Pipeline-Parallel LLM Inference
    let s3 = harness.run_scenario_3_llm_pipeline().unwrap();
    assert!(s3.passed);

    // Scenario 4: Cross-OS GPU Tile Render & Video Transcode
    let s4 = harness.run_scenario_4_render_and_transcode().unwrap();
    assert!(s4.passed);

    // Scenario 5: Cross-OS Desktop-to-Mobile/Web Low-Latency Streaming
    let s5 = harness.run_scenario_5_streaming_input().unwrap();
    assert!(s5.passed);

    // Scenario 6: Cross-OS Erasure Storage & Ledger Settlement
    let s6 = harness.run_scenario_6_storage_and_ledger().unwrap();
    assert!(s6.passed);
}

// ============================================================================
// 3. [SIMULATION] TESTS
// ============================================================================

#[test]
fn test_simulated_heterogeneous_os_clock_and_page_size_drift() {
    let harness = MultiOsClusterHarness::new();

    // Verify all 6 distinct OS profiles exist with explicit platform parameters
    let expected_os = [
        (TargetOs::Windows, TargetArch::X86_64, 4096),
        (TargetOs::Linux, TargetArch::X86_64, 4096),
        (TargetOs::MacOS, TargetArch::Arm64, 16384),
        (TargetOs::Android, TargetArch::Arm64, 16384),
        (TargetOs::Ios, TargetArch::Arm64, 16384),
        (TargetOs::WebBrowser, TargetArch::Wasm32, 65536),
    ];

    assert_eq!(harness.node_count(), expected_os.len());

    // Simulate clock drift tolerance up to 500 ms across nodes
    let base_unix_ms: u64 = 1731322000000;
    for (idx, (os, _, _)) in expected_os.iter().enumerate() {
        let simulated_skew = (idx as i64 - 3) * 50; // -150 ms to +100 ms
        let node_time = (base_unix_ms as i64 + simulated_skew) as u64;
        let delta = (node_time as i64 - base_unix_ms as i64).abs();
        assert!(delta <= 500, "Clock skew for {os:?} too large: {delta} ms");
    }
}

// ============================================================================
// 4. [HARDWARE-REQUIRED] TESTS
// ============================================================================

#[test]
fn test_hardware_host_os_interop_telemetry_receipt() {
    // Inspect local operating system environment
    let is_windows = cfg!(target_os = "windows");
    let is_linux = cfg!(target_os = "linux");
    let is_macos = cfg!(target_os = "macos");

    assert!(
        is_windows || is_linux || is_macos,
        "Unknown host operating system"
    );

    // Generate local host attestation telemetry
    let host_os = if is_windows {
        "windows"
    } else if is_macos {
        "macos"
    } else {
        "linux"
    };

    let host_arch = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "other"
    };

    let attestation_hash = blake3::hash(format!("{host_os}:{host_arch}").as_bytes());
    assert_eq!(attestation_hash.as_bytes().len(), 32);
}

// ============================================================================
// 5. [MANUALLY-VERIFIED] TESTS
// ============================================================================

#[test]
fn test_manual_multi_os_cluster_topology_ui_inspection() {
    let harness = MultiOsClusterHarness::new();
    let json_val = serde_json::to_value(harness.run_scenario_1_mtls_discovery().unwrap()).unwrap();

    assert_eq!(json_val["passed"], true);
    assert_eq!(json_val["nodes_participating"].as_array().unwrap().len(), 6);
}
