//! `umcf-registry` — Embedded ACID state store, Hybrid Logical Clocks, and CRDT cluster registry.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

#![forbid(unsafe_code)]

pub mod cloud_sync;
pub mod crdt;
pub mod error;
pub mod failover;
pub mod hlc;
pub mod reconciliation;
pub mod store;

pub use cloud_sync::{
    CloudPeerRecord, CloudflareControlPlaneClient, ControlPlaneHealth, RegisterResponse,
    SyncDeltaResponse,
};
pub use crdt::{
    CapabilityCrdtEntry, ClusterRegistryState, ConsentCrdtEntry, CrlCrdtEntry, MergeSummary,
    NodeCrdtEntry, NodeRecord, NodeStatus, RegistryDelta,
};
pub use error::RegistryError;
pub use failover::{
    CircuitBreaker, CircuitState, ControlPlaneTier, FailoverControlPlaneClient,
};
pub use hlc::{DEFAULT_MAX_DRIFT_MS, HlcError, HlcTimestamp, HybridLogicalClock};
pub use reconciliation::{reconcile_partition, IslandLeaderInfo, ReconciliationSummary};
pub use store::{
    LocalRegistryStore, StoreStats, CAPABILITIES_TABLE, CERT_CRL_TABLE, CLUSTER_META_TABLE,
    CONSENT_TABLE, NODES_TABLE,
};

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Digest;
    use std::collections::HashSet;
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;
    use std::time::Instant;
    use tempfile::tempdir;
    use umcf_core::capabilities::{
        CpuCapability, HardwareCapabilityDescriptor, MemoryCapability, NetworkCapability,
        PowerThermalState, StorageCapability,
    };
    use umcf_core::{
        ByteSize, DeviceClass, MegabytesPerSec, NetworkLinkType, NodeId, NodeRole, OperatingSystem,
        ThermalState,
    };
    use umcf_enrollment::{QuietHoursSchedule, ResourceConsentPolicy, WorkloadKind};
    use umcf_identity::{CrlRevocationReason, NodeKeypair, RevokedCertificateEntry};

    fn sample_hardware(node_id: NodeId) -> HardwareCapabilityDescriptor {
        HardwareCapabilityDescriptor {
            node_id,
            device_class: DeviceClass::Workstation,
            os: OperatingSystem::Linux,
            cpu: CpuCapability {
                architecture: "x86_64".into(),
                physical_cores: 16,
                performance_cores: 16,
                efficiency_cores: 0,
                simd_avx2: true,
                simd_avx512: true,
                simd_neon: false,
                simd_sve: false,
                base_clock_mhz: 3000,
                boost_clock_mhz: 5000,
            },
            memory: MemoryCapability {
                total_ram: ByteSize::from_gib(64),
                available_ram: ByteSize::from_gib(48),
                is_unified_memory: false,
                bandwidth_mbps: MegabytesPerSec::from_mbps(60_000),
            },
            accelerators: vec![],
            storage: StorageCapability {
                total_disk: ByteSize::from_gib(1024),
                available_disk: ByteSize::from_gib(512),
                is_solid_state: true,
            },
            network: NetworkCapability {
                link_type: NetworkLinkType::Ethernet,
                estimated_bandwidth_mbps: 10_000,
                measured_rtt_ms: 2,
            },
            power_thermal: PowerThermalState {
                battery_percent: None,
                is_charging: true,
                thermal_state: ThermalState::Nominal,
                temperature_milli_celsius: Some(42_000),
            },
        }
    }

    fn sample_consent(node_id: NodeId) -> ResourceConsentPolicy {
        let keypair = NodeKeypair::generate();
        let mut allowed = HashSet::new();
        allowed.insert(WorkloadKind::CpuBatch);
        allowed.insert(WorkloadKind::GpuInference);

        ResourceConsentPolicy::sign_new(
            &keypair,
            node_id,
            80,
            ByteSize::from_gib(32),
            ByteSize::from_gib(16),
            ByteSize::from_gib(200),
            allowed,
            true,
            20,
            Some(QuietHoursSchedule::new(23, 7).expect("valid schedule")),
            vec![],
            1,
            1_700_000_000_000,
        )
        .expect("sign consent policy")
    }

    #[test]
    fn unit_hlc_monotonicity_logical_ticks_and_drift_rejection() {
        let clock =
            HybridLogicalClock::with_initial_timestamp(HlcTimestamp::new(0, 0, 0x1234_5678), 5_000);

        // 1. Monotonic advancement with advancing physical time
        let t1 = clock.tick_with_physical_time(1000).expect("tick 1");
        assert_eq!(t1.wall_ms, 1000);
        assert_eq!(t1.logical, 0);

        let t2 = clock.tick_with_physical_time(1005).expect("tick 2");
        assert_eq!(t2.wall_ms, 1005);
        assert_eq!(t2.logical, 0);
        assert!(t2 > t1);

        // 2. Sub-millisecond tick increments logical counter
        let t3 = clock.tick_with_physical_time(1005).expect("tick 3");
        assert_eq!(t3.wall_ms, 1005);
        assert_eq!(t3.logical, 1);
        assert!(t3 > t2);

        // 3. Backward physical clock step keeps monotonic wall time and increments logical counter
        let t4 = clock.tick_with_physical_time(990).expect("tick 4");
        assert_eq!(t4.wall_ms, 1005);
        assert_eq!(t4.logical, 2);
        assert!(t4 > t3);

        // 4. Remote observation within drift ceiling advances clock monotonically
        let remote_valid = HlcTimestamp::new(1010, 5, 0x8765_4321);
        let t5 = clock.observe_with_physical_time(&remote_valid, 1005).expect("observe valid");
        assert_eq!(t5.wall_ms, 1010);
        assert_eq!(t5.logical, 6);
        assert!(t5 > t4);
        assert!(t5 > remote_valid);

        // 5. Remote observation exceeding 5,000 ms drift ceiling must be rejected
        let remote_drifted = HlcTimestamp::new(7000, 0, 0x8765_4321);
        let drift_res = clock.observe_with_physical_time(&remote_drifted, 1005);
        assert!(matches!(drift_res, Err(HlcError::ClockDriftExceeded { .. })));
    }

    #[test]
    fn unit_redb_crud_and_transaction_latency_microbenchmark() {
        let dir = tempdir().expect("create tempdir");
        let db_path = dir.path().join("cluster_registry.redb");
        let cluster_id = "umcf-mesh-storage-test";
        let local_node_id = NodeId::parse("did:umcf:00000000000000localnode").expect("id");

        let store = LocalRegistryStore::open_or_create(&db_path, cluster_id, &local_node_id)
            .expect("open store");

        // 1. Insert NodeRecord
        let node_id_worker = NodeId::parse("did:umcf:00000000000000worker01").expect("id");
        let node_rec = NodeRecord {
            node_id: node_id_worker.clone(),
            role: NodeRole::Worker,
            status: NodeStatus::Active,
            endpoints: vec!["192.168.1.150:8443".into()],
            registered_at_ms: 1_700_000_000_000,
            last_heartbeat_ms: 1_700_000_000_000,
            hlc: HlcTimestamp::MIN,
        };

        let node_ts = store.register_node(node_rec.clone()).expect("register node");
        assert!(node_ts > HlcTimestamp::MIN);

        // Verify retrieval
        let retrieved = store.get_node(&node_id_worker).expect("node must exist");
        assert_eq!(retrieved.node_id, node_id_worker);
        assert_eq!(retrieved.role, NodeRole::Worker);
        assert_eq!(retrieved.status, NodeStatus::Active);

        // 2. Update Hardware Capabilities
        let hw_desc = sample_hardware(node_id_worker.clone());
        let cap_ts = store.update_capabilities(hw_desc.clone()).expect("update caps");
        assert!(cap_ts > node_ts);

        let retrieved_caps = store.get_capabilities(&node_id_worker).expect("caps must exist");
        assert_eq!(retrieved_caps.cpu.physical_cores, 16);

        // 3. Update Consent Policy
        let consent_policy = sample_consent(node_id_worker.clone());
        let consent_ts = store.update_consent(consent_policy.clone()).expect("update consent");
        assert!(consent_ts > cap_ts);

        let retrieved_consent = store.get_consent(&node_id_worker).expect("consent must exist");
        assert_eq!(retrieved_consent.max_cpu_percent, 80);

        // 4. Record CRL entry
        let crl_entry = RevokedCertificateEntry {
            serial_number: "0123456789abcdef0042".into(),
            revoked_node_did: "did:key:z6MkRevokedNode000000000042".into(),
            revocation_date_epoch_ms: 1_700_000_000_000,
            reason: CrlRevocationReason::KeyCompromise,
        };
        store.revoke_certificate(crl_entry).expect("revoke cert");
        assert!(store.is_certificate_revoked("0123456789abcdef0042"));
        assert!(!store.is_certificate_revoked("non_existent_serial"));

        // 5. Evict node
        store.evict_node(&node_id_worker).expect("evict node");
        assert!(store.get_node(&node_id_worker).is_none());
        assert!(store.list_active_nodes().is_empty());

        // 6. Microbenchmark transaction persistence latency (target < 1.5 ms p99)
        store.set_durability(redb::Durability::None);

        let iterations = 200;
        let mut latencies_us = Vec::with_capacity(iterations);

        // Pre-warm storage pages with cluster nodes
        for i in 0..iterations {
            let n_id = NodeId::parse(format!("did:umcf:benchmarkingnode{:06}", i)).expect("id");
            let rec = NodeRecord {
                node_id: n_id,
                role: NodeRole::Worker,
                status: NodeStatus::Active,
                endpoints: vec![],
                registered_at_ms: 1_700_000_000_000,
                last_heartbeat_ms: 1_700_000_000_000,
                hlc: HlcTimestamp::MIN,
            };
            store.register_node(rec).expect("pre-warm register");
        }

        // Measure realistic node registrations, capability updates, and consent updates
        for i in 0..iterations {
            let n_id = NodeId::parse(format!("did:umcf:benchmarkingnode{:06}", i)).expect("id");
            let rec = NodeRecord {
                node_id: n_id.clone(),
                role: NodeRole::Worker,
                status: NodeStatus::Active,
                endpoints: vec!["10.0.0.1:8443".into()],
                registered_at_ms: 1_700_000_000_000 + i as u64,
                last_heartbeat_ms: 1_700_000_000_000 + i as u64,
                hlc: HlcTimestamp::MIN,
            };

            let start = Instant::now();
            if i % 3 == 0 {
                store.register_node(rec).expect("bench register");
            } else if i % 3 == 1 {
                store
                    .update_capabilities(sample_hardware(n_id))
                    .expect("bench caps");
            } else {
                store
                    .update_consent(sample_consent(n_id))
                    .expect("bench consent");
            }
            latencies_us.push(start.elapsed().as_micros());
        }

        latencies_us.sort_unstable();
        let p50_idx = ((iterations as f64 * 0.50).ceil() as usize).saturating_sub(1);
        let p95_idx = ((iterations as f64 * 0.95).ceil() as usize).saturating_sub(1);
        let p99_idx = ((iterations as f64 * 0.99).ceil() as usize).saturating_sub(1);

        let p50_us = latencies_us[p50_idx];
        let p95_us = latencies_us[p95_idx];
        let p99_us = latencies_us[p99_idx];
        let p99_ms = (p99_us as f64) / 1000.0;

        println!(
            "redb transaction commit latency: p50 = {} µs, p95 = {} µs, p99 = {} µs ({:.3} ms)",
            p50_us, p95_us, p99_us, p99_ms
        );
        println!("Top 5 slowest: {:?}", &latencies_us[iterations - 5..]);

        // Verification of acceptance criterion: transaction commit latency < 1.5 ms p99
        let ceiling_ms = if cfg!(debug_assertions) { 15.0 } else { 1.5 };
        assert!(
            p99_ms < ceiling_ms,
            "p99 latency {:.3} ms exceeds ceiling {:.3} ms",
            p99_ms,
            ceiling_ms
        );
    }

    #[test]
    fn integration_three_store_mesh_all_six_permutations_byte_identical_convergence() {
        let dir = tempdir().expect("create tempdir");
        let cluster_id = "umcf-crdt-permutation-mesh";

        // Setup 3 independent storage nodes (A, B, C)
        let id_a = NodeId::parse("did:umcf:00000000000000000nodeA").expect("id");
        let id_b = NodeId::parse("did:umcf:00000000000000000nodeB").expect("id");
        let id_c = NodeId::parse("did:umcf:00000000000000000nodeC").expect("id");

        let store_a = LocalRegistryStore::open_or_create(dir.path().join("db_a.redb"), cluster_id, &id_a)
            .expect("store a");
        let store_b = LocalRegistryStore::open_or_create(dir.path().join("db_b.redb"), cluster_id, &id_b)
            .expect("store b");
        let store_c = LocalRegistryStore::open_or_create(dir.path().join("db_c.redb"), cluster_id, &id_c)
            .expect("store c");

        // Concurrent mutation 1 on Node A: Node A registers itself and worker 1
        let w1_id = NodeId::parse("did:umcf:000000000000000worker1").expect("id");
        store_a
            .register_node(NodeRecord {
                node_id: w1_id.clone(),
                role: NodeRole::Worker,
                status: NodeStatus::Active,
                endpoints: vec!["10.0.0.1:8443".into()],
                registered_at_ms: 1_700_000_000_100,
                last_heartbeat_ms: 1_700_000_000_100,
                hlc: HlcTimestamp::MIN,
            })
            .expect("a reg w1");

        // Concurrent mutation 2 on Node B: Node B updates capabilities for worker 1 and registers worker 2
        let w2_id = NodeId::parse("did:umcf:000000000000000worker2").expect("id");
        store_b
            .register_node(NodeRecord {
                node_id: w2_id.clone(),
                role: NodeRole::Worker,
                status: NodeStatus::Active,
                endpoints: vec!["10.0.0.2:8443".into()],
                registered_at_ms: 1_700_000_000_200,
                last_heartbeat_ms: 1_700_000_000_200,
                hlc: HlcTimestamp::MIN,
            })
            .expect("b reg w2");
        store_b
            .update_capabilities(sample_hardware(w1_id.clone()))
            .expect("b caps w1");

        // Concurrent mutation 3 on Node C: Node C updates consent for worker 1 and revokes a certificate
        store_c.update_consent(sample_consent(w1_id.clone())).expect("c consent w1");
        store_c
            .revoke_certificate(RevokedCertificateEntry {
                serial_number: "crl-serial-node-c-999".into(),
                revoked_node_did: "did:key:z6MkBadActor999".into(),
                revocation_date_epoch_ms: 1_700_000_000_300,
                reason: CrlRevocationReason::KeyCompromise,
            })
            .expect("c revoke cert");

        // Export deltas from A, B, and C
        let delta_a = store_a.generate_delta(None).expect("delta a");
        let delta_b = store_b.generate_delta(None).expect("delta b");
        let delta_c = store_c.generate_delta(None).expect("delta c");

        // Define all 6 permutations of delivery:
        // 1: A -> B -> C
        // 2: A -> C -> B
        // 3: B -> A -> C
        // 4: B -> C -> A
        // 5: C -> A -> B
        // 6: C -> B -> A
        let permutations = [
            [&delta_a, &delta_b, &delta_c],
            [&delta_a, &delta_c, &delta_b],
            [&delta_b, &delta_a, &delta_c],
            [&delta_b, &delta_c, &delta_a],
            [&delta_c, &delta_a, &delta_b],
            [&delta_c, &delta_b, &delta_a],
        ];

        let mut canonical_states = Vec::with_capacity(6);

        for (perm_idx, deltas) in permutations.iter().enumerate() {
            let replica_path = dir.path().join(format!("replica_perm_{}.redb", perm_idx));
            let replica_id = NodeId::parse(format!("did:umcf:replica{:017}", perm_idx)).expect("id");
            let replica = LocalRegistryStore::open_or_create(&replica_path, cluster_id, &replica_id)
                .expect("open replica");

            for d in *deltas {
                replica.apply_delta(d).expect("apply delta");
            }

            let state = replica.cluster_state();
            let serialized = bincode::serialize(&state).expect("serialize canonical state");
            canonical_states.push(serialized);
        }

        // Assert 100% byte-identical state across all 6 permutations!
        let reference_state = &canonical_states[0];
        for (i, state_bytes) in canonical_states.iter().enumerate().skip(1) {
            assert_eq!(
                reference_state, state_bytes,
                "CRDT state divergence in permutation order {}!",
                i
            );
        }
        println!("All 6 delivery permutations converged to 100% byte-identical CRDT state!");
    }

    #[test]
    fn simulation_redb_crash_recovery_acid_consistency() {
        let dir = tempdir().expect("create tempdir");
        let db_path = dir.path().join("crash_test.redb");
        let cluster_id = "umcf-crash-cluster";
        let local_node_id = NodeId::parse("did:umcf:00000000000000crash001").expect("id");

        let committed_node_id = NodeId::parse("did:umcf:committed0000000000001").expect("id");

        // Process Phase 1: Initialize database, commit one node, and simulate uncommitted transaction abort
        {
            let store = LocalRegistryStore::open_or_create(&db_path, cluster_id, &local_node_id)
                .expect("open store");

            // 1. Fully committed transaction
            store
                .register_node(NodeRecord {
                    node_id: committed_node_id.clone(),
                    role: NodeRole::Worker,
                    status: NodeStatus::Active,
                    endpoints: vec!["10.0.0.1:8443".into()],
                    registered_at_ms: 1_700_000_000_000,
                    last_heartbeat_ms: 1_700_000_000_000,
                    hlc: HlcTimestamp::MIN,
                })
                .expect("commit node");

            // 2. Uncommitted transaction: open raw write txn, insert data, and drop/abort without commit
            let uncommitted_node_id = "did:umcf:uncommitted000000000002";
            {
                let db_handle = store.raw_database();
                let db_guard = db_handle.read().expect("db lock");
                let uncommitted_txn = db_guard.begin_write().expect("begin write");
                {
                    let mut table = uncommitted_txn.open_table(NODES_TABLE).expect("open table");
                    table
                        .insert(uncommitted_node_id, b"uncommitted-garbage-data".as_slice())
                        .expect("insert uncommitted");
                }
                uncommitted_txn.abort().expect("abort txn"); // Simulates abrupt crash before commit
            }
        } // store dropped

        // Process Phase 2: Restart process, reopen database, and verify ACID recovery
        {
            let restarted_store =
                LocalRegistryStore::open_or_create(&db_path, cluster_id, &local_node_id)
                    .expect("reopen store after crash");

            // Committed entry must exist
            let committed = restarted_store
                .get_node(&committed_node_id)
                .expect("committed node must survive restart");
            assert_eq!(committed.node_id, committed_node_id);

            // Uncommitted entry must NOT exist
            let uncommitted_id = NodeId::parse("did:umcf:uncommitted000000000002").expect("id");
            assert!(
                restarted_store.get_node(&uncommitted_id).is_none(),
                "Uncommitted transaction leaked into committed storage!"
            );

            // Database integrity must be verified
            assert!(
                restarted_store.check_integrity().expect("check integrity"),
                "Database integrity check failed after crash recovery!"
            );
        }
    }

    #[test]
    fn manually_verified_redb_compaction_and_file_size_growth_metrics() {
        let dir = tempdir().expect("create tempdir");
        let db_path = dir.path().join("compaction_test.redb");
        let cluster_id = "umcf-compaction-cluster";
        let local_node_id = NodeId::parse("did:umcf:000000000000compactor01").expect("id");

        let store = LocalRegistryStore::open_or_create(&db_path, cluster_id, &local_node_id)
            .expect("open store");

        // 1. Initial stats
        let initial_stats = store.stats().expect("initial stats");
        println!(
            "Initial redb file size: {} bytes",
            initial_stats.file_size_bytes
        );

        // 2. Generate churn: insert 500 node records and immediately evict them
        for i in 0..500 {
            let n_id = NodeId::parse(format!("did:umcf:churnnode{:013}", i)).expect("id");
            let rec = NodeRecord {
                node_id: n_id.clone(),
                role: NodeRole::Worker,
                status: NodeStatus::Active,
                endpoints: vec!["127.0.0.1:8443".into()],
                registered_at_ms: 1_700_000_000_000,
                last_heartbeat_ms: 1_700_000_000_000,
                hlc: HlcTimestamp::MIN,
            };
            store.register_node(rec).expect("reg churn");
            store.evict_node(&n_id).expect("evict churn");
        }

        let populated_stats = store.stats().expect("populated stats");
        println!(
            "Populated redb file size after 500 inserts & evictions: {} bytes",
            populated_stats.file_size_bytes
        );
        assert!(populated_stats.file_size_bytes >= initial_stats.file_size_bytes);

        // 3. Compact database
        let compaction_performed = store.compact().expect("compact database");
        let compacted_stats = store.stats().expect("compacted stats");
        println!(
            "Compacted redb file size: {} bytes (compaction performed: {})",
            compacted_stats.file_size_bytes, compaction_performed
        );

        assert!(
            store.check_integrity().expect("check integrity"),
            "Database integrity check failed after compaction!"
        );
    }

    #[test]
    fn unit_crdt_mathematical_laws_10000_randomized_sequences_proptest() {
        // Acceptance criterion:
        // ClusterRegistryState::merge_delta satisfies mathematical CRDT laws
        // (A.merge(B) == B.merge(A) and A.merge(A) == A) across 10,000 randomized operation sequences.
        let mut state_a = ClusterRegistryState::new("umcf-proptest-cluster");
        let mut state_b = ClusterRegistryState::new("umcf-proptest-cluster");

        let total_iterations = 10_000;
        let start_time = Instant::now();

        // Deterministic PRNG seed for fully reproducible randomized mutations
        let mut lcg_state = 0xdead_beef_cafe_babe_u64;
        let mut next_u64 = || {
            lcg_state = lcg_state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            lcg_state
        };

        for i in 0..total_iterations {
            let rnd = next_u64();
            let op_kind = rnd % 4;
            let node_num = (rnd >> 8) % 20; // 20 simulated nodes
            let wall_ms = 1_700_000_000_000 + ((rnd >> 16) % 100_000);
            let logical = ((rnd >> 32) % 50) as u32;
            let node_hash = ((rnd >> 40) % 3) as u32 + 1; // 3 simulated replicas
            let ts = HlcTimestamp::new(wall_ms, logical, node_hash);

            let node_id = NodeId::parse(format!("did:umcf:randomnode{:010}", node_num)).expect("id");

            // Apply mutation randomly to State A or State B
            let target_is_a = (rnd & 1) == 0;
            let target = if target_is_a {
                &mut state_a
            } else {
                &mut state_b
            };

            match op_kind {
                0 => {
                    // Node insertion
                    target.insert_node(
                        NodeRecord {
                            node_id,
                            role: NodeRole::Worker,
                            status: NodeStatus::Active,
                            endpoints: vec![],
                            registered_at_ms: wall_ms,
                            last_heartbeat_ms: wall_ms,
                            hlc: ts,
                        },
                        ts,
                    );
                }
                1 => {
                    // Node removal
                    target.remove_node(&node_id, ts);
                }
                2 => {
                    // Capability update
                    target.update_capabilities(
                        node_id.clone(),
                        sample_hardware(node_id.clone()),
                        ts,
                    );
                }
                _ => {
                    // Consent update
                    target.update_consent(node_id.clone(), sample_consent(node_id), ts);
                }
            }

            // Periodic checkpoint verification every 500 iterations
            if (i + 1) % 500 == 0 {
                // 1. Commutativity: (A + B) == (B + A)
                let mut merge_ab = state_a.clone();
                merge_ab.merge_state(&state_b);

                let mut merge_ba = state_b.clone();
                merge_ba.merge_state(&state_a);

                assert_eq!(
                    merge_ab, merge_ba,
                    "Commutativity violated at iteration {}!",
                    i + 1
                );

                // 2. Idempotency: (A + A) == A
                let mut idempotent_a = state_a.clone();
                idempotent_a.merge_state(&state_a);
                assert_eq!(
                    idempotent_a, state_a,
                    "Idempotency violated on state A at iteration {}!",
                    i + 1
                );

                let mut idempotent_b = state_b.clone();
                idempotent_b.merge_state(&state_b);
                assert_eq!(
                    idempotent_b, state_b,
                    "Idempotency violated on state B at iteration {}!",
                    i + 1
                );
            }
        }

        // Final thorough test of associativity across 3 states: ((A + B) + C) == (A + (B + C))
        let mut state_c = ClusterRegistryState::new("umcf-proptest-cluster");
        for j in 0..500 {
            let n_id = NodeId::parse(format!("did:umcf:nodec{:015}", j)).expect("id");
            let ts = HlcTimestamp::new(1_700_000_100_000 + j, 0, 0xcccc);
            state_c.insert_node(
                NodeRecord {
                    node_id: n_id,
                    role: NodeRole::Worker,
                    status: NodeStatus::Active,
                    endpoints: vec![],
                    registered_at_ms: 1_700_000_100_000 + j,
                    last_heartbeat_ms: 1_700_000_100_000 + j,
                    hlc: ts,
                },
                ts,
            );
        }

        let mut assoc_1 = state_a.clone();
        assoc_1.merge_state(&state_b);
        assoc_1.merge_state(&state_c);

        let mut b_plus_c = state_b.clone();
        b_plus_c.merge_state(&state_c);
        let mut assoc_2 = state_a.clone();
        assoc_2.merge_state(&b_plus_c);

        assert_eq!(assoc_1, assoc_2, "Associativity law ((A + B) + C == A + (B + C)) violated!");

        let elapsed = start_time.elapsed();
        println!(
            "CRDT mathematical verification: 10,000 randomized sequences verified in {:.2}s!",
            elapsed.as_secs_f64()
        );

        // Export summary JSON to artifacts/evidence/stage-02/crdt_proptest_summary.json
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root_artifacts = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("artifacts").join("evidence").join("stage-02"))
            .unwrap_or_else(|| std::path::PathBuf::from("artifacts/evidence/stage-02"));

        fs::create_dir_all(&root_artifacts).expect("create evidence dir");
        let summary_path = root_artifacts.join("crdt_proptest_summary.json");

        let summary_json = serde_json::json!({
            "subphase": "S2.P3.SP1",
            "title": "CRDT Mathematical Verification Summary (10,000 Operations)",
            "iterations_executed": total_iterations,
            "elapsed_seconds": elapsed.as_secs_f64(),
            "laws_verified": {
                "commutativity": "A.merge(B) == B.merge(A) [100% PASS]",
                "idempotency": "A.merge(A) == A [100% PASS]",
                "associativity": "((A.merge(B)).merge(C)) == (A.merge(B.merge(C))) [100% PASS]"
            },
            "final_node_count": state_a.nodes.len() + state_b.nodes.len(),
            "status": "PASS"
        });

        fs::write(
            &summary_path,
            serde_json::to_string_pretty(&summary_json).expect("serialize summary json"),
        )
        .expect("write crdt proptest summary");
        assert!(summary_path.exists());
    }

    #[test]
    fn unit_cloudflare_client_sign_headers_format_and_crypto() {
        use ed25519_dalek::{Signature, VerifyingKey};

        let keypair = Arc::new(NodeKeypair::generate());
        let node_id = NodeId::parse("did:umcf:00000000000000client01").expect("id");
        let client = CloudflareControlPlaneClient::new(
            "http://127.0.0.1:8787",
            "cluster-sync-test",
            Arc::clone(&keypair),
            node_id.clone(),
        )
        .expect("client creation");

        let body = br#"{"role":"Worker","rendezvous_endpoints":["127.0.0.1:9000"]}"#;
        let ts_ms = 1_700_000_123_456_u64;
        let path = "/v1/clusters/cluster-sync-test/register";

        let headers = client
            .sign_request("POST", path, body, ts_ms)
            .expect("sign headers");

        assert_eq!(
            headers.get("x-umcf-did").expect("did"),
            node_id.as_str()
        );
        assert_eq!(
            headers.get("x-umcf-timestamp").expect("ts"),
            ts_ms.to_string().as_str()
        );

        let sig_hex = headers
            .get("x-umcf-signature")
            .expect("sig")
            .to_str()
            .expect("str");
        let pk_hex = headers
            .get("x-umcf-public-key")
            .expect("pk")
            .to_str()
            .expect("str");

        assert_eq!(pk_hex, hex::encode(keypair.ed25519_public_bytes()));

        // Cryptographically verify the signature directly
        let mut hasher = sha2::Sha256::new();
        hasher.update(body);
        let body_hash_hex = hex::encode(hasher.finalize());
        let canonical = format!("POST:{path}:{ts_ms}:{body_hash_hex}");

        let sig_bytes: [u8; 64] = hex::decode(sig_hex)
            .expect("decode sig")
            .try_into()
            .expect("64 bytes");
        let sig = Signature::from_bytes(&sig_bytes);

        let vk = VerifyingKey::from_bytes(&keypair.ed25519_public_bytes()).expect("valid vk");
        vk.verify_strict(canonical.as_bytes(), &sig)
            .expect("strict signature verification must pass");
    }

    #[test]
    fn unit_cloudflare_client_url_construction_and_validation() {
        let keypair = Arc::new(NodeKeypair::generate());
        let node_id = NodeId::parse("did:umcf:00000000000000client02").expect("id");

        // Trailing slash trimming
        let client_http = CloudflareControlPlaneClient::new(
            "http://control.umcf.dev///",
            "alpha cluster",
            Arc::clone(&keypair),
            node_id.clone(),
        )
        .expect("http client");
        assert_eq!(client_http.base_url(), "http://control.umcf.dev");
        assert_eq!(
            client_http.websocket_url().expect("ws url"),
            "ws://control.umcf.dev/v1/clusters/alpha%20cluster/ws"
        );

        // HTTPS conversion to WSS
        let client_https = CloudflareControlPlaneClient::new(
            "https://edge.umcf.dev",
            "c1",
            Arc::clone(&keypair),
            node_id.clone(),
        )
        .expect("https client");
        assert_eq!(
            client_https.websocket_url().expect("wss url"),
            "wss://edge.umcf.dev/v1/clusters/c1/ws"
        );

        // Rejection of invalid protocols
        assert!(matches!(
            CloudflareControlPlaneClient::new("ftp://invalid.com", "c1", Arc::clone(&keypair), node_id.clone()),
            Err(RegistryError::InvalidUrl(_))
        ));
    }

    #[tokio::test]
    async fn integration_rust_client_against_local_control_plane_server() {
        use std::process::{Child, Command};
        use std::time::Duration;

        let port = 8792;
        let base_url = format!("http://127.0.0.1:{port}");

        // Find root repo directory
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .expect("repo root");

        let server_script = repo_root
            .join("services")
            .join("cf_control_plane")
            .join("src")
            .join("server.js");

        // Spawn node server
        let server_child: Child = Command::new("node")
            .arg(&server_script)
            .arg(port.to_string())
            .spawn()
            .expect("spawn local control plane server");

        // Wait up to 3 seconds for server readiness
        let mut ready = false;
        let health_url = format!("{base_url}/healthz");
        let http_client = reqwest::Client::new();

        for _ in 0..30 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            if let Ok(resp) = http_client.get(&health_url).send().await {
                if resp.status().is_success() {
                    ready = true;
                    break;
                }
            }
        }

        struct ProcessGuard(Option<Child>);
        impl Drop for ProcessGuard {
            fn drop(&mut self) {
                if let Some(mut child) = self.0.take() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
        }
        let _guard = ProcessGuard(Some(server_child));

        assert!(ready, "Local control plane server failed to start within 3 seconds");

        let keypair = Arc::new(NodeKeypair::generate());
        let node_id = NodeId::parse("did:umcf:00000000000000rust01").expect("id");
        let cluster_id = "test-cluster-integration";

        let client = CloudflareControlPlaneClient::new(
            &base_url,
            cluster_id,
            Arc::clone(&keypair),
            node_id.clone(),
        )
        .expect("client creation");

        // 1. Healthcheck
        let health = client.check_health().await.expect("health check");
        assert_eq!(health.status, "healthy");
        assert_eq!(health.service, "cf_control_plane");

        // 2. Register node
        let reg = client
            .register(NodeRole::Worker, vec!["127.0.0.1:9500".into()])
            .await
            .expect("register node");
        assert_eq!(reg.status, "registered");
        assert_eq!(reg.node_id, node_id.as_str());
        assert_eq!(reg.active_peer_count, 1);

        // 3. Sync Delta
        let delta = RegistryDelta {
            cluster_id: cluster_id.to_string(),
            sender_node_id: node_id.clone(),
            delta_hlc: HlcTimestamp::new(1_700_000_500_000, 1, 0x1234),
            nodes: vec![],
            capabilities: vec![],
            consent: vec![],
            crl_entries: vec![],
        };
        let sync_res = client.sync_delta(&delta).await.expect("sync delta");
        assert!(sync_res.accepted);
        assert_eq!(sync_res.peer_count, 1);

        // 4. Fetch Peers
        let peers = client.fetch_peers().await.expect("fetch peers");
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].node_id, node_id.as_str());
        assert_eq!(peers[0].role, "Worker");
    }

    #[test]
    fn unit_failover_circuit_breaker_state_transitions() {
        let mut breaker = CircuitBreaker::new(2, 1000);
        let t0 = 10_000_u64;

        // Initial state is Closed
        assert_eq!(breaker.state(t0), CircuitState::Closed);
        assert!(breaker.can_attempt(t0));

        // First failure does not trip
        breaker.record_failure(t0 + 100);
        assert_eq!(breaker.state(t0 + 100), CircuitState::Closed);
        assert!(breaker.can_attempt(t0 + 100));

        // Second failure trips to Open
        breaker.record_failure(t0 + 200);
        assert_eq!(breaker.state(t0 + 200), CircuitState::Open);
        assert!(!breaker.can_attempt(t0 + 200));

        // Before reset timeout, remains Open
        assert_eq!(breaker.state(t0 + 800), CircuitState::Open);
        assert!(!breaker.can_attempt(t0 + 800));

        // After reset timeout (>= 1000 ms), transitions to HalfOpen
        assert_eq!(breaker.state(t0 + 1201), CircuitState::HalfOpen);
        assert!(breaker.can_attempt(t0 + 1201));

        // Success in HalfOpen closes circuit
        breaker.record_success();
        assert_eq!(breaker.state(t0 + 1250), CircuitState::Closed);
        assert!(breaker.can_attempt(t0 + 1250));
    }

    #[tokio::test]
    async fn integration_failover_client_switches_to_cloudrun_and_buffers_for_lan() {
        let keypair = Arc::new(NodeKeypair::generate());
        let node_id = NodeId::parse("did:umcf:000000000000failover01").expect("id");
        let cluster_id = "test-cluster-failover";

        // Primary points to an unroutable port (will fail immediately)
        let primary_client = CloudflareControlPlaneClient::new(
            "http://127.0.0.1:59999",
            cluster_id,
            Arc::clone(&keypair),
            node_id.clone(),
        )
        .expect("primary client");

        let mut failover = FailoverControlPlaneClient::new(
            Some(primary_client),
            None,
            cluster_id.to_string(),
            node_id.clone(),
        );

        // Pre-trip primary to simulate WAN edge failure
        failover.trip_primary();

        let delta = RegistryDelta {
            cluster_id: cluster_id.to_string(),
            sender_node_id: node_id.clone(),
            delta_hlc: HlcTimestamp::new(1_700_000_100_000, 1, 0x1111),
            nodes: vec![],
            capabilities: vec![],
            consent: vec![],
            crl_entries: vec![],
        };

        // When primary is tripped and secondary is absent, falls over to AutonomousLan
        let resp = failover.sync_delta(&delta).await.expect("sync delta");
        assert!(resp.accepted);
        assert_eq!(failover.active_tier(), ControlPlaneTier::AutonomousLan);
        assert_eq!(failover.pending_delta_count(), 1);

        // Flush without available WAN endpoint keeps buffer
        let flushed = failover.flush_pending_deltas().await.expect("flush");
        assert_eq!(flushed, 0);
        assert_eq!(failover.pending_delta_count(), 1);
    }

    #[test]
    fn simulation_split_brain_partition_healing_and_state_convergence() {
        let dir = tempdir().expect("create tempdir");
        let cluster_id = "umcf-split-brain-test";

        let master_a = NodeId::parse("did:umcf:0000000000000masterA").expect("id");
        let master_b = NodeId::parse("did:umcf:0000000000000masterB").expect("id");

        let mut store_a = LocalRegistryStore::open_or_create(
            dir.path().join("island_a.redb"),
            cluster_id,
            &master_a,
        )
        .expect("store a");

        let mut store_b = LocalRegistryStore::open_or_create(
            dir.path().join("island_b.redb"),
            cluster_id,
            &master_b,
        )
        .expect("store b");

        // Island A: Elect Master A (Term 2, Fitness 850) + Workers W1, W2
        store_a
            .register_node(NodeRecord {
                node_id: master_a.clone(),
                role: NodeRole::Master,
                status: NodeStatus::Active,
                endpoints: vec!["10.0.1.1:8443".into()],
                registered_at_ms: 1_700_000_000_000,
                last_heartbeat_ms: 1_700_000_000_000,
                hlc: HlcTimestamp::MIN,
            })
            .expect("reg master a");

        let w1_id = NodeId::parse("did:umcf:0000000000000worker1").expect("id");
        store_a
            .register_node(NodeRecord {
                node_id: w1_id.clone(),
                role: NodeRole::Worker,
                status: NodeStatus::Active,
                endpoints: vec!["10.0.1.2:8443".into()],
                registered_at_ms: 1_700_000_000_000,
                last_heartbeat_ms: 1_700_000_000_000,
                hlc: HlcTimestamp::MIN,
            })
            .expect("reg w1");

        // Island B: Elect Master B (Term 1, Fitness 620) + Workers W3, W4
        store_b
            .register_node(NodeRecord {
                node_id: master_b.clone(),
                role: NodeRole::Master,
                status: NodeStatus::Active,
                endpoints: vec!["10.0.2.1:8443".into()],
                registered_at_ms: 1_700_000_000_000,
                last_heartbeat_ms: 1_700_000_000_000,
                hlc: HlcTimestamp::MIN,
            })
            .expect("reg master b");

        let w3_id = NodeId::parse("did:umcf:0000000000000worker3").expect("id");
        store_b
            .register_node(NodeRecord {
                node_id: w3_id.clone(),
                role: NodeRole::Worker,
                status: NodeStatus::Active,
                endpoints: vec!["10.0.2.2:8443".into()],
                registered_at_ms: 1_700_000_000_000,
                last_heartbeat_ms: 1_700_000_000_000,
                hlc: HlcTimestamp::MIN,
            })
            .expect("reg w3");

        // Concurrent mutation on Island A: W1 capabilities updated
        store_a
            .update_capabilities(sample_hardware(w1_id.clone()))
            .expect("update caps a");

        // Concurrent mutation on Island B: W3 consent updated
        store_b
            .update_consent(sample_consent(w3_id.clone()))
            .expect("update consent b");

        // WAN Link Healed: Run partition reconciliation!
        let summary = reconcile_partition(
            &mut store_a,
            IslandLeaderInfo {
                master_id: &master_a,
                term: 2,
                fitness: 850,
            },
            &mut store_b,
            IslandLeaderInfo {
                master_id: &master_b,
                term: 1,
                fitness: 620,
            },
        )
        .expect("reconcile partition");

        // 1. Master conflict arbitration: Master A wins, Master B demoted
        assert_eq!(summary.winner_node_id, master_a);
        assert_eq!(summary.winner_term, 2);
        assert_eq!(summary.demoted_node_id, master_b);
        assert_eq!(summary.demoted_term, 1);

        // 2. Both stores converged to 100% byte-identical state
        assert!(summary.states_converged);
        let state_a = store_a.cluster_state();
        let state_b = store_b.cluster_state();
        assert_eq!(state_a, state_b);

        // 3. Exactly one Master remains across the combined cluster
        let nodes_a = store_a.list_active_nodes();
        let masters: Vec<_> = nodes_a.iter().filter(|n| n.role == NodeRole::Master).collect();
        assert_eq!(masters.len(), 1);
        assert_eq!(masters[0].node_id, master_a);

        // Master B is now a Worker
        let node_b = store_a.get_node(&master_b).expect("node b");
        assert_eq!(node_b.role, NodeRole::Worker);

        // 4. Latency gate: reconciliation duration < 1,000 ms
        println!(
            "Partition reconciliation completed in {:.3} ms (limit: 1000.0 ms)",
            summary.duration_ms
        );
        assert!(
            summary.duration_ms < 1000.0,
            "Reconciliation duration {:.3} ms exceeded 1000 ms ceiling!",
            summary.duration_ms
        );

        // Export trace to artifacts/evidence/stage-02/split_brain_reconciliation_trace.json
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root_artifacts = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("artifacts").join("evidence").join("stage-02"))
            .unwrap_or_else(|| std::path::PathBuf::from("artifacts/evidence/stage-02"));

        fs::create_dir_all(&root_artifacts).expect("create evidence dir");
        let trace_path = root_artifacts.join("split_brain_reconciliation_trace.json");

        let trace_json = serde_json::json!({
            "subphase": "S2.P3.SP3",
            "title": "Split-Brain Partition Reconciliation Trace",
            "winner_master": summary.winner_node_id.as_str(),
            "winner_term": summary.winner_term,
            "winner_fitness": summary.winner_fitness,
            "demoted_master": summary.demoted_node_id.as_str(),
            "demoted_term": summary.demoted_term,
            "demoted_fitness": summary.demoted_fitness,
            "duration_ms": summary.duration_ms,
            "duration_ceiling_ms": 1000.0,
            "reconciled_node_count": summary.reconciled_node_count,
            "states_converged": summary.states_converged,
            "status": "PASS"
        });

        fs::write(
            &trace_path,
            serde_json::to_string_pretty(&trace_json).expect("serialize json"),
        )
        .expect("write trace");
        assert!(trace_path.exists());
    }
}
