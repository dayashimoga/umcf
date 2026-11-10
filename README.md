# Universal Mobile Compute Fabric (UMCF)

A production-grade, decentralized, cross-platform compute fabric where **Android, iOS, Windows, Linux, macOS, and Server nodes** dynamically operate as **Masters (Coordinators)**, **Workers**, or **Hybrid nodes** over LAN and WAN.

UMCF powers **distributed AI inference (`llama.cpp`)**, **private local AI**, **heterogeneous CPU/GPU/NPU compute**, **distributed rendering & media transcoding**, **ultra-low-latency remote desktop & game streaming**, **content-addressed distributed storage**, and **cryptographically metered compute marketplace workloads**.

---

## Core Architectural Highlights

- **Strict Control-Plane vs. Data-Plane Separation**:
  - **Control Plane**: Local Autonomous Coordinator (offline/LAN-first), Cloudflare Workers + Durable Objects (global edge primary), and Google Cloud Run (container fallback).
  - **Data Plane**: Direct peer-to-peer multiplexed QUIC (mTLS 1.3) transport with mDNS LAN discovery, STUN/ICE NAT hole-punching, and encrypted relay fallback. Bulk tensors, media segments, and frames never traverse the cloud control plane.
- **Explicit Hardware Heterogeneity (Zero Fake Pooling)**:
  - Heterogeneous GPUs, NPUs, CPUs, and OS memory spaces are never falsely presented as uniform shared memory. UMCF negotiates explicit hardware capability profiles and partitions workloads via pipeline/layer parallelism (`llama.cpp`), tile/frame rendering, and segment media processing.
- **Podman-First Reproducible Environment (Strict 30 GB Cap)**:
  - All toolchains (Rust, Flutter, Svelte 5 / Node, Wrangler) can run inside reproducible Podman containers with an enforced `<= 30 GB` storage allocation (`scripts/enforce-podman-quota.ps1` / `.sh`).

---

## Repository Structure

| Path | Component | Description |
|---|---|---|
| [`ENGINEERING_CONTRACT.md`](./ENGINEERING_CONTRACT.md) | Governance | Mandatory coding standards, quality gates (`>=95%` coverage, `100%` pass rate), security invariants & DoD |
| [`AGENTS.md`](./AGENTS.md) | Antigravity | Global agent instructions, directory map & customization index |
| [`.agent/`](./.agent/) | Customizations | `.agent/rules/`, `.agent/skills/`, and `.agent/workflows/` |
| [`crates/umcf-proto`](./crates/umcf-proto) | Rust Crate | Versioned wire protocol envelopes, capability negotiation & error codes |
| [`crates/umcf-core`](./crates/umcf-core) | Rust Crate | Node identity, dynamic `Master`/`Worker`/`Hybrid` role engine, resource descriptors & consent policies |
| [`crates/umcf-coordinator`](./crates/umcf-coordinator) | Rust Crate | Cluster membership registry, node lease management & job admission |
| [`crates/umcf-worker`](./crates/umcf-worker) | Rust Crate | Worker runtime guard, hardware capability probe & sandboxed task executor |
| [`crates/umcf-scheduler`](./crates/umcf-scheduler) | Rust Crate | Heterogeneous capability-aware DAG scheduler & partition solver |
| [`crates/umcf-cli`](./crates/umcf-cli) | Rust Binary | `umcf` developer & operator CLI (`doctor`, `inspect-env`, `verify-protocol`, `plan-inference`) |
| [`crates/umcf-daemon`](./crates/umcf-daemon) | Rust Binary | `umcfd` node daemon (`self-check`, `print-default-config`, `run-oneshot`) |
| [`apps/mobile-desktop`](./apps/mobile-desktop) | Flutter App | Cross-platform Android, iOS, Windows, Linux & macOS node controller and dashboard |
| [`apps/web-dashboard`](./apps/web-dashboard) | Svelte 5 App | Real-time cluster topology, workload scheduler & telemetry web console |
| [`services/control-plane-cf`](./services/control-plane-cf) | Cloudflare | Cloudflare Workers + Durable Objects global signaling & registry control plane |
| [`services/control-plane-cloudrun`](./services/control-plane-cloudrun) | Cloud Run | Stateless Rust/container control-plane fallback service |
| [`infra/podman`](./infra/podman) | DevOps | Reproducible Podman container environment & 30 GB quota configuration |
| [`docs/`](./docs/) | Documentation | Architecture, ADRs, Traceable Requirements, 8-Stage Master Roadmap & 104 Executable Prompts |

---

## Quick Start

### 1. Verify Podman 30 GB Storage Quota & Environment
```powershell
# Windows (PowerShell)
pwsh ./scripts/enforce-podman-quota.ps1
```
```bash
# Linux / macOS
bash ./scripts/enforce-podman-quota.sh
```

### 2. Build & Test Rust Core Workspace
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p umcf-cli -- doctor --json
```

### 3. Build & Test Web Dashboard & Cloudflare Control Plane
```bash
cd apps/web-dashboard && npm install && npm test && npm run build
cd ../../services/control-plane-cf && npm install && npm test
```

### 4. Documentation Index
- **Architecture**: [`docs/architecture/SYSTEM_OVERVIEW.md`](./docs/architecture/SYSTEM_OVERVIEW.md) | [`COMPONENT_CONTRACTS.md`](./docs/architecture/COMPONENT_CONTRACTS.md) | [`DATA_FLOWS.md`](./docs/architecture/DATA_FLOWS.md)
- **Requirements**: [`docs/requirements/FUNCTIONAL_REQUIREMENTS.md`](./docs/requirements/FUNCTIONAL_REQUIREMENTS.md) | [`NON_FUNCTIONAL_REQUIREMENTS.md`](./docs/requirements/NON_FUNCTIONAL_REQUIREMENTS.md) | [`TRACEABILITY_MATRIX.md`](./docs/requirements/TRACEABILITY_MATRIX.md)
- **Implementation Roadmap**: [`docs/implementation/MASTER_ROADMAP.md`](./docs/implementation/MASTER_ROADMAP.md)
- **Guides**: [`docs/DEVELOPER_GUIDE.md`](./docs/DEVELOPER_GUIDE.md) | [`docs/USER_GUIDE.md`](./docs/USER_GUIDE.md) | [`SECURITY.md`](./SECURITY.md)

