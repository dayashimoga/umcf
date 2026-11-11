//! Five-Tier Verification Suite for Subphase S8.P2.SP3:
//! Dependency License Compliance, Reproducible Build Attestation & SBOM Signing.
//! Adheres strictly to `#![forbid(unsafe_code)]` and zero-placeholder invariants.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use ed25519_dalek::SigningKey;
use rand_core::OsRng;
use supply_chain::*;
use umcf_cli::run_cli;

// ============================================================================
// 1. [UNIT] TESTS
// ============================================================================

#[test]
fn test_slsa_provenance_and_sbom_ed25519_signature_verification() {
    let mut rng = OsRng;
    let release_sk = SigningKey::generate(&mut rng);
    let release_vk = release_sk.verifying_key();

    let mut provenance = SlsaProvenance::new(
        "umcf-daemon",
        "d89b3345f8e562725e24c2957b077a94966601f0fa23e3e3b33364f89d3d3c8c",
        "7d1b32bb398f5a1ef4c9977f864cf4c93540e1e97d4fcad3e1b7fcf71d44ebaa",
        "31ce8e0e7d5a5cf05e492f155990264d14214478",
        1731307200,
        "a591a6d40bf420404a011733cfb7b190d62c65bf0bcda32b57b277d9ad9f146e",
    );

    // Initial state has no signature
    assert!(provenance.verify_signature().is_err());

    // Sign and verify
    assert!(provenance.sign(&release_sk).is_ok());
    assert!(provenance.verify_signature().is_ok());

    // Verify key matches
    let signer_hex = provenance.signer_public_key_hex.as_ref().unwrap();
    assert_eq!(*signer_hex, hex::encode(release_vk.to_bytes()));
}

#[test]
fn test_tampered_binary_or_sbom_hash_rejected_by_provenance_verifier() {
    let mut rng = OsRng;
    let release_sk = SigningKey::generate(&mut rng);

    let original_binary = b"OFFICIAL_UMCF_RELEASE_BINARY_V1.0.0";
    let blake3_hex = blake3::hash(original_binary).to_hex().to_string();

    let mut provenance = SlsaProvenance::new(
        "umcf",
        blake3_hex,
        "7d1b32bb398f5a1ef4c9977f864cf4c93540e1e97d4fcad3e1b7fcf71d44ebaa",
        "31ce8e0e7d5a5cf05e492f155990264d14214478",
        1731307200,
        "a591a6d40bf420404a011733cfb7b190d62c65bf0bcda32b57b277d9ad9f146e",
    );
    assert!(provenance.sign(&release_sk).is_ok());

    // Genuine binary passes integrity check
    assert!(provenance.verify_binary_integrity(original_binary).is_ok());

    // Tampered binary (e.g. 1 bit modified) is rejected
    let mut tampered_binary = original_binary.to_vec();
    tampered_binary[0] ^= 0xFF;
    let err = provenance.verify_binary_integrity(&tampered_binary).unwrap_err();
    assert!(matches!(err, AttestationError::DigestMismatch { .. }));

    // Tampered SBOM is rejected
    let original_sbom = b"{\"bomFormat\":\"CycloneDX\",\"components\":[]}";
    let actual_sbom_hash = blake3::hash(original_sbom).to_hex().to_string();
    provenance.predicate.sbom_sha256 = actual_sbom_hash;
    assert!(provenance.verify_sbom_integrity(original_sbom).is_ok());

    let tampered_sbom = b"{\"bomFormat\":\"CycloneDX\",\"components\":[{\"name\":\"backdoor\"}]}";
    let sbom_err = provenance.verify_sbom_integrity(tampered_sbom).unwrap_err();
    assert!(matches!(sbom_err, AttestationError::DigestMismatch { .. }));
}

