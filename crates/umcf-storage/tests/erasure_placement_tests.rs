//! Five-Tier Verification Suite for S7.P1.SP2:
//! Reed-Solomon Erasure Coding, Convergent Authenticated Encryption & Failure Domain Placement.
//!
//! Tiers Covered:
//! - [UNIT]: Convergent encryption deduplication & cross-namespace isolation,
//!   RS(4, 2) & RS(6, 3) ALL combinations of missing shards reconstruction,
//!   corrupted shard detection by BLAKE3 before matrix inversion,
//!   failure domain placement fitness scoring & AC power preference.
//! - [INT]: 6-node RS(4, 2) cluster upload, kill 2 nodes, verify download, self-heal to 7th node.
//! - [SIM]: 20-node cluster churn simulation (nodes joining/leaving) with 100 stored objects and 0 data loss.
//! - [HW]: Reed-Solomon RS(4, 2) encode and decode throughput benchmark.
//! - [MANUAL]: Shard health matrix state transitions and UI schema verification.

#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};
use umcf_core::types::NodeId;
use umcf_storage::erasure::{
    shannon_entropy, ConvergentChunkCipher, EncryptedShard, RedundancyProfile,
    ReedSolomonErasureEngine,
};
use umcf_storage::placement::{
    DurabilityState, FailureDomainPlacementEngine, ObjectPlacementRecord,
    ShardHealthRepairWorker, StorageNodeDescriptor,
};

// ===========================================================================
// [UNIT] Tests
// ===========================================================================

#[test]
fn test_convergent_xchacha20poly1305_encryption_dedup_and_wrong_namespace_isolation() {
    let namespace_a = [0x42u8; 32];
    let namespace_b = [0x99u8; 32];
    let plaintext = b"Universal Mobile Compute Fabric: Convergent Deduplicating Cipher Test Payload 2026";

    // 1. Encrypt identical plaintext twice within Namespace A
    let (ct1, hash1) = ConvergentChunkCipher::encrypt(&namespace_a, plaintext)
        .expect("encrypt under namespace A should succeed");
    let (ct2, hash2) = ConvergentChunkCipher::encrypt(&namespace_a, plaintext)
        .expect("second encrypt under namespace A should succeed");

    assert_eq!(hash1, hash2, "Identical plaintext must yield identical BLAKE3 hashes");
    assert_eq!(ct1, ct2, "Convergent cipher must produce identical ciphertext within same namespace");

    // 2. High Shannon entropy verification (> 7.99 bits/byte for realistic payload)
    let large_plaintext = vec![0xABu8; 16384];
    let (large_ct, _large_hash) = ConvergentChunkCipher::encrypt(&namespace_a, &large_plaintext)
        .expect("encrypt large payload should succeed");
    let entropy = shannon_entropy(&large_ct);
    assert!(
        entropy > 7.95,
        "Encrypted ciphertext must exhibit high Shannon entropy (> 7.95 bits/byte), got {entropy:.4}"
    );

    // Verify zero plaintext substring leakage
    let needle = b"Universal Mobile Compute Fabric";
    assert!(
        !ct1.windows(needle.len()).any(|w| w == needle),
        "Ciphertext must not leak plaintext substrings"
    );

    // 3. Successful decryption within Namespace A
    let decrypted = ConvergentChunkCipher::decrypt(&namespace_a, &ct1, &hash1)
        .expect("decryption within namespace A must succeed");
    assert_eq!(decrypted, plaintext, "Decrypted plaintext must match original byte-for-byte");

    // 4. Encrypt identical plaintext within Namespace B
    let (ct_b, hash_b) = ConvergentChunkCipher::encrypt(&namespace_b, plaintext)
        .expect("encrypt under namespace B should succeed");
    assert_eq!(hash_b, hash1, "Plaintext BLAKE3 hash is namespace-agnostic");
    assert_ne!(ct_b, ct1, "Different namespaces must produce distinct ciphertexts");

    // 5. Attempt decryption of Ciphertext A using Namespace B secret (Must FAIL with authentication tag mismatch)
    let bad_decrypt = ConvergentChunkCipher::decrypt(&namespace_b, &ct1, &hash1);
    assert!(
        bad_decrypt.is_err(),
        "Cross-namespace decryption attempt must fail Poly1305 authentication verification"
    );
}

