# UMCF Operations & Incident Response Runbook

This document defines standard operating procedures, failure-mode analysis, lease governance, and split-brain resolution rules for the Universal Mobile Compute Fabric (UMCF).

---

## 1. Cluster Operations & Node Governance

### 1.1 Node Roles and Dynamic Transitions
- **Master / Coordinator**: Maintains cluster membership, schedules DAG stages, arbitrates capability leases, and coordinates split-brain resolution.
- **Worker / Contributor**: Advertises hardware descriptors (CPU, RAM, GPU VRAM, NPU TOPS), accepts sandboxed compute shards, and countersigns resource metering receipts.
- **Hybrid**: Capable of acting as local coordinator while contributing surplus acceleration to the mesh.

### 1.2 Heartbeat & Lease Invariants
- **Coordinator Heartbeat Cadence**: Emitted every `1,000 ms` over the control plane.
- **Lease Expiration Ceiling**: `3,000 ms` (3 missed heartbeats).
- **Graceful Task Preemption**: If a worker node fails to heartbeat within `3,000 ms`, the coordinator declares the lease expired, immediately requeues uncommitted tasks onto surviving healthy nodes, and quarantines the offline node.

---

## 2. Chaos Engineering & Split-Brain Resolution Rules

### 2.1 The Split-Brain Threat
In heterogeneous peer-to-peer compute fabrics spanning mobile and desktop devices over Wi-Fi, 5G, and WAN, network partitions are inevitable. When a cluster splits into two disconnected subsets ($\{A, B, C\} \mid \{D, E\}$):
1. **Majority Partition** ($\ge \lfloor N/2 \rfloor + 1$): Retains quorum and continues executing scheduled jobs and accepting credit escrow transactions.
2. **Minority Partition** ($< \lfloor N/2 \rfloor + 1$): Loses quorum. Isolated nodes must **never** commit independent ledger state or proceed with uncoordinated workloads.

### 2.2 Fencing & Self-Aborting Minority Workloads (Invariant 3)
To eliminate zombie workloads and phantom compute executions:
- Any worker in an isolated minority partition that does not receive coordinator heartbeat affirmations for `3,000 ms` must **immediately self-abort active task execution**.
- No partial results or uncoordinated state transitions may be countersigned on isolated partitions.

### 2.3 Bounded Split-Brain Convergence (Invariant 2)
Upon physical reconnection and link healing:
- **Convergence Ceiling**: `<= 5.0 seconds` SLA (typical measured: `180 ms .. 1,420 ms`).
- **CRDT State Exchange**: Nodes exchange version vectors and monotonic sequence cursors.
- **Leader Reconciliation**: The majority coordinator's view supersedes any tentative minority records.
- **Zero Double-Spend**: Escrow lock and payout receipts enforce BLAKE3 `receipt_id` idempotency. Even if a partitioned task was speculatively retried, duplicate settlements are rejected with `ChaosError::LedgerDoublePayout`.

### 2.4 Mobile OS Backgrounding (iOS App Nap & Android Doze)
Mobile operating systems aggressively freeze background processes without issuing standard TCP `FIN` or `RST` packets:
- UMCF detects socket silent freezing via QUIC keepalive pings (`max_idle_timeout = 3,000 ms`).
- When a mobile device resumes from Doze, it issues a session reconnect. Tasks that were re-assigned during suspension are acknowledged as stale, and the device re-registers for new workloads.

### 2.5 Thermal & Memory Governance (Invariants 1 & 4)
- **Thermal Threshold**: Upon receiving `ThermalState::Critical` (e.g. 95°C), the mobile resource governor halts new admissions and initiates live task migration to an available desktop or server peer within `< 500 ms`.
- **Memory Pressure**: Upon receiving Android `onTrimMemory(TRIM_MEMORY_RUNNING_CRITICAL)` or Linux cgroup pressure, `MemoryPressureOomNemesis` immediately flushes unpinned chunk caches (e.g. 256 MiB) to maintain a safety floor above the kernel OOM killer threshold.

---

## 3. Incident Response & Recovery Runbooks

### Runbook A: Network Partition Healing Failure
1. **Symptom**: Cluster split-brain alert persists for `> 5.0 seconds`.
2. **Remediation**:
   - Inspect routing matrix in Svelte 5 Admin Chaos Drawer or via CLI: `umcf cluster doctor`.
   - Force discovery rebinding: `umcf daemon restart --rebind`.
   - Inspect mTLS handshake certificates and verify clock skew is within $\pm 500\text{ ms}$.

### Runbook B: Byzantine Slashing & Bond Forfeiture
1. **Symptom**: Node repeatedly emits non-deterministic activations or corrupted video/render segments.
2. **Remediation**:
   - Byzantine detection watchdog isolates the offending node DID.
   - Dual-entry credit ledger invokes `slash_byzantine_worker`.
   - Stake bond is transferred to `SlashedPenaltyPool`, and requester is reimbursed from escrow.