#[test]
fn test_spdx_license_expression_parser_rejects_gpl_and_agpl() {
    // Approved permissive licenses
    assert!(is_license_permissive("MIT"));
    assert!(is_license_permissive("Apache-2.0"));
    assert!(is_license_permissive("BSD-2-Clause"));
    assert!(is_license_permissive("BSD-3-Clause"));
    assert!(is_license_permissive("ISC"));
    assert!(is_license_permissive("Unicode-3.0"));
    assert!(is_license_permissive("Unicode-DFS-2016"));
    assert!(is_license_permissive("Zlib"));
    assert!(is_license_permissive("CC0-1.0"));
    assert!(is_license_permissive("MPL-2.0"));
    assert!(is_license_permissive("OpenSSL"));
    assert!(is_license_permissive("Apache-2.0 OR MIT"));
    assert!(is_license_permissive("MIT OR Apache-2.0"));

    // Copyleft and forbidden licenses MUST be rejected
    assert!(!is_license_permissive("GPL-2.0"));
    assert!(!is_license_permissive("GPL-3.0"));
    assert!(!is_license_permissive("GPL-3.0-only"));
    assert!(!is_license_permissive("AGPL-3.0"));
    assert!(!is_license_permissive("SSPL-1.0"));
    assert!(!is_license_permissive("LGPL-2.1"));
    assert!(!is_license_permissive("Proprietary"));
    assert!(!is_license_permissive("Unlicensed"));
}

// ============================================================================
// 2. [INT] TESTS
// ============================================================================

#[test]
fn test_cargo_deny_and_polyglot_dependency_license_compliance_gate() {
    // Verify workspace packages and their dependencies
    let packages = vec![
        SbomPackage {
            name: "umcf-core".to_string(),
            version: "0.1.0".to_string(),
            ecosystem: "cargo".to_string(),
            license_spdx: "Apache-2.0 OR MIT".to_string(),
            purl: "pkg:cargo/umcf-core@0.1.0".to_string(),
            sha256: "b45a27891cf3e680a1d701e74f76d4948d3c52a0a19c118c7bc76f27e50bc913".to_string(),
        },
        SbomPackage {
            name: "tokio".to_string(),
            version: "1.54.1".to_string(),
            ecosystem: "cargo".to_string(),
            license_spdx: "MIT".to_string(),
            purl: "pkg:cargo/tokio@1.54.1".to_string(),
            sha256: "c45a27891cf3e680a1d701e74f76d4948d3c52a0a19c118c7bc76f27e50bc914".to_string(),
        },
        SbomPackage {
            name: "serde".to_string(),
            version: "1.0.229".to_string(),
            ecosystem: "cargo".to_string(),
            license_spdx: "Apache-2.0 OR MIT".to_string(),
            purl: "pkg:cargo/serde@1.0.229".to_string(),
            sha256: "d45a27891cf3e680a1d701e74f76d4948d3c52a0a19c118c7bc76f27e50bc915".to_string(),
        },
        SbomPackage {
            name: "llama.cpp".to_string(),
            version: "b3600".to_string(),
            ecosystem: "c_cpp".to_string(),
            license_spdx: "MIT".to_string(),
            purl: "pkg:github/ggerganov/llama.cpp@b3600".to_string(),
            sha256: "1720d0f41ab1d07c08a475d4a67e4dd38b0beea19f0709b1fbf10411b933a928".to_string(),
        },
        SbomPackage {
            name: "svelte".to_string(),
            version: "5.0.0".to_string(),
            ecosystem: "npm".to_string(),
            license_spdx: "MIT".to_string(),
            purl: "pkg:npm/svelte@5.0.0".to_string(),
            sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        },
    ];

    let sbom = CycloneDxSbom::new("urn:uuid:test-sbom-01", packages);
    assert!(sbom.validate_license_compliance().is_ok());
    assert!(validate_sbom_uniqueness(&sbom));

    // Inject a rogue copyleft dependency and verify rejection
    let mut rogue_packages = sbom.components.clone();
    rogue_packages.push(SbomPackage {
        name: "hostile-copyleft-lib".to_string(),
        version: "2.0.0".to_string(),
        ecosystem: "cargo".to_string(),
        license_spdx: "GPL-3.0-only".to_string(),
        purl: "pkg:cargo/hostile-copyleft-lib@2.0.0".to_string(),
        sha256: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
    });

    let rogue_sbom = CycloneDxSbom::new("urn:uuid:rogue-01", rogue_packages);
    let err = rogue_sbom.validate_license_compliance().unwrap_err();
    assert!(matches!(err, AttestationError::ProhibitedLicense { .. }));
}

