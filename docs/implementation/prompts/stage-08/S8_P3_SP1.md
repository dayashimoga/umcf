# Subphase S8.P3.SP1 Execution Prompt: Data-Plane Zero-Copy Optimization & Mobile Battery Impact Tuning

## 1. Role & Context
You are the Systems Performance & Mobile Energy Efficiency Architect for UMCF. In Subphase `S8.P3.SP1`, you will implement zero-copy data-plane optimizations in `crates/umcf-net/src/zero_copy.rs` and `crates/umcf-core/src/buffer_pool.rs`, and the mobile battery & radio wakeup governor in `crates/umcf-hw/src/power_governor.rs`. High-speed AI activation transfers, 4K60 remote game streaming, and multi-gigabyte GGUF/storage shard syncs must saturate Wi-Fi 6 / 10GbE links with minimal CPU overhead, while idle mobile nodes must consume negligible battery (`< 1.0%/hour`).

## 2. Prerequisites & Dependencies
- Stages 1–7 and Phases `S8.P1`–`S8.P2` complete.
- `criterion` benchmark harness and allocation counting hooks.

## 3. Implementation Tasks
1. **Zero-Copy Data-Plane & Page-Aligned Buffer Pool (`crates/umcf-core/src/buffer_pool.rs`, `crates/umcf-net/src/zero_copy.rs`)**:
   - Implement `AlignedSlabBufferPool` providing reusable, cache-line (`64B`) and page-aligned (`4096B` / `16384B`) pinned byte buffers backed by custom `Bytes` owners (`Bytes::from_owner`) so buffers return automatically to the slab pool when their last reference drops—eliminating `malloc`/`free` churn on every network packet or tensor activation.
   - Audit and refactor the 4 hottest data paths to guarantee **zero intermediate payload cloning**:
     1. **Storage & Model Shard Read -> QUIC Send**: Slice directly from `VerifiedModelMmap` / chunk mmap via `Bytes::from_owner(MmapSliceGuard)` into `quinn::SendStream::write_chunk(Bytes)`.
     2. **AI Pipeline Activation Transfer**: Read GGML backend tensor data directly into an `AlignedSlabBufferPool` lease and pass `IoSlice` vectored writes (`write_vectored`) to the QUIC stream without copying between temporary `Vec<u8>` buffers.
     3. **Remote Streaming Capture -> Encode -> Datagram**: Pass GPU staging/encoder output bitstream slices directly to `ReedSolomonFec` packetizer using sub-slice views (`Bytes::slice(offset..offset+mtu)`).
     4. **Local Loopback IPC (`127.0.0.1` CLI/UI/OpenAI API)**: Use OS shared-memory ring buffers / zero-copy splice when producer and consumer reside on the same host.
2. **Mobile Battery & Radio Wakeup Governor (`crates/umcf-hw/src/power_governor.rs`)**:
   - Implement `MobilePowerGovernor` with 4 automatic power states driven by `umcf-hw` battery/charging/screen/network telemetry:
     - **`AcPoweredUnrestricted`**: Plugged into charger + Wi-Fi/Ethernet -> Full compute contribution, 5s gossip heartbeat, proactive storage scrub & shard repair enabled.
     - **`BatteryActiveInteractive`**: On battery + user actively using UMCF app -> Local + cluster requests enabled, 15s gossip heartbeat, background storage repair paused.
     - **`BatteryBackgroundEco` (Default when app backgrounded on battery `> 25%`)**: Coalesces all network keepalives, DHT refreshes, and telemetry pulses into a single synchronized **60-second radio wakeup window** (`TimerCoalescer`), allowing cellular/Wi-Fi radios to enter deep PSM/sleep between ticks; rejects heavy GPU/Media compute tasks while serving lightweight metadata queries.
     - **`BatteryDeepSaver` (Battery `<= 20%` or OS Low Power Mode active)**: Suspends all incoming remote worker tasks, switches to passive client-only mode, and drops background wakeup frequency to 300 seconds (or push-notification wakeup only).

## 4. Acceptance Criteria
- >= 95% line/branch coverage in `buffer_pool.rs`, `zero_copy.rs`, and `power_governor.rs`; 100% test pass rate.
- **Zero-Copy Pointer Identity**: Unit/integration tests verify that slicing a 4 MiB shard from `VerifiedModelMmap` into `MediaShardPacket` / QUIC frame chunks preserves the exact underlying memory address (`chunk_slice.as_ptr()` lies within `mmap.as_ptr()..mmap.as_ptr()+len`) with **0 heap allocations** for the payload bytes.
- **Throughput Gain**: `AlignedSlabBufferPool` + zero-copy `write_chunk` reduces CPU instructions-per-byte by `>= 35%` and achieves `>= 1.8 GB/s` loopback QUIC shard transfer throughput on desktop.
- **Mobile Idle Wakeup SLA**: In `BatteryBackgroundEco` mode, total network/timer wakeups over a simulated 1-hour idle period are `<= 65 wakeups` (<= 1.08 wakeups/minute) and 0 background GPU/WASM compute tasks are accepted in `BatteryDeepSaver` mode.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_aligned_slab_buffer_pool_64b_and_16kb_alignment_and_auto_return_on_drop`
  - `test_mmap_to_bytes_slice_preserves_exact_pointer_address_zero_alloc`
  - `test_timer_coalescer_batches_heartbeats_into_single_radio_wakeup_window`
  - `test_power_governor_state_transitions_ac_to_eco_to_deep_saver`
- **`[INT]`**:
  - `test_zero_copy_quic_4mib_shard_stream_zero_payload_heap_copies`
  - `test_battery_deep_saver_gracefully_drains_and_rejects_remote_worker_tasks`
- **`[SIM]`**:
  - 24-hour accelerated mobile battery model simulation comparing uncoalesced timers vs `MobilePowerGovernor` coalesced wakeups, verifying >= 75% reduction in radio active duration.
- **`[HW]`**:
  - Run `criterion` data-plane benchmarks (`benches/data_plane_zero_copy.rs`) on target hardware and archive throughput (GB/s) and latency (ns/op).
- **`[MANUAL]`**:
  - Toggle Power Profile (`Auto Eco`, `Max Performance`, `Battery Saver`) in Flutter Mobile Settings and inspect the "Estimated Battery Impact (%/hr)" and wakeup counter.

## 6. Required Evidence & Artifacts
- `artifacts/stage-08/p3/sp1_zero_copy_pointer_and_criterion_report.json`
- `artifacts/stage-08/p3/sp1_mobile_power_governor_wakeup_audit.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document zero-copy buffer architecture and `MobilePowerGovernor` state table in `docs/OPERATIONS_RUNBOOK.md` (Section 4).
- Mark `S8.P3.SP1` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If any `MmapSliceGuard` can outlive its backing file descriptor, or if `BatteryDeepSaver` allows background tasks to drain a mobile battery below 20%.
- **Completion Gate**: All tests and Criterion benchmarks green before proceeding to `S8.P3.SP2`.