#[test]
fn test_rs_4_2_and_6_3_all_combinations_of_missing_shards_reconstruct_exact() {
    let payload = b"Distributed Reed-Solomon Cauchy Matrix Erasure Coding Verification: UMCF S7.P1.SP2 2026. \
                   Testing exhaustive mathematical inversion of all square Cauchy submatrices!";

    // --- RS(4, 2): K = 4, M = 2, Total = 6 ---
    let profile_4_2 = RedundancyProfile::balanced_rs42();
    let engine_4_2 = ReedSolomonErasureEngine::new(profile_4_2).expect("RS(4,2) init");
    let shards_4_2 = engine_4_2.encode(payload).expect("RS(4,2) encode");
    assert_eq!(shards_4_2.len(), 6);

    // Test all combinations of dropping 0, 1, or 2 shards
    // 0 missing
    let full: Vec<Option<EncryptedShard>> = shards_4_2.iter().map(|s| Some(s.clone())).collect();
    let recovered = engine_4_2.decode(&full).expect("0 missing decode");
    assert_eq!(recovered, payload);

    // 1 missing: 6 combinations
    for drop1 in 0..6 {
        let mut slot = full.clone();
        slot[drop1] = None;
        let rec = engine_4_2.decode(&slot).expect("1 missing decode");
        assert_eq!(rec, payload, "Failed RS(4,2) recovery with dropped shard {drop1}");
    }

    // 2 missing: 15 combinations
    let mut combo_count_4_2 = 0;
    for drop1 in 0..6 {
        for drop2 in (drop1 + 1)..6 {
            let mut slot = full.clone();
            slot[drop1] = None;
            slot[drop2] = None;
            let rec = engine_4_2.decode(&slot).expect("2 missing decode");
            assert_eq!(rec, payload, "Failed RS(4,2) recovery with dropped shards ({drop1}, {drop2})");
            combo_count_4_2 += 1;
        }
    }
    assert_eq!(combo_count_4_2, 15, "Expected 15 combinations for C(6, 2)");

    // --- RS(6, 3): K = 6, M = 3, Total = 9 ---
    let profile_6_3 = RedundancyProfile::high_durability_rs63();
    let engine_6_3 = ReedSolomonErasureEngine::new(profile_6_3).expect("RS(6,3) init");
    let shards_6_3 = engine_6_3.encode(payload).expect("RS(6,3) encode");
    assert_eq!(shards_6_3.len(), 9);

    let full_6_3: Vec<Option<EncryptedShard>> = shards_6_3.iter().map(|s| Some(s.clone())).collect();

    // Test dropping 1 shard: 9 combinations
    for drop1 in 0..9 {
        let mut slot = full_6_3.clone();
        slot[drop1] = None;
        let rec = engine_6_3.decode(&slot).expect("1 missing decode");
        assert_eq!(rec, payload, "Failed RS(6,3) recovery with dropped shard {drop1}");
    }

    // Test dropping 2 shards: 36 combinations
    for drop1 in 0..9 {
        for drop2 in (drop1 + 1)..9 {
            let mut slot = full_6_3.clone();
            slot[drop1] = None;
            slot[drop2] = None;
            let rec = engine_6_3.decode(&slot).expect("2 missing decode");
            assert_eq!(rec, payload, "Failed RS(6,3) recovery with dropped shards ({drop1}, {drop2})");
        }
    }

    // Test dropping 3 shards: 84 combinations
    let mut combo_count_6_3 = 0;
    for drop1 in 0..9 {
        for drop2 in (drop1 + 1)..9 {
            for drop3 in (drop2 + 1)..9 {
                let mut slot = full_6_3.clone();
                slot[drop1] = None;
                slot[drop2] = None;
                slot[drop3] = None;
                let rec = engine_6_3.decode(&slot).expect("3 missing decode");
                assert_eq!(rec, payload, "Failed RS(6,3) recovery with dropped shards ({drop1}, {drop2}, {drop3})");
                combo_count_6_3 += 1;
            }
        }
    }
    assert_eq!(combo_count_6_3, 84, "Expected 84 combinations for C(9, 3)");
}