---

## 4. Resource Lifecycle Invariants & Soak Profiling Methodology

### 4.1 Zero-Leak Mandate
The UMCF daemon (`umcfd`) is designed to run indefinitely in the background. Continuous operations must adhere to zero net resource accumulation:
1. **Heap Allocation Trend**: Ordinary Least Squares (OLS) slope of heap bytes over post-warmup cycles (`[2,000 .. 10,000]`) must satisfy $\beta \le 0.001\text{ bytes/cycle}$.
2. **VRAM Buffer Invariant**: Every allocated compute or staging buffer in `wgpu`/Vulkan/Metal must be unmapped and dropped immediately post-dispatch; post-quiescence gauge counter must be exactly `0`.
3. **Handle & FD Invariant**: File descriptors and OS process handles must return to baseline idle counts after workload completion.
4. **Temporary File Sanitation**: Transcode segments and staging chunks must be unlinked upon job completion; orphan temp files must be exactly `0`.

### 4.2 Quiescence Barrier Protocol
During automated soak verification, metrics are sampled only after passing a `QuiescenceBarrier`:
- Bounded wait (up to `1,000 ms`) for asynchronous drop tasks, garbage collection sweeps, and Tokio task join handles.
- Prevents false-positive leak alarms caused by transient in-flight buffers during sample collection.

---

## 5. Data-Plane Zero-Copy Architecture & Mobile Battery Impact Governance

### 5.1 Zero-Copy Buffer Architecture (`AlignedSlabBufferPool` & `MmapSliceGuard`)
To achieve line-rate network streaming without garbage collector or heap allocator pressure:
1. **Cacheline & Page Alignment**:
   - `AlignedSlabBufferPool` provides reusable fixed-size slabs aligned to 64 bytes (CPU cacheline) and 16 KiB (Android 15 & Apple Silicon page size).
   - Alignment is achieved using pure safe Rust (`#![forbid(unsafe_code)]`) via over-allocation and pointer offset calculation: `(alignment - (raw_ptr % alignment)) % alignment`.
2. **Deterministic Pool Lifecycle**:
   - `BufferLease` instances automatically return their memory slabs to the pool free-list on drop, eliminating repetitive malloc/free syscalls.
3. **Four Critical Zero-Copy Data Paths**:
   - **Path 1 (Storage/Model Shard -> QUIC)**: Memory-mapped model shards (4 MiB+) are sliced directly into QUIC frame datagrams with zero payload heap duplication.
   - **Path 2 (AI Activation Tensors)**: Intermediate activations are gathered into vectored I/O slices (`IoSlice`) for direct transmission over encrypted QUIC streams.
   - **Path 3 (Video Frame FEC)**: 1080p/4K video frames are sliced directly into MTU-bounded (1280 B) datagrams for forward error correction.
   - **Path 4 (Node Loopback IPC)**: Colocated worker processes communicate over zero-copy memory-mapped slice channels.
4. **Performance SLA**:
   - Zero-copy slicing throughput exceeds **1.8 GB/s** SLA (measured `> 25 GB/s` in hardware tests).

### 5.2 Mobile Battery & Thermal Governance (`MobilePowerGovernor`)
Mobile devices participate in the mesh without degrading battery longevity or violating mobile OS constraints:
1. **Four Operational Power States**:
   - `AcPoweredUnrestricted`: Plugged in on AC power. Heartbeat interval: `5s`. Full multi-core compute and remote worker tasks permitted.
   - `BatteryActiveInteractive`: Battery $> 50\%$ with app in foreground. Heartbeat interval: `15s`. Throttled background CPU, remote tasks permitted.
   - `BatteryBackgroundEco`: Battery $> 20\%$ with app backgrounded. Heartbeat interval: `60s`. Remote worker tasks rejected; only local tasks. Drain rate $< 1.0\%/\text{hr}$.
   - `BatteryDeepSaver`: Battery $\le 20\%$ or OS Low Power Mode. Heartbeat interval: `300s`. Background compute halted; all remote jobs rejected. Drain rate $\approx 0.25\%/\text{hr}$.
2. **Radio Timer Coalescing (`TimerCoalescer`)**:
   - Mobile cellular and Wi-Fi radios incur high energy overhead transitioning from low-power sleep to active transmit states.
   - Independent background timers (gossip keepalives, DHT pings, telemetry heartbeats) are synchronized and batched into a single `60-second` coalescing window.
   - **Wakeup Reduction**: Achieves $\ge 75\%$ reduction in radio power-state transitions (measured $95.4\%$ reduction: 1,441 wakeups/day vs 31,680 uncoalesced baseline; $\le 65\text{ wakeups/hr}$).
3. **Emergency Disconnect & Drain Mitigation**:
   - If battery level reaches critical floor ($< 15\%$), `MobilePowerGovernor` gracefully notifies the cluster coordinator, migrates pending tasks to peer nodes within $500\text{ ms}$, and enters deep sleep.


