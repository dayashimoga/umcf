---
name: umcf-devops-podman
description: >-
  Guides Podman containerization, strict 30 GB storage quota enforcement, multi-platform release builds, SBOM generation, SHA-256 checksums, and iOS signing honesty for UMCF. Use when writing Containerfiles, build scripts, CI workflows, or packaging releases.
---

# UMCF DevOps, Podman Quota & Multi-Platform Packaging Skill

Use this skill when creating or running `Containerfile` builds, managing Podman containers/volumes, enforcing the 30 GB disk quota, or packaging multi-platform UMCF release artifacts.

---

## 1. Strict 30 GB Podman Storage Quota Discipline

The host environment enforces a **hard 30 GB limit** on all Podman storage combined.

### Quota Inspection & Enforcement Pattern
Before and after every `podman build` or `podman-compose up`:
1. Query current Podman disk consumption:
   ```powershell
   podman system df --format json
   ```
2. Enforce thresholds:
   - **Green (< 20 GB)**: Proceed normally.
   - **Yellow (20 GB – 24 GB)**: Automatically prune dangling images, stopped containers, and build cache (`podman system prune -f --volumes=false` and `podman builder prune -f`).
   - **Red (> 24 GB / approaching 30 GB hard cap)**: Halt new image layer creation immediately, prune unused intermediate images, clean Rust incremental `target/` artifacts inside build volumes, and verify reclaimed space before continuing.

### Lean `Containerfile` Best Practices
- Use multi-stage builds (`FROM rust:1.85-slim-bookworm AS builder` -> `FROM debian:bookworm-slim AS runtime`).
- Combine `apt-get update && apt-get install ... && rm -rf /var/lib/apt/lists/*` in a single `RUN` layer.
- Build release binaries with `CARGO_PROFILE_RELEASE_STRIP="symbols"` and `CARGO_INCREMENTAL=0` inside containers to minimize `target/` bloat.

---

## 2. Multi-Platform Release Packaging Standards

All release artifacts are emitted directly into `dist/v<version>/` as **independent, flat platform deliverables** (never nested archives):

| Platform | Artifact Filename Pattern | Format & Contents |
| :--- | :--- | :--- |
| **Windows (x86_64)** | `umcf-v<ver>-windows-x86_64.zip` | Portable bundle containing Flutter desktop executable, `umcf_ffi.dll`, `umcf-cli.exe`, and README |
| **Linux (x86_64)** | `umcf-v<ver>-linux-x86_64.tar.gz` | Stripped Flutter Linux bundle + `libumcf_ffi.so` + `umcf-cli` |
| **Linux (ARM64)** | `umcf-v<ver>-linux-aarch64.tar.gz` | Stripped ARM64 daemon (`umcf-cli` / `umcf-node`) + library |
| **Android** | `umcf-v<ver>-android-universal.apk` | Signed release APK (plus `.aab` bundle) with ARM64/x86_64 JNI libs |
| **macOS** | `umcf-v<ver>-macos-universal.tar.gz` | macOS `.app` bundle + `umcf-cli` (or cross-compiled core + source bundle if built on non-macOS host) |
| **iOS** | `umcf-v<ver>-ios-unsigned-archive.tar.gz` | Explicitly labeled unsigned archive / Xcode bundle when Apple Developer signing certs are absent |
| **Web / Control Plane** | `umcf-v<ver>-web-control-plane.tar.gz` | Built Svelte 5 static assets + Cloudflare Worker bundle |

---

## 3. Release Integrity Artifacts

Every release directory `dist/v<version>/` must include:
1. `SHA256SUMS.txt`: Standard `sha256sum` formatted digests of every file in `dist/v<version>/`.
2. `sbom.cdx.json`: CycloneDX SBOM listing all Rust, Dart/Flutter, and Node/Svelte dependencies with exact versions and licenses.
3. `release-manifest.json`: Machine-verifiable JSON summary containing commit hash, build timestamp, artifact metadata, signing status (honestly reporting unsigned iOS status when applicable), and verification test results.