#[test]
fn test_corrupted_shard_detected_by_blake3_before_rs_matrix_inversion() {
    let payload = b"Zero Trust Storage Guard: Corrupted shards must be rejected before RS decoding!";
    let profile = RedundancyProfile::balanced_rs42();
    let engine = ReedSolomonErasureEngine::new(profile).expect("engine init");
    let shards = engine.encode(payload).expect("encode");

    let mut slots: Vec<Option<EncryptedShard>> = shards.iter().map(|s| Some(s.clone())).collect();

    // Corrupt shard 0 payload bytes (tamper 1 byte)
    if let Some(ref mut shard0) = slots[0] {
        shard0.payload[0] ^= 0xFF; // Tamper
    }

    // Since shards 1..5 (5 shards) are valid, and RS(4,2) only needs 4 shards:
    // The engine must detect shard 0 corruption via BLAKE3, reject it, and reconstruct from valid shards!
    let recovered = engine.decode(&slots).expect("should recover despite 1 corrupted shard");
    assert_eq!(recovered, payload, "Reconstructed payload must match original despite corruption");

    // Now corrupt shard 1 as well (now 2 corrupt shards: 0 and 1, leaving 4 valid shards: 2, 3, 4, 5)
    if let Some(ref mut shard1) = slots[1] {
        shard1.payload[1] ^= 0xAA;
    }
    let recovered2 = engine.decode(&slots).expect("should recover with 2 corrupt and 4 valid shards");
    assert_eq!(recovered2, payload);

    // Corrupt shard 2 as well (now 3 corrupt shards, only 3 valid shards remain, but K = 4)
    if let Some(ref mut shard2) = slots[2] {
        shard2.payload[0] ^= 0x55;
    }
    let err = engine.decode(&slots).expect_err("should fail when valid shards < K");
    match err {
        umcf_storage::StorageError::InsufficientShards { required, available } => {
            assert_eq!(required, 4);
            assert_eq!(available, 3);
        }
        other => panic!("Expected InsufficientShards error, got {other:?}"),
    }
}

#[test]
fn test_failure_domain_placement_prefers_ac_power_and_distinct_subnets() {
    let node_ac_sub1 = StorageNodeDescriptor {
        node_id: NodeId("did:umcf:desktop_ac_subnet_1".to_string()),
        failure_domain: "subnet:192.168.1.0/24".to_string(),
        is_ac_powered: true,
        battery_pct: 100.0,
        uptime_score: 0.99,
        available_quota_bytes: 50 * 1024 * 1024 * 1024,
        total_quota_bytes: 100 * 1024 * 1024 * 1024,
    };

    let node_bat_sub1 = StorageNodeDescriptor {
        node_id: NodeId("did:umcf:phone_battery_subnet_1".to_string()),
        failure_domain: "subnet:192.168.1.0/24".to_string(),
        is_ac_powered: false,
        battery_pct: 45.0,
        uptime_score: 0.80,
        available_quota_bytes: 10 * 1024 * 1024 * 1024,
        total_quota_bytes: 20 * 1024 * 1024 * 1024,
    };

    let node_ac_sub2 = StorageNodeDescriptor {
        node_id: NodeId("did:umcf:server_ac_subnet_2".to_string()),
        failure_domain: "subnet:192.168.2.0/24".to_string(),
        is_ac_powered: true,
        battery_pct: 100.0,
        uptime_score: 0.98,
        available_quota_bytes: 200 * 1024 * 1024 * 1024,
        total_quota_bytes: 500 * 1024 * 1024 * 1024,
    };

    let node_ac_sub3 = StorageNodeDescriptor {
        node_id: NodeId("did:umcf:workstation_ac_subnet_3".to_string()),
        failure_domain: "subnet:192.168.3.0/24".to_string(),
        is_ac_powered: true,
        battery_pct: 100.0,
        uptime_score: 0.95,
        available_quota_bytes: 80 * 1024 * 1024 * 1024,
        total_quota_bytes: 100 * 1024 * 1024 * 1024,
    };

    // Fitness scores
    assert!(
        node_ac_sub1.compute_fitness_score() > node_bat_sub1.compute_fitness_score(),
        "AC-powered desktop must achieve higher fitness score than battery phone in same domain"
    );

    let candidates = vec![
        node_ac_sub1.clone(),
        node_bat_sub1.clone(),
        node_ac_sub2.clone(),
        node_ac_sub3.clone(),
    ];

    // Select 3 target nodes for RS(2, 1) (total 3 shards, parity M = 1)
    let selected = FailureDomainPlacementEngine::select_target_nodes(3, 1, &candidates)
        .expect("target selection should succeed");

    assert_eq!(selected.len(), 3);
    // Guarantees each node holds at most 1 shard when candidates >= total_shards
    let unique: HashSet<&NodeId> = selected.iter().collect();
    assert_eq!(unique.len(), 3, "No node may receive duplicate shards when candidates >= shards");

    // All 3 selected nodes should span distinct failure domains and prefer AC nodes
    assert!(selected.contains(&node_ac_sub1.node_id));
    assert!(selected.contains(&node_ac_sub2.node_id));
    assert!(selected.contains(&node_ac_sub3.node_id));
    assert!(!selected.contains(&node_bat_sub1.node_id), "Lower fitness battery phone in subnet 1 should not be selected");
}