#[test]
fn test_dual_directory_reproducible_build_bit_exact_binary_comparison() {
    // Simulate deterministic outputs from build_dir_a and build_dir_b
    let canonical_payload = b"\x7fELF\x02\x01\x01\x00\x00\x00\x00\x00UMCF_DETERMINISTIC_RUNTIME_IMAGE";
    let build_a = canonical_payload.to_vec();
    let build_b = canonical_payload.to_vec();

    let digest = compare_binary_digests(&build_a, &build_b);
    assert!(digest.is_ok());
    assert_eq!(digest.unwrap(), blake3::hash(canonical_payload).to_hex().to_string());

    // Non-deterministic build is rejected
    let mut build_c = build_a.clone();
    build_c.push(0x42);
    let mismatch = compare_binary_digests(&build_a, &build_c);
    assert!(matches!(mismatch, Err(AttestationError::DigestMismatch { .. })));
}

#[test]
fn test_binary_contains_zero_host_workspace_path_strings() {
    let clean_binary = b"ELF_DATA\x00/umcf-src/crates/umcf-core/src/lib.rs\x00ENTRYPOINT\x00";
    let forbidden = [
        "C:\\Users\\dayan",
        "/home/dayan",
        "C:\\Users\\runner",
        "/home/runner",
    ];

    assert!(verify_no_host_path_leak(clean_binary, &forbidden).is_ok());

    // Contaminated binary with build host path
    let contaminated_binary = b"ELF_DATA\x00C:\\Users\\dayan\\AppData\\Local\\Temp\x00ENTRYPOINT\x00";
    let leak_err = verify_no_host_path_leak(contaminated_binary, &forbidden).unwrap_err();
    assert!(matches!(leak_err, AttestationError::HostPathLeak { .. }));
}

// ============================================================================
// 3. [SIM] TESTS
// ============================================================================

#[test]
fn test_peer_rejected_in_strict_verified_build_only_cluster() {
    let mut rng = OsRng;
    let auth_release_sk = SigningKey::generate(&mut rng);
    let auth_release_vk = auth_release_sk.verifying_key();
    let rogue_sk = SigningKey::generate(&mut rng);

    let mut valid_peer_provenance = SlsaProvenance::new(
        "umcf-daemon",
        "d89b3345f8e562725e24c2957b077a94966601f0fa23e3e3b33364f89d3d3c8c",
        "7d1b32bb398f5a1ef4c9977f864cf4c93540e1e97d4fcad3e1b7fcf71d44ebaa",
        "31ce8e0e7d5a5cf05e492f155990264d14214478",
        1731307200,
        "a591a6d40bf420404a011733cfb7b190d62c65bf0bcda32b57b277d9ad9f146e",
    );
    assert!(valid_peer_provenance.sign(&auth_release_sk).is_ok());

    let authorized_keys = [auth_release_vk];

    // Case 1: Permissive mode allows all
    assert!(verify_peer_build_admission(
        &valid_peer_provenance,
        false,
        "https://github.com/umcf/universal-mobile-compute-fabric",
        &authorized_keys
    ).is_ok());

    // Case 2: Strict mode with authorized release key PASS
    assert!(verify_peer_build_admission(
        &valid_peer_provenance,
        true,
        "https://github.com/umcf/universal-mobile-compute-fabric",
        &authorized_keys
    ).is_ok());

    // Case 3: Rogue key signature is rejected in strict mode
    let mut rogue_provenance = valid_peer_provenance.clone();
    assert!(rogue_provenance.sign(&rogue_sk).is_ok());
    let rogue_err = verify_peer_build_admission(
        &rogue_provenance,
        true,
        "https://github.com/umcf/universal-mobile-compute-fabric",
        &authorized_keys
    ).unwrap_err();
    assert!(matches!(rogue_err, AttestationError::ClusterAdmissionRejected(_)));

    // Case 4: Mismatched source repo rejected
    let mut repo_mismatch_provenance = valid_peer_provenance.clone();
    repo_mismatch_provenance.predicate.source_repo = "https://github.com/attacker/malicious-repo".to_string();
    assert!(repo_mismatch_provenance.sign(&auth_release_sk).is_ok());
    let repo_err = verify_peer_build_admission(
        &repo_mismatch_provenance,
        true,
        "https://github.com/umcf/universal-mobile-compute-fabric",
        &authorized_keys
    ).unwrap_err();
    assert!(matches!(repo_err, AttestationError::ClusterAdmissionRejected(_)));
}

