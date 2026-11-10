# Phase S8.P1 Execution Prompt: Cross-Platform End-to-End Integration & Chaos Resilience Testing

## 1. Role & Context
You are the Principal Distributed Systems Reliability & Chaos Engineering Lead for UMCF. Phase `S8.P1` validates that the entire Unified Mobile-First Compute Fabric operates seamlessly as a single cohesive organism across heterogeneous operating systems (Android, iOS, Windows, Linux, macOS, and Web browsers) while surviving adversarial real-world failure modes: sudden Wi-Fi/cellular drops, asymmetric network partitions, split-brain heals, mobile OS background freezes, thermal throttling, OOM memory pressure, and multi-day continuous load.

This phase consists of three sequential subphases:
1. `S8_P1_SP1.md`: Multi-OS Cluster Integration Suite (Android, iOS, Windows, Linux, macOS, Web)
2. `S8_P1_SP2.md`: Network Partition, Node Churn, Thermal Throttle & Memory Pressure Chaos Tests
3. `S8_P1_SP3.md`: Long-Haul Soak Testing & Zero-Leak Memory/FD Profiling

## 2. Prerequisites & Dependencies
- Completion of Stages 1–7 (all crates, SDKs, CLI, and UIs complete).

## 3. Implementation Tasks
1. **Subphase S8.P1.SP1 (`tests/e2e-cluster/src/multi_os_suite.rs`)**:
   - Implement the automated cross-OS compatibility & endianness/ABI interop harness testing mixed clusters of Windows (`x86_64`), Linux (`x86_64`/`aarch64`), macOS (`arm64`), Android (`arm64-v8a`), iOS (`arm64`), and Web (`wasm32`/WebRTC/WebGPU) executing all 6 workload families simultaneously.
2. **Subphase S8.P1.SP2 (`crates/umcf-chaos/src/nemesis.rs`)**:
   - Implement deterministic & randomized Chaos Nemesis injectors (`NetworkPartitionNemesis`, `PacketDropJitterNemesis`, `NodeKillChurnNemesis`, `ThermalThrottleSpikeNemesis`, `MemoryPressureOomNemesis`, `ClockSkewNemesis`) verifying safety invariants (zero data loss, zero ledger imbalance, bounded recovery time) under concurrent faults.
3. **Subphase S8.P1.SP3 (`tests/soak/src/soak_runner.rs`)**:
   - Implement the continuous multi-workload soak test runner with automated RSS heap, VRAM allocation, file descriptor/socket handle, and tokio task count tracking to mathematically prove zero resource leaks over 10,000+ job lifecycles.

## 4. Acceptance Criteria
- >= 95% line/branch coverage across all integration/chaos harnesses and zero regressions in core crates; 100% test pass rate.
- Mixed-OS cluster executes concurrent WASM, Distributed LLM, GPU Render, Media Transcode, Remote Stream, and Storage workflows with 100% deterministic output verification.
- After healing a 3-way asymmetric network partition (split-brain), cluster gossip/CRDT state, storage shard health, and ledger balances converge across 100% of nodes within <= 5.0 seconds.
- Over 10,000 consecutive full job lifecycles in the soak harness, RSS memory growth slope is `<= 0.0 KB/job` after warmup (zero unbounded growth) and open file descriptors/sockets return to exact baseline when idle.

## 5. Required Tests
- **`[UNIT]`**: Nemesis fault-schedule generator, linear-regression memory leak detector, FD/handle baseline comparator.
- **`[INT]`**: 6-platform wire protocol & cryptographic receipt cross-verification suite; concurrent multi-workload cluster execution.
- **`[SIM]`**: 50-scenario Chaos Nemesis matrix (split-brain, flapping links, 50% node churn, thermal critical storm, synthetic OOM pressure).
- **`[HW]`**: Physical multi-OS device cluster integration test and multi-hour hardware soak run.
- **`[MANUAL]`**: Observe Chaos Dashboard in Svelte 5 while triggering live node disconnects and thermal/battery overrides.

## 6. Required Evidence & Artifacts
- `artifacts/stage-08/p1/multi_os_interop_matrix.json`
- `artifacts/stage-08/p1/chaos_nemesis_convergence_report.json`
- `artifacts/stage-08/p1/soak_rss_fd_vram_time_series.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document Chaos Nemesis test scenarios and failure recovery SLAs in `docs/OPERATIONS_RUNBOOK.md` (Section 1).
- Mark `S8.P1.SP1`–`S8.P1.SP3` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: Any deadlock, split-brain ledger divergence, unrecovered orphaned task, or positive linear memory/FD leak slope during soak testing.
- **Gate**: All 3 subphases (`S8_P1_SP1`..`SP3`) pass before starting Phase `S8.P2`.
