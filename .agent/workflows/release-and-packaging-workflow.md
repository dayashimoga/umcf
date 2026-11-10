---
description: Multi-platform build, checksum, SBOM, release manifest, Podman quota check, and artifact verification workflow for UMCF.
---

# UMCF Multi-Platform Release & Packaging Workflow

Follow this workflow to build, package, verify, and manifest all UMCF platform deliverables while respecting the strict **30 GB Podman storage quota** and **independent artifact (zero nested zip)** packaging rules.

---

## Step 1: Pre-Release Readiness & Quota Verification

1. Run `.agent/workflows/audit-workflow.md` and `.agent/workflows/testing-and-coverage-workflow.md` to confirm 100% test pass rate, >= 95% coverage, and zero clippy/linter warnings.
2. Check Podman disk usage:
   ```powershell
   podman system df
   ```
   Ensure total Podman storage is **< 20 GB** before starting containerized cross-builds (hard limit: **30 GB**).

---

## Step 2: Build Independent Platform Deliverables

Create a clean output directory `dist/v<version>/` and build each platform artifact directly into `dist/v<version>/` without nesting archives inside archives:

1. **Windows (`x86_64-pc-windows-msvc`)**:
   - Build release Rust binaries (`umcf-cli.exe`, `umcf-node.exe`, `umcf_ffi.dll`) with `--release`.
   - Build Flutter Windows desktop release (`flutter build windows --release`).
   - Package into `dist/v<version>/umcf-v<version>-windows-x86_64.zip` and standalone CLI binary `dist/v<version>/umcf-cli-v<version>-windows-x86_64.exe`.
2. **Linux (`x86_64-unknown-linux-gnu` & `aarch64-unknown-linux-gnu`)**:
   - Build stripped release binaries and Flutter Linux bundle inside the rootless Podman builder container (monitoring the 30 GB quota).
   - Emit `dist/v<version>/umcf-v<version>-linux-x86_64.tar.gz` and `dist/v<version>/umcf-v<version>-linux-aarch64.tar.gz`.
3. **Android (`.apk` & `.aab`)**:
   - Cross-compile Rust JNI libraries (`arm64-v8a`, `x86_64`) via cargo-ndk.
   - Build `flutter build apk --release` and `flutter build appbundle --release`.
   - Copy `umcf-v<version>-android-universal.apk` and `umcf-v<version>-android.aab` directly to `dist/v<version>/`.
4. **macOS & iOS (Honest Signing & Cross-Platform Packaging)**:
   - On macOS hosts: build `umcf-v<version>-macos-universal.tar.gz` and iOS archive.
   - When Apple Developer code-signing certificates are absent (or when building on Windows/Linux hosts):
     - Produce the verified iOS/macOS project & preflight bundle explicitly named `umcf-v<version>-ios-unsigned-source-archive.tar.gz` (or `.xcarchive`).
     - **Never fabricate fake signatures**. Document `signing_status: "UNSIGNED_REQUIRES_APPLE_DEVELOPER_CERT"` in `release-manifest.json` and include `IOS_SIGNING_INSTRUCTIONS.md`.
5. **Web Dashboard & Control Plane Worker**:
   - Build `apps/web_dashboard` production bundle and `services/control_plane_worker` worker bundle.
   - Emit `dist/v<version>/umcf-v<version>-web-control-plane.tar.gz`.

---

## Step 3: Verify Zero Nested Archives

Inspect every `.zip` and `.tar.gz` in `dist/v<version>/` to verify that no archive contains another `.zip`, `.tar.gz`, `.7z`, or `.rar` archive inside it.

---

## Step 4: Generate SBOM, SHA-256 Checksums & Release Manifest

1. **Software Bill of Materials (`sbom.cdx.json`)**:
   - Generate CycloneDX SBOMs aggregating `Cargo.lock`, `pubspec.lock`, and `package-lock.json` into `dist/v<version>/sbom.cdx.json`.
2. **Cryptographic Checksums (`SHA256SUMS.txt`)**:
   - Compute SHA-256 digests for all artifacts in `dist/v<version>/` and write `dist/v<version>/SHA256SUMS.txt`.
3. **Release Manifest (`release-manifest.json`)**:
   - Write `dist/v<version>/release-manifest.json` with version, git commit SHA, build timestamp, per-artifact size + SHA-256 + signing status, test coverage summary, and Podman storage footprint.

---

## Step 5: Smoke-Test Packaged Artifacts & Post-Build Quota Check

1. Verify every checksum in `SHA256SUMS.txt`.
2. Extract and run `umcf-cli --version` and `umcf-cli doctor` from the packaged release archive to verify dynamic library resolution and binary integrity.
3. Run `podman system df` and prune intermediate build cache so final Podman disk usage remains strictly under **30 GB**.