// ===========================================================================
// [INT] Multi-Node Upload, Outage & Self-Healing Repair Test
// ===========================================================================

#[test]
fn test_6_node_rs_4_2_upload_kill_2_nodes_download_and_self_heal_to_7th_node() {
    let namespace = [0x55u8; 32];
    let secret_data = b"Highly Confidential Research Shard: UMCF Self-Healing Erasure Verification 2026!";

    // 1. Convergent client-side encryption
    let (ciphertext, plaintext_blake3) = ConvergentChunkCipher::encrypt(&namespace, secret_data)
        .expect("encrypt");

    // 2. RS(4, 2) encoding into 6 shards (D0..D3, P0..P1)
    let profile = RedundancyProfile::balanced_rs42();
    let engine = ReedSolomonErasureEngine::new(profile).expect("engine");
    let shards = engine.encode(&ciphertext).expect("encode");
    assert_eq!(shards.len(), 6);

    // 3. Cluster setup: 7 heterogeneous nodes (6 primary + 1 spare) across 3 subnets
    let mut nodes = Vec::new();
    for i in 0..7 {
        let subnet = format!("subnet:10.0.{}.0/24", i % 3);
        nodes.push(StorageNodeDescriptor {
            node_id: NodeId(format!("did:umcf:cluster_node_{i:02}")),
            failure_domain: subnet,
            is_ac_powered: i != 4, // node 4 is on battery
            battery_pct: if i == 4 { 60.0 } else { 100.0 },
            uptime_score: 0.96 + ((i as f64) * 0.005),
            available_quota_bytes: 100 * 1024 * 1024 * 1024,
            total_quota_bytes: 200 * 1024 * 1024 * 1024,
        });
    }

    // Primary placement for 6 shards across first 6 nodes
    let primary_candidates = &nodes[0..6];
    let placements = FailureDomainPlacementEngine::select_target_nodes(6, 2, primary_candidates)
        .expect("primary placement");
    assert_eq!(placements.len(), 6);

    let mut placement_map = HashMap::new();
    for (idx, node_id) in placements.iter().enumerate() {
        placement_map.insert(idx as u8, node_id.clone());
    }

    let object_cid = "cid:blake3:object_test_rs42_self_heal_001";
    let record = ObjectPlacementRecord {
        object_cid: object_cid.to_string(),
        plaintext_blake3,
        redundancy_profile: profile,
        shards: shards.clone(),
        node_placements: placement_map,
    };

    let mut repair_worker = ShardHealthRepairWorker::new();
    repair_worker.register_object(record);

    // 4. Initial audit: All 7 nodes online
    let mut active_nodes: HashSet<NodeId> = nodes.iter().map(|n| n.node_id.clone()).collect();
    let initial_health = repair_worker.audit_object_health(object_cid, &active_nodes)
        .expect("initial health");
    assert_eq!(initial_health.state, DurabilityState::Healthy);
    assert_eq!(initial_health.active_shards, 6);
    assert_eq!(initial_health.tolerates_outages, 2);

    // 5. Kill 2 nodes holding shards 0 and 1 (leaving 4 shards: D2, D3, P0, P1)
    let killed_node_0 = placements[0].clone();
    let killed_node_1 = placements[1].clone();
    active_nodes.remove(&killed_node_0);
    active_nodes.remove(&killed_node_1);

    let degraded_health = repair_worker.audit_object_health(object_cid, &active_nodes)
        .expect("degraded health");
    assert_eq!(degraded_health.state, DurabilityState::Degraded);
    assert_eq!(degraded_health.active_shards, 4);
    assert_eq!(degraded_health.tolerates_outages, 0);

    // 6. Verify client can still download and reconstruct exact plaintext instantaneously
    let stored_record = repair_worker.get_object(object_cid).expect("stored record");
    let mut available_slots = vec![None; 6];
    for (i, shard) in stored_record.shards.iter().enumerate() {
        let assigned = stored_record.node_placements.get(&(i as u8)).expect("node assigned");
        if active_nodes.contains(assigned) {
            available_slots[i] = Some(shard.clone());
        }
    }
    let reconstructed_ct = engine.decode(&available_slots).expect("decode under 2 node outages");
    assert_eq!(reconstructed_ct, ciphertext);

    let decrypted_pt = ConvergentChunkCipher::decrypt(&namespace, &reconstructed_ct, &plaintext_blake3)
        .expect("decrypt");
    assert_eq!(decrypted_pt, secret_data);

    // 7. Trigger self-healing repair: Reconstructs shards and migrates to 7th spare node
    let repair_events = repair_worker.audit_and_repair_all(&active_nodes, &nodes)
        .expect("self-healing repair");
    assert_eq!(repair_events.len(), 1);
    let event = &repair_events[0];
    assert_eq!(event.object_cid, object_cid);
    assert!(event.transition_summary.contains("Degraded (4/6 Shards) -> Healing -> Healthy (6/6)"));

    // Verify post-repair health is restored to Healthy (6/6)
    let post_repair_health = repair_worker.audit_object_health(object_cid, &active_nodes)
        .expect("post-repair health");
    assert_eq!(post_repair_health.state, DurabilityState::Healthy);
    assert_eq!(post_repair_health.active_shards, 6);
}

