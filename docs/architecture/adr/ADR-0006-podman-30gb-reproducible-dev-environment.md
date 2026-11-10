# ADR-0006: Rootless Podman Development, Build & Multi-Node Simulation Environment Within a Strict 30 GB Disk Cap

- **Status**: Accepted
- **Date**: 2026-11-10
- **Deciders**: UMCF Core Architecture Team
- **Related Components**: `containers/*`, `scripts/*`, Workspace Build & Test Pipeline

---

## 1. Context and Problem Statement

UMCF targets 7 platforms and integrates Rust workspaces, C/C++ bindings (`llama.cpp`), Flutter SDKs, Svelte/Node toolchains, and multi-node network simulation tests. Unconstrained Rust `target/` directories with full DWARF debug symbols and multiple Docker/Podman container images routinely consume **60 GB to 120 GB** of disk space.

To guarantee that UMCF can be built, tested, and simulated reproducibly on developer workstations and CI runners with constrained storage, we must enforce a **strict, non-negotiable 30 GB maximum total disk footprint** for all Podman images, volumes, build caches, and artifacts combined.

## 2. Decision Drivers

1. **Hard 30 GB Ceiling**: Total Podman storage (`podman system df`) + workspace build caches (`target/`, `.dart_tool/`, `node_modules/`) must never exceed `30.0 GB` (with an automated fail-safe alert at `25.0 GB`).
2. **Rootless Security & Network Namespace Simulation**: Must support simulating multi-node LAN, WAN latency/jitter (`tc netem`), and NAT topologies without requiring host VM hypervisors.
3. **High Build Speed**: Avoid redownloading crates or recompiling unchanged dependencies across test runs.

## 3. Decision Outcome

We standardize all containerized builds, unit/integration tests, and multi-node cluster simulations on **Rootless Podman** governed by the following storage-minimization architecture:

### 3.1 Single Unified Multi-Purpose Builder & Simulator Image
Instead of maintaining 6 separate fat container images (`umcf-rust-builder`, `umcf-android-builder`, `umcf-web-builder`, `umcf-sim-node`, etc.), we use a **two-tier image architecture**:
1. **`localhost/umcf-dev-toolchain:latest` (`<= 4.5 GB`)**: Based on `debian:bookworm-slim`, containing Rust stable (`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `wasm32-unknown-unknown`), `cargo-llvm-cov`, Node 22 Slim, and `iproute2`/`nftables` for network simulation.
2. **`localhost/umcf-node-runtime:latest` (`<= 180 MB`)**: Distroless/Debian-slim runtime image mounting compiled test/daemon binaries from the host workspace for lightweight 4-to-16-node cluster simulations.

### 3.2 Workspace Cargo Profile Optimizations (`.cargo/config.toml` & `Cargo.toml`)
Full DWARF debug info (`debug = 2`) is the #1 cause of 40+ GB Rust `target/` directories. We configure workspace profiles as follows:
```toml
[profile.dev]
opt-level = 1
debug = "line-tables-only"   # Reduces target/ size by ~75% while preserving full stack trace line numbers
split-debuginfo = "unpacked"
incremental = true

[profile.test]
opt-level = 1
debug = "line-tables-only"

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = "symbols"
panic = "abort"
```

### 3.3 Automated Disk Budget Guardrail (`scripts/check-disk-budget.ps1` / `.sh`)
Every build and test script invokes a pre-flight and post-flight disk budget check:
- **Green Zone (`< 22.0 GB`)**: Normal operation.
- **Yellow Zone (`22.0 GB .. 26.0 GB`)**: Automatically runs `cargo sweep --time 1` (or cleans intermediate non-cached test binaries) and `podman image prune -f`.
- **Red Zone (`>= 27.0 GB`)**: Halts build before breaching the 30 GB hard limit and executes mandatory cache compaction (`cargo clean --doc`, `podman system prune -f`).

## 4. Consequences

### Positive
- **Guaranteed `<= 30 GB` Footprint**: Typical peak usage stays between **11 GB and 18 GB** even during full workspace compilation, coverage generation, and 8-node Podman cluster simulation.
- **Fast Multi-Node Simulation**: Spawning 10 `umcf-node-runtime` containers takes `< 2 seconds` and `< 50 MB` of overlay filesystem overhead because all containers bind-mount the compiled host binary read-only.

### Negative & Mitigations
- **Line-Tables-Only Debug Info in Dev Builds**: Local GDB variable inspection requires temporarily passing `--config profile.dev.debug=2` for the specific crate being debugged.
