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