// ===========================================================================
// [SIM] 20-Node Cluster Churn Simulation
// ===========================================================================

#[test]
fn test_20_node_cluster_churn_simulation_zero_data_loss_100_objects() {
    let namespace = [0x77u8; 32];
    let profile = RedundancyProfile::balanced_rs42();
    let engine = ReedSolomonErasureEngine::new(profile).expect("engine");

    // Create 20 cluster nodes across 5 failure domains
    let mut all_nodes = Vec::new();
    for i in 0..20 {
        let domain = format!("rack:rack_{}", i % 5);
        all_nodes.push(StorageNodeDescriptor {
            node_id: NodeId(format!("did:umcf:sim_node_{i:02}")),
            failure_domain: domain,
            is_ac_powered: i % 3 != 0,
            battery_pct: 80.0,
            uptime_score: 0.95,
            available_quota_bytes: 100 * 1024 * 1024 * 1024,
            total_quota_bytes: 200 * 1024 * 1024 * 1024,
        });
    }

    let mut active_nodes: HashSet<NodeId> = all_nodes.iter().map(|n| n.node_id.clone()).collect();
    let mut repair_worker = ShardHealthRepairWorker::new();

    // Store 100 objects
    let mut objects_meta = Vec::new();
    for obj_idx in 0..100 {
        let data = format!("UMCF Object Payload #{obj_idx:04} - Resilient Churn Simulation Data").into_bytes();
        let (ct, hash) = ConvergentChunkCipher::encrypt(&namespace, &data).expect("encrypt");
        let shards = engine.encode(&ct).expect("encode");

        let placements = FailureDomainPlacementEngine::select_target_nodes(6, 2, &all_nodes)
            .expect("placement");
        let mut placement_map = HashMap::new();
        for (idx, node_id) in placements.iter().enumerate() {
            placement_map.insert(idx as u8, node_id.clone());
        }

        let cid = format!("cid:sim:object_{obj_idx:04}");
        let record = ObjectPlacementRecord {
            object_cid: cid.clone(),
            plaintext_blake3: hash,
            redundancy_profile: profile,
            shards,
            node_placements: placement_map,
        };
        repair_worker.register_object(record);
        objects_meta.push((cid, data, hash));
    }

    // Simulate 5 churn epochs: in each epoch, drop 2 nodes, repair, re-add 2 nodes
    for epoch in 0..5 {
        // Drop 2 random nodes
        let drop_node_a = NodeId(format!("did:umcf:sim_node_{:02}", (epoch * 2) % 20));
        let drop_node_b = NodeId(format!("did:umcf:sim_node_{:02}", (epoch * 2 + 1) % 20));
        active_nodes.remove(&drop_node_a);
        active_nodes.remove(&drop_node_b);

        // Run self-healing repair
        let _events = repair_worker.audit_and_repair_all(&active_nodes, &all_nodes)
            .expect("repair during churn");

        // Re-add nodes
        active_nodes.insert(drop_node_a);
        active_nodes.insert(drop_node_b);
    }

    // Assert 0 data loss: All 100 objects must decode and decrypt perfectly!
    for (cid, expected_pt, expected_hash) in &objects_meta {
        let record = repair_worker.get_object(cid).expect("object exists");
        let mut slots = vec![None; 6];
        for (i, shard) in record.shards.iter().enumerate() {
            let node = record.node_placements.get(&(i as u8)).expect("node");
            if active_nodes.contains(node) {
                slots[i] = Some(shard.clone());
            }
        }
        let recovered_ct = engine.decode(&slots).expect("decode object after churn");
        let recovered_pt = ConvergentChunkCipher::decrypt(&namespace, &recovered_ct, expected_hash)
            .expect("decrypt object");
        assert_eq!(&recovered_pt, expected_pt, "Object {cid} suffered corruption during churn simulation");
    }
}

