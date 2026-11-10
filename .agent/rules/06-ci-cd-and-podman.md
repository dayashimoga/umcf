---
trigger: always_on
description: Podman reproducible build environment, strict 30 GB storage budget, artifact packaging, SBOMs, and release honesty standards for UMCF.
---

# 06 - CI/CD, Release Packaging & Podman Environment (UMCF)

All build pipelines, container images, developer environments, and release artifacts for UMCF must obey strict reproducibility, storage quota, and packaging integrity rules.

---

## 1. Podman-First Reproducible Environment

1. **Rootless Podman Standard**:
   - All containerized builds, local control-plane clusters, multi-node network simulations, and cross-compilation toolchains must use **Podman** (`podman`, `podman-compose` / `podman kube play`) with rootless-compatible `Containerfile` definitions.
2. **Pinned Base Images & Toolchains**:
   - Pin all base container images by version tag and digest where applicable, and pin exact Rust (`rust-toolchain.toml`), Flutter, Node/Bun, and Android NDK/SDK versions for deterministic builds.

---

## 2. Strict 30 GB Podman Storage Quota Enforcement

1. **Hard 30 GB Ceiling**:
   - Total Podman storage utilization (images, layers, build cache, volumes, and running container writable layers combined) on the host machine must **never exceed 30 GB** (`30 * 1024^3` bytes).
2. **Mandatory Pre-Build & Post-Build Quota Guards**:
   - Every Podman build script and CI workflow must run a storage inspection check before and after image builds:
     - Use multi-stage `Containerfile` builds that discard intermediate build artifacts (`target/debug`, incremental Rust objects, downloaded SDK archives) in the same layer or copy only final stripped binaries into a minimal runtime image (`debian:bookworm-slim` / `distroless`).
     - Mount host-shared or bounded cargo/build caches with automatic pruning (`cargo sweep` / `podman image prune -f` / `podman build --layers=false` when appropriate) whenever storage approaches **24 GB (80% warning threshold)**, and abort with a cleanup runbook before crossing **30 GB**.

---

## 3. Release Artifact Packaging Rules

1. **Independent Platform Artifacts (Zero Nested Zips)**:
   - Every target platform and deliverable must be packaged as an independent, directly usable artifact in the release directory.
   - **Never wrap zip/tar archives inside another zip archive** (no `release-all.zip` containing `windows.zip`, `linux.tar.gz`, etc.).
2. **Standard Deliverable Matrix**:
   - **Android**: Standalone installable `.apk` and `.aab` packages.
   - **Windows**: Standalone portable `.zip` or installer `.msi`/`.exe` containing the Flutter desktop client, Rust core DLLs, and `umcf-cli.exe`.
   - **Linux**: Standalone `.tar.gz` / `.AppImage` / `.deb` packages for x86_64 and aarch64.
   - **macOS**: Standalone `.dmg` or `.tar.gz` app bundle.
   - **iOS**: Signed `.ipa` when Apple Developer signing certificates/provisioning profiles are present; otherwise an **explicitly labeled unsigned `.xcarchive` / payload bundle** (see Section 4).
   - **Web Dashboard & Control Plane**: Self-contained Cloudflare Worker bundle + static assets archive and container image manifest.

---

## 4. iOS & Platform Signing Honesty

1. **Never Fake Code Signatures**:
   - When building on Windows/Linux hosts or macOS environments without Apple Developer signing credentials, **never fabricate dummy `.ipa` signatures** or claim an unsigned bundle is directly installable on stock non-jailbroken iOS devices without sideloading/signing.
2. **Explicit Documentation & Unsigned Archive Output**:
   - When Apple signing credentials or macOS Xcode toolchains are absent, produce the verified cross-platform Rust static libraries / Flutter iOS source & Xcode project configuration (or unsigned `.xcarchive` on macOS) and explicitly record `signing_status: "UNSIGNED_REQUIRES_APPLE_DEVELOPER_CERT"` in the release manifest with step-by-step Xcode signing instructions.

---

## 5. Checksums, SBOMs, and Release Manifests

1. **Cryptographic Checksums (`SHA256SUMS.txt`)**:
   - Every release build must generate a `SHA256SUMS.txt` file containing the exact SHA-256 hex digest and filename of every produced artifact.
2. **Software Bill of Materials (SBOM)**:
   - Generate machine-readable CycloneDX or SPDX SBOMs (`sbom.cdx.json`) covering Cargo (`Cargo.lock`), Flutter/Dart (`pubspec.lock`), and npm/pnpm (`package-lock.json` / `pnpm-lock.yaml`) dependencies.
3. **Verifiable Release Manifest (`release-manifest.json`)**:
   - Accompany every release with a structured JSON manifest recording:
     - Semantic version and Git commit SHA
     - Build timestamp and host/container environment details
     - Per-artifact path, byte size, SHA-256 hash, target triple, and signing status
     - Test suite summary (total tests, pass rate, coverage percentage)
