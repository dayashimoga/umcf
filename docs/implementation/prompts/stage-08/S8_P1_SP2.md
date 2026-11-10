# Subphase S8.P1.SP2 Execution Prompt: Network Partition, Node Churn, Thermal Throttle & Memory Pressure Chaos Tests

## 1. Role & Context
You are the Chaos Engineering & Fault-Injection Architect for UMCF. In Subphase `S8.P1.SP2`, you will implement `crates/umcf-chaos`, a Jepsen-inspired deterministic and randomized fault-injection harness that subjects an active UMCF cluster to extreme real-world mobile and network chaos while asserting strict safety and liveness invariants.

## 2. Prerequisites & Dependencies
- `S8.P1.SP1` (`MultiOsClusterHarness`).
- All Stage 1–7 fault-tolerance mechanisms (scheduler task requeue, AI pipeline failover, GPU speculative tail execution, media segment checkpoint resume, storage Reed-Solomon self-healing, and ACID ledger).

## 3. Implementation Tasks
1. **Deterministic & Randomized Chaos Nemesis Framework (`crates/umcf-chaos/src/nemesis.rs`)**:
   - Implement composable `ChaosNemesis` injectors driven by a seedable PRNG (`chaos_seed: u64` for 100% reproducible failure replay):
     1. **`NetworkPartitionNemesis`**: Injects symmetric splits (`{A,B} | {C,D,E}`), asymmetric/bridge partitions (`A` can talk to `B`, `B` can talk to `C`, but `A` cannot talk to `C`), and rapid link flapping (500 ms up / 500 ms down).
     2. **`PacketImpairmentNemesis`**: Injects 10% packet loss, 250 ms RTT spikes, packet reordering, and 1 Mbps bandwidth throttling on random links.
     3. **`NodeChurnNemesis`**: Abruptly kills (`SIGKILL` / hard transport drop) and restarts 30%–50% of worker nodes mid-computation, plus simulates mobile OS background app suspension (iOS App Nap / Android Doze freeze for 30 seconds without TCP FIN).
     4. **`ThermalAndBatteryStormNemesis`**: Simultaneously transitions 50% of mobile nodes to `ThermalState::Critical` or `BatteryState::Discharging(5%)` while they are actively executing LLM layers, GPU tiles, or video segments.
     5. **`MemoryPressureOomNemesis`**: Simulates sudden OS memory pressure (`MemoryPressure::Critical` / Android `onTrimMemory(TRIM_MEMORY_RUNNING_CRITICAL)`), forcing immediate unpinned cache eviction and graceful task migration before an OS OOM kill can occur.
2. **Continuous Invariant Checker (`crates/umcf-chaos/src/invariants.rs`)**:
   - Run `ClusterInvariantMonitor` continuously during and after every Chaos Nemesis campaign to verify 5 non-negotiable invariants:
     - **Invariant 1 (No Silent Data or Compute Corruption)**: Every completed job output matches the expected deterministic/perceptual verification hash (`S5.P3`, `S6.P1.SP3`, `S6.P2.SP3`).
     - **Invariant 2 (Bounded Split-Brain Convergence)**: Within `<= 5.0 seconds` of healing a network partition, all surviving nodes converge on an identical cluster membership view, job state table, and CRDT catalog.
     - **Invariant 3 (No Orphaned / Zombie Workloads)**: Preempted or partitioned tasks on isolated minority nodes self-abort within their lease heartbeat timeout (`<= 3.0 seconds`).
     - **Invariant 4 (Storage Durability)**: Any object stored with $RS(k, m)$ remains 100% readable as long as at least $k$ nodes survive, and automatically heals back to $k + m$ shards once replacement nodes join.
     - **Invariant 5 (Strict Ledger Conservation & No Double-Payout)**: Even if a task is executed twice due to a network partition retry, escrow settlement is strictly idempotent per task/receipt rules and $\sum \text{Debits} == \sum \text{Credits}$ holds at every millisecond.

## 4. Acceptance Criteria
- >= 95% line/branch coverage in `crates/umcf-chaos`; 100% pass rate across all 25 seeded chaos campaigns.
- Every chaos failure detected during development is reproducible via `cargo test --package umcf-chaos -- --seed <SEED>`.
- Zero deadlocks, zero panics, zero corrupted outputs, and <= 5.0s post-partition convergence across all 5 Nemesis categories running concurrently against mixed AI/GPU/Media/WASM/Storage workloads.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_asymmetric_network_partition_routing_matrix`
  - `test_deterministic_prng_chaos_schedule_reproducibility`
  - `test_invariant_monitor_detects_synthetic_split_brain_divergence`
- **`[INT]`**:
  - `test_jepsen_style_majority_minority_split_brain_and_heal_under_load`
  - `test_simultaneous_thermal_critical_and_low_battery_live_task_migration`
  - `test_mobile_background_freeze_30s_lease_expiry_and_requeue`
  - `test_critical_memory_pressure_evicts_cache_and_migrates_worker_without_oom`
- **`[SIM]`**:
  - 25-seed randomized combined Chaos Nemesis suite (Partition + Churn + Thermal + Memory Pressure) running 200 concurrent jobs across an 8-node simulated cluster.
- **`[HW]`**:
  - Physical Wi-Fi disconnect / reconnect and thermal throttling test on real hardware nodes.
- **`[MANUAL]`**:
  - Trigger "Simulate Network Partition" and "Simulate Thermal Spike" in the Svelte 5 Admin Chaos Testing Drawer and observe real-time task migration and self-healing in the UI.

## 6. Required Evidence & Artifacts
- `artifacts/stage-08/p1/sp2_chaos_25_seed_campaign_report.json`
- `artifacts/stage-08/p1/sp2_split_brain_convergence_timelines.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document Chaos Engineering architecture, lease timeouts, and split-brain resolution rules in `docs/OPERATIONS_RUNBOOK.md` (Section 2).
- Mark `S8.P1.SP2` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If any chaos seed causes a permanent cluster deadlock, split-brain double-spend, or unrecovered stalled job after network heal.
- **Completion Gate**: All 25 chaos seeds and invariants green before proceeding to `S8.P1.SP3`.