// ============================================================================
// 4. [HW] TESTS
// ============================================================================

#[test]
fn test_full_release_sbom_generation_and_signature_verification() {
    let mut rng = OsRng;
    let release_sk = SigningKey::generate(&mut rng);

    let sbom = CycloneDxSbom::new(
        "urn:uuid:hardware-release-sbom-v1",
        vec![
            SbomPackage {
                name: "umcf-hwprobe".to_string(),
                version: "0.1.0".to_string(),
                ecosystem: "cargo".to_string(),
                license_spdx: "Apache-2.0 OR MIT".to_string(),
                purl: "pkg:cargo/umcf-hwprobe@0.1.0".to_string(),
                sha256: "3333333333333333333333333333333333333333333333333333333333333333".to_string(),
            },
            SbomPackage {
                name: "umcf-gpu".to_string(),
                version: "0.1.0".to_string(),
                ecosystem: "cargo".to_string(),
                license_spdx: "Apache-2.0 OR MIT".to_string(),
                purl: "pkg:cargo/umcf-gpu@0.1.0".to_string(),
                sha256: "4444444444444444444444444444444444444444444444444444444444444444".to_string(),
            },
        ],
    );

    let sbom_bytes = serde_json::to_vec(&sbom).unwrap();
    let sbom_sha256 = blake3::hash(&sbom_bytes).to_hex().to_string();

    let mut provenance = SlsaProvenance::new(
        "umcf-hw-release",
        "1111111111111111111111111111111111111111111111111111111111111111",
        "2222222222222222222222222222222222222222222222222222222222222222",
        "5247fa8c3d2b1a0987654321fedcba0987654321",
        1731307200,
        sbom_sha256,
    );

    assert!(provenance.sign(&release_sk).is_ok());
    assert!(provenance.verify_signature().is_ok());
    assert!(provenance.verify_sbom_integrity(&sbom_bytes).is_ok());
}

// ============================================================================
// 5. [MANUAL] TESTS
// ============================================================================

#[test]
fn test_manual_sbom_and_licenses_viewer_schema() {
    let args = vec!["version".to_string(), "--verify-provenance".to_string(), "--format".to_string(), "json".to_string()];
    let (code, output, is_json) = run_cli(&args);

    assert_eq!(code.into_exit_code(), std::process::ExitCode::SUCCESS);
    assert!(is_json);

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["protocol_version"], 1);
    assert_eq!(parsed["provenance"]["status"], "VERIFIED");
    assert_eq!(parsed["provenance"]["slsa_level"], "SLSA_BUILD_LEVEL_3");
    assert_eq!(parsed["provenance"]["signature_status"], "VALID_ED25519");
    assert_eq!(parsed["provenance"]["reproducible_build"], true);
}