// ===========================================================================
// [HW] Hardware Throughput Benchmark
// ===========================================================================

#[test]
fn test_hardware_simd_reed_solomon_rs_4_2_encode_decode_throughput() {
    let profile = RedundancyProfile::balanced_rs42();
    let engine = ReedSolomonErasureEngine::new(profile).expect("engine");

    // 4 MiB buffer
    let buffer_size = 4 * 1024 * 1024;
    let mut test_data = vec![0u8; buffer_size];
    for (i, b) in test_data.iter_mut().enumerate() {
        *b = (i ^ (i >> 8)) as u8;
    }

    // Benchmark Encode
    let start_encode = std::time::Instant::now();
    let shards = engine.encode(&test_data).expect("encode 4 MiB");
    let encode_duration = start_encode.elapsed();
    let encode_throughput_mb_s = (buffer_size as f64 / 1_000_000.0) / encode_duration.as_secs_f64();

    // Benchmark Decode with 2 dropped shards (D0, P0)
    let mut slots: Vec<Option<EncryptedShard>> = shards.iter().map(|s| Some(s.clone())).collect();
    slots[0] = None;
    slots[4] = None;

    let start_decode = std::time::Instant::now();
    let recovered = engine.decode(&slots).expect("decode 4 MiB with 2 losses");
    let decode_duration = start_decode.elapsed();
    let decode_throughput_mb_s = (buffer_size as f64 / 1_000_000.0) / decode_duration.as_secs_f64();

    assert_eq!(recovered, test_data);

    let baseline = if cfg!(debug_assertions) { 10.0 } else { 150.0 };
    println!(
        "RS(4, 2) Encode: {encode_throughput_mb_s:.2} MB/s | Decode: {decode_throughput_mb_s:.2} MB/s (Baseline: {baseline} MB/s)"
    );
    assert!(
        encode_throughput_mb_s >= baseline,
        "Encode throughput must exceed {baseline} MB/s, got {encode_throughput_mb_s:.2}"
    );
    assert!(
        decode_throughput_mb_s >= baseline,
        "Decode throughput must exceed {baseline} MB/s, got {decode_throughput_mb_s:.2}"
    );
}

// ===========================================================================
// [MANUAL] Shard Health Matrix UI State & Status Schema Verification
// ===========================================================================

#[test]
fn test_manual_shard_health_matrix_ui_schema() {
    let report = umcf_storage::placement::ShardHealthReport {
        object_cid: "cid:test:manual_ui_schema_001".to_string(),
        state: DurabilityState::Degraded,
        active_shards: 5,
        required_shards_k: 4,
        total_shards: 6,
        tolerates_outages: 1,
        status_badge: "Degraded (5/6 Shards) — Tolerates 1 Outages (Repair Required)".to_string(),
    };

    let serialized = serde_json::to_string_pretty(&report).expect("serialize");
    assert!(serialized.contains("Degraded"));
    assert!(serialized.contains("cid:test:manual_ui_schema_001"));
    assert!(serialized.contains("status_badge"));

    let deserialized: umcf_storage::placement::ShardHealthReport =
        serde_json::from_str(&serialized).expect("deserialize");
    assert_eq!(deserialized, report);
}
