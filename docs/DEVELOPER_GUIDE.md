# Universal Mobile Compute Fabric (UMCF) — Developer Guide

> [!IMPORTANT]
> **Hard Disk Budget Constraint (ADR-0006)**: All containerized builds, toolchains, Rust `target/` caches, and multi-node simulations MUST remain strictly within the **30.0 GB total disk footprint ceiling**. Always use the provided workspace scripts (`scripts/check-disk-budget.*`) and shared Podman cache mounts.

---

## 1. Repository Structure & Monorepo Layout

```text
g:\ucf\
├── Cargo.toml                         # Rust workspace root (shared dependencies & size-optimized profiles)
├── .cargo/config.toml                 # Compiler flags, line-tables-only debuginfo, target runners
├── crates/
│   ├── umcf-proto/                    # Wire framing (16-byte header + CRC32C), schemas, MessagePack/JSON codecs
│   ├── umcf-core/                     # Ed25519 identity, RBAC tokens, QUIC (quinn) transport, mDNS, ICE/Relay
│   ├── umcf-coordinator/              # Cluster registry, lease_epoch fencing, heartbeats, autonomous election
│   ├── umcf-scheduler/                # Constraint solver, DAG partitioner, bandwidth-aware placement, rebalance
│   ├── umcf-worker/                   # Sandbox executor, 75% mobile RAM guard, thermal/battery monitor
│   ├── umcf-engine-storage/           # BLAKE3 Content-Addressed Storage (CAS), 4 MiB chunk swarming, mmap
│   ├── umcf-engine-ai/                # Distributed llama.cpp / GGML layer-pipeline inference & KV-cache sync
│   ├── umcf-engine-media/             # Cross-platform wgpu (Vulkan/Metal/DX12/WebGPU) & FFmpeg GOP transcode
│   ├── umcf-engine-stream/            # Low-latency screen capture, HW encode, QUIC datagram FEC, input return
│   ├── umcf-ffi/                      # C-ABI & flutter_rust_bridge v2 bindings for mobile/desktop apps
│   └── umcf-cli/                      # Headless CLI daemon (`umcf`) & multi-node chaos simulator (`umcf-sim`)
├── services/
│   ├── control-plane-cf/              # Primary WAN Control Plane (Cloudflare Workers + Durable Objects, TS/Rust)
│   └── control-plane-cloudrun/        # Fallback WAN Control Plane (Stateless Rust Axum container for Cloud Run)
├── apps/
│   ├── mobile-desktop/                # Unified Flutter 3.x client (Android, iOS, Windows, Linux, macOS)
│   └── web-dashboard/                 # Svelte 5 + TypeScript + WebGPU + WASM client
├── containers/
│   ├── Containerfile.toolchain        # Unified <4.5 GB Debian-Slim builder & test runner image
│   ├── Containerfile.node             # Lightweight <180 MB node runtime container for cluster simulation
│   └── compose.sim.yaml               # Podman Compose topology for LAN/WAN/NAT multi-node simulations
├── scripts/                           # Build, test, coverage, and 30 GB disk-budget enforcement scripts
└── docs/
    ├── architecture/                  # System Overview, Component Contracts, Data Flows, Risks, ADRs
    ├── requirements/                  # Functional (FR-001..080), Non-Functional (NFR-001..040), Traceability
    ├── USER_GUIDE.md
    └── DEVELOPER_GUIDE.md
```

---

## 2. Development Environment Setup

### 2.1 Prerequisites
1. **Podman 4.x / 5.x** (Rootless mode configured)
2. **Rust Toolchain (`1.82+` stable)**:
   ```bash
   rustup toolchain install stable --profile minimal --component rustfmt clippy llvm-tools-preview
   rustup target add wasm32-unknown-unknown aarch64-unknown-linux-gnu
   ```
3. **Node.js `22 LTS`** (for `services/control-plane-cf` and `apps/web-dashboard`)
4. **Flutter `3.24+`** (optional on host if building native UI locally; containerized verification supported)

### 2.2 Managing the 30 GB Podman Disk Budget
Before and after running builds, verify your workspace and container storage footprint:

```powershell
# On Windows PowerShell:
pwsh ./scripts/check-disk-budget.ps1

# On Linux / macOS / inside Podman container:
./scripts/check-disk-budget.sh
```

#### How We Stay Under 30 GB:
- **Line-Tables-Only Debug Symbols**: `.cargo/config.toml` sets `debug = "line-tables-only"`, reducing `target/` size from ~35 GB to **~4.5 GB** while preserving exact file/line backtraces.
- **Bind-Mounted Target Cache**: Containerized builds mount the host's `./target` and `~/.cargo/registry` rather than copying gigabytes of intermediate `.rlib` artifacts into container layers.
- **Two-Image Cap**: Never create ad-hoc fat container images. Always reuse `localhost/umcf-dev-toolchain:latest` and `localhost/umcf-node-runtime:latest`.

---

## 3. Building & Testing the Workspace

