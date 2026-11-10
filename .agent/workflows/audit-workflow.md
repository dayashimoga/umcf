---
description: Step-by-step repository, security, dependency, and requirement compliance audit workflow for UMCF.
---

# UMCF Comprehensive Repository & Compliance Audit Workflow

Execute this workflow prior to starting a phase, before cutting a release, or whenever conducting a health, security, and requirement compliance audit of the UMCF repository.

---

## Step 1: Workspace Structure & Governance Verification

1. Verify that all required workspace directories (`crates/`, `apps/flutter_client/`, `apps/web_dashboard/`, `services/`, `podman/`, `docs/`, `.agent/`) match the canonical UMCF layout.
2. Confirm that `TODO.md` and `CHANGELOG.md` exist at the repository root and have preserved their append-only history without deleted entries.

---

## Step 2: Zero-Placeholder & Dead-Code Scan

Scan all source files across Rust, Dart/Flutter, and Svelte/TypeScript for prohibited patterns:

1. **Rust Placeholder & Panic Scan**:
   - Search `crates/` (excluding test modules) for `todo!`, `unimplemented!`, `.unwrap()`, `.expect(`, `dbg!`, and `unsafe` outside audited `*-sys` crates.
2. **Flutter UI Completeness Scan**:
   - Search `apps/flutter_client/lib/` for empty callbacks (`onPressed: () {}`, `onChanged: (_) {}`), hardcoded mock strings in production views, or `TODO` markers.
3. **Svelte 5 & TypeScript Scan**:
   - Search `apps/web_dashboard/src/` for legacy Svelte 3/4 reactivity (`export let`, `$:`, `createEventDispatcher`), `: any`, `@ts-ignore`, or unimplemented button handlers.

---

## Step 3: Dependency & Supply-Chain Security Audit

1. **Rust Dependencies**:
   - Run `cargo audit` (or `cargo deny check advisories bans licenses sources`) to verify zero known CVEs, zero unmaintained crates in critical paths, and license compliance.
2. **Flutter / Dart Dependencies**:
   - Run `flutter pub outdated` and verify all packages in `pubspec.lock` resolve cleanly without discontinued packages.
3. **Web / Edge Dependencies**:
   - Run `npm audit` / `pnpm audit` in `apps/web_dashboard` and `services/control_plane_worker`.
4. **Secrets & Credential Leak Scan**:
   - Scan tracked and untracked files for accidental private keys (`PRIVATE KEY`), API tokens, `.env` files, or Android keystore `.jks` credentials.

---

## Step 4: Architectural & Security Invariant Audit

1. **Control Plane vs Data Plane Separation**:
   - Verify that `services/control_plane_worker` and `services/control_plane_fallback` handle only discovery, enrollment, NAT signaling, and topology metadata—never tensor activations, model weights, or streaming frames.
2. **Mutual Authentication & Capability Enforcement**:
   - Verify that every QUIC stream handler in `umcf-p2p` / `umcf-node` verifies peer identity (`NodeId`) and validates a signed `CapabilityToken` before executing any workload.
3. **Hardware Capability Honesty**:
   - Verify that `umcf-hw-probe` and `umcf-scheduler` never fabricate GPU/NPU acceleration status or claim unified cross-network memory pooling.

---

## Step 5: Podman Storage Quota Check

1. Inspect Podman disk usage (`podman system df`) and verify total storage is well below the **30 GB hard limit** (target < 24 GB).
2. Prune dangling build layers or stale caches if usage exceeds 20 GB.

---

## Step 6: Generate Structured Audit Report

Compile findings into a timestamped report (`docs/audits/audit-<timestamp>.md` or phase verification artifact) containing:
- Requirement compliance matrix (`PASS` / `FAIL` with file + line references)
- Static analysis & lint results (`cargo clippy`, `flutter analyze`, `svelte-check`)
- Security & dependency status
- Remediation action items appended to `TODO.md`
