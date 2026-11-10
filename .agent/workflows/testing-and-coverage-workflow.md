---
description: Workflow for running unit, integration, simulation, and hardware tests and verifying >=95% coverage and 100% pass rate in UMCF.
---

# UMCF Testing & Coverage Verification Workflow

Follow this workflow to execute all test tiers across the UMCF workspace and enforce the mandatory **100% pass rate** and **>= 95% code coverage** gates.

---

## Step 1: Rust Workspace Unit, Integration & Simulation Tests

1. **Execute Fast Test Suite**:
   ```powershell
   cargo test --workspace --all-targets --all-features
   ```
   - Verify **0 failed**, **0 ignored** (except explicitly documented `[Hardware-Required]` tests when running on a host without that specific accelerator).
2. **Execute Multi-Node Mesh Simulation Suite (`[Simulation]`)**:
   - Run the deterministic multi-node simulation tests validating:
     - Heterogeneous pipeline partitioning across simulated phone, tablet, laptop, and desktop nodes
     - Mid-inference node disconnect and live stage migration under thermal/battery pressure
     - NAT traversal waterfall (LAN mDNS -> STUN hole punching -> encrypted relay fallback)

---

## Step 2: Rust Code Coverage Measurement (`>= 95%`)

1. **Run `cargo llvm-cov`**:
   ```powershell
   cargo llvm-cov --workspace --all-features --summary-only
   ```
2. **Inspect Per-Crate Coverage**:
   - Generate detailed line and region coverage reports:
     ```powershell
     cargo llvm-cov --workspace --all-features --json --output-path target/llvm-cov-summary.json
     ```
   - Verify that aggregate workspace coverage and each core domain crate (`umcf-proto`, `umcf-crypto`, `umcf-hw-probe`, `umcf-resource-governor`, `umcf-p2p`, `umcf-scheduler`, `umcf-llama-engine`, `umcf-stream-engine`, `umcf-node`) achieve **>= 95% coverage**.
   - If any module falls below 95%, inspect uncovered branches and add targeted tests for error paths, boundary values, and state transitions.

---

## Step 3: Flutter Client Tests & Coverage (`>= 95%`)

1. **Static Analysis**:
   ```powershell
   cd apps/flutter_client
   flutter analyze --fatal-infos --fatal-warnings
   ```
2. **Widget, Controller & Integration Tests with Coverage**:
   ```powershell
   flutter test --coverage
   ```
   - Verify 100% pass rate across all Riverpod notifiers, adaptive layout breakpoints, QR enrollment flows, resource governor sliders, and streaming controls.
   - Parse `coverage/lcov.info` to confirm **>= 95% line coverage**.

---

## Step 4: Svelte 5 Web Dashboard & Control Plane Worker Tests (`>= 95%`)

1. **Typecheck & Lint**:
   ```powershell
   cd apps/web_dashboard
   npm run check
   ```
2. **Unit & Component Coverage (`Vitest`)**:
   ```powershell
   npm run test:coverage
   ```
   - Verify 100% pass rate and >= 95% line/branch coverage across `.svelte.ts` runes stores and `.svelte` components.
3. **Control Plane Worker & Fallback Tests**:
   - Run the Cloudflare Worker / Durable Object and fallback control plane test suites verifying enrollment token TTL enforcement, peer signaling, and WebSocket broadcast resilience.
4. **End-to-End Browser Verification (`Playwright`)**:
   ```powershell
   npm run test:e2e
   ```

---

## Step 5: Record Classification & Coverage Evidence

Summarize all test results in the verification artifact and `CHANGELOG.md` broken down by the five-tier taxonomy (`[Unit]`, `[Integration]`, `[Simulation]`, `[Hardware-Required]`, `[Manually-Verified]`), recording exact passed/total counts and coverage percentages.