### 3.1 Building Rust Crates & CLI Daemon
```bash
# Check compilation & strict Clippy lints across all workspace crates
cargo clippy --workspace --all-targets -- -D warnings

# Build the CLI daemon & simulator in optimized dev mode
cargo build --workspace

# Build stripped release binaries (< 25 MiB)
cargo build --release -p umcf-cli -p umcf-ffi
```

### 3.2 Running the Automated Test Suite (`>= 95%` Coverage Target)
UMCF classifies tests into five categories (`Unit`, `Integration`, `Simulation`, `Hardware-Required`, `Manually Verified`):

```bash
# 1. Run all Unit and Integration tests across the workspace
cargo test --workspace --all-targets

# 2. Run Property-Based Fuzz Tests (100,000 iterations on wire parsers & scheduler)
cargo test -p umcf-proto -p umcf-scheduler --features proptest-suite

# 3. Generate LLVM Line & Branch Coverage Report (Enforces >= 95% threshold)
cargo llvm-cov --workspace --lcov --output-path lcov.info
cargo llvm-cov report --fail-under-lines 95
```

### 3.3 Running Multi-Node Cluster Simulations in Podman
You can simulate an entire heterogeneous cluster (e.g., 1 Hybrid Master, 2 LAN Workers, 1 WAN Symmetric-NAT Worker, plus the Fallback Control Plane) on a single machine using Rootless Podman:

```bash
# Build the lightweight node runtime container (bind-mounts compiled ./target/debug/umcf)
podman build -t localhost/umcf-node-runtime:latest -f containers/Containerfile.node .

# Launch a 4-node simulated cluster with artificial latency/loss (tc netem)
podman compose -f containers/compose.sim.yaml up --abort-on-container-exit

# Or run the built-in deterministic multi-node simulation harness directly:
cargo test -p umcf-cli --test cluster_simulation -- --nocapture
```

### 3.4 Building & Testing Control Plane Services

```bash
# Primary Control Plane (Cloudflare Workers + Durable Objects)
cd services/control-plane-cf
npm ci
npm test                 # Runs Vitest + Miniflare Durable Object integration tests

# Fallback Control Plane (Google Cloud Run Rust Axum Service)
cargo test -p umcf-control-plane-cloudrun
```

### 3.5 Building & Testing Client Applications

```bash
# Web Dashboard (Svelte 5 + TypeScript + WASM)
cd apps/web-dashboard
npm ci
npm run check            # svelte-check + TypeScript strict verification
npm test                 # Vitest unit & component accessibility tests
npm run build

# Mobile & Desktop App (Flutter + flutter_rust_bridge v2)
cd apps/mobile-desktop
flutter pub get
flutter analyze --no-fatal-infos
flutter test
```

---

## 4. Architecture Guide for Adding New Capabilities

### 4.1 Adding a New Wire Message Type
1. Open [`crates/umcf-proto/src/lib.rs`](file:///g:/ucf/crates/umcf-proto/src/lib.rs) and register the discriminant in `MessageType`.
2. Define the strongly-typed payload struct deriving `Serialize, Deserialize, Debug, Clone, PartialEq`.
3. Add a round-trip serialization + CRC32C corruption + fuzz test in `crates/umcf-proto/tests/`.
4. Verify that Control Plane messages remain strictly `<= 64 KiB` (`MAX_CONTROL_FRAME_BYTES`).

### 4.2 Adding or Extending an Execution Engine
All execution engines (`umcf-engine-ai`, `umcf-engine-media`, `umcf-engine-stream`, `umcf-engine-storage`) implement the `EngineExecutor` trait consumed by `umcf-worker`:
1. **Never bypass `umcf-worker` resource guards**: Before allocating large buffers or memory-mapping CAS files, invoke `ResourceGuard::reserve_memory(bytes)` so the 75% mobile RAM watermark is enforced.
2. **Respect cancellation & thermal preemption tokens**: Every long-running engine loop (e.g., token generation step or GOP transcode loop) must select on `tokio_util::sync::CancellationToken` triggered when `ThermalState::Critical` is reached.
3. **Chunk GPU dispatches**: Ensure individual GPU command buffer submissions complete in `<= 500ms` (`NFR-022`) to prevent OS GPU watchdog resets (TDR).

---

## 5. Debugging & Profiling Tips

- **Inspecting Live Wire Frames**: Set `RUST_LOG=umcf_proto=trace,umcf_core::transport=debug` to log envelope headers, correlation IDs, `lease_epoch` numbers, and QUIC stream priorities (secrets and payloads are automatically redacted).
- **Simulating Thermal or Battery Drops Locally**: Use the CLI fault-injection flags on any worker node:
  ```bash
  umcf debug inject-telemetry --node local --battery-temp-c 43.5 --battery-pct 18 --ac-power false
  ```
  Watch the node immediately transition to `NodeRole::Draining`, trigger `IncrementalRebalance` on the Coordinator, and hand off its active pipeline layers.
