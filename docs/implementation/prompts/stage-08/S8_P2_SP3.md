# Subphase S8.P2.SP3 Execution Prompt: Dependency License Compliance, Reproducible Build Attestation & SBOM Signing

## 1. Role & Context
You are the Software Supply Chain Security & Release Compliance Engineer for UMCF. In Subphase `S8.P2.SP3`, you will implement the supply-chain verification pipeline (`deny.toml`, `scripts/release/supply_chain_verify.rs`, and `crates/umcf-core/src/attestation.rs`), enforcing strict dependency license compliance, zero known vulnerability advisories across Rust/NPM/Dart/Python dependencies, deterministic reproducible builds, and cryptographically signed CycloneDX/SPDX Software Bills of Materials (SBOMs) and SLSA v1.0 provenance attestations.

## 2. Prerequisites & Dependencies
- `S8.P2.SP1` & `S8.P2.SP2` complete.
- Workspace lockfiles (`Cargo.lock`, `ui/admin_web/pnpm-lock.yaml`, `sdks/typescript/pnpm-lock.yaml`, `ui/flutter_app/pubspec.lock`).

## 3. Implementation Tasks
1. **Strict Dependency & License Gate (`deny.toml`)**:
   - Configure `cargo-deny` at the workspace root (`g:\ucf\deny.toml`):
     - **`[advisories]`**: Deny all `vulnerability = "deny"`, `unmaintained = "warn"`, `yanked = "deny"`, `notice = "deny"`.
     - **`[licenses]`**: Allow only approved OSI permissive licenses (`MIT`, `Apache-2.0`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Zlib`, `Unicode-DFS-2016`, `MPL-2.0`, `CC0-1.0`); strictly `deny` copyleft `GPL-*` / `AGPL-*` / `SSPL-*` or unlicensed crates.
     - **`[bans]`**: Deny wildcard dependencies (`wildcards = "deny"`) and audit duplicate transitive versions.
     - **`[sources]`**: Allow only `https://github.com/rust-lang/crates.io-index` and pinned, SHA-256-verified vendor submodules (`vendor/llama.cpp`); deny unknown git repositories.
   - Extend license and CVE scanning to `pnpm audit` / `license-checker` (for Svelte 5 & TypeScript SDK), `flutter pub deps` (for Flutter), and `pip-audit` (for Python SDK).
2. **Deterministic Reproducible Build Pipeline (`scripts/release/reproducible_build.rs`)**:
   - Configure deterministic compiler flags across the workspace:
     - Set `SOURCE_DATE_EPOCH` to the commit timestamp.
     - Pass `--remap-path-prefix $WORKSPACE_ROOT=/umcf-src` in `RUSTFLAGS` and CMake `CMAKE_CXX_FLAGS` so build host paths (`C:\Users\...` or `/home/...`) never leak into compiled binaries or debug symbols.
     - Enforce `CARGO_INCREMENTAL=0`, `codegen-units = 1`, and deterministic archive ordering (`Zero-timestamp ar/tar/zip`).
   - Verify that building `umcf-daemon` and `umcf` CLI twice from clean checkouts in two different working directory paths produces **bit-identical SHA-256 / BLAKE3 binary hashes**.
3. **Signed CycloneDX/SPDX SBOM & Binary Self-Attestation (`crates/umcf-core/src/attestation.rs`)**:
   - Generate `umcf-v1.0.0.sbom.cyclonedx.json` and `umcf-v1.0.0.sbom.spdx.json` enumerating every Rust crate, C/C++ library (`llama.cpp`/`ggml`), NPM package, Dart package, and Python package with exact version, license SPDX expression, and cryptographic package hash.
   - Sign the release binaries, SBOMs, and SLSA v1.0 provenance manifest (`umcf-v1.0.0.intoto.jsonl`) with the UMCF Release Authority Ed25519 key.
   - Implement `umcf version --verify-provenance` and embedded binary startup self-check so nodes in a cluster can verify that joining peers are running an authentic, unmodified UMCF release build.

## 4. Acceptance Criteria
- `cargo deny check` and `cargo audit` exit with code `0` (0 vulnerabilities, 0 license violations, 0 unverified sources).
- Two independent clean builds from different directory paths (`build_dir_a` vs `build_dir_b`) with identical `SOURCE_DATE_EPOCH` produce identical binary BLAKE3 hashes (`hash(build_a) == hash(build_b)`), and `strings <binary>` contains 0 occurrences of the host user's home directory path.
- Generated CycloneDX 1.5 SBOM validates against the official CycloneDX JSON schema and includes 100% of workspace dependencies.
- `umcf version --verify-provenance` cryptographically verifies the binary's Ed25519 signature and SBOM digest in < 10 ms.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_slsa_provenance_and_sbom_ed25519_signature_verification`
  - `test_tampered_binary_or_sbom_hash_rejected_by_provenance_verifier`
  - `test_spdx_license_expression_parser_rejects_gpl_and_agpl`
- **`[INT]`**:
  - `test_cargo_deny_and_polyglot_dependency_license_compliance_gate`
  - `test_dual_directory_reproducible_build_bit_exact_binary_comparison`
  - `test_binary_contains_zero_host_workspace_path_strings`
- **`[SIM]`**:
  - Simulate a peer attempting to join a `StrictVerifiedBuildOnly` cluster while presenting a modified binary attestation; verify cluster admission denial.
- **`[HW]`**:
  - Execute full release build + SBOM generation + signature verification on host platform.
- **`[MANUAL]`**:
  - Run `umcf version --verify-provenance --format json` in terminal and inspect the SBOM & Third-Party Licenses viewer in Flutter (`About -> Open Source Licenses`) and Svelte 5 (`/settings/about`).

## 6. Required Evidence & Artifacts
- `artifacts/stage-08/p2/sp3_cargo_deny_compliance_report.json`
- `artifacts/stage-08/p2/sp3_reproducible_build_hashes.json`
- `artifacts/stage-08/p2/sp3_umcf_v1.0.0_sbom.cyclonedx.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document reproducible build instructions, `SOURCE_DATE_EPOCH`, and SBOM verification in `docs/SECURITY_AUDIT.md` (Section 3).
- Mark `S8.P2.SP3` and Phase `S8.P2` complete in `TODO.md`; update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: Any non-permissive dependency license, any unpatched CVE advisory, or any host filesystem path leaked into release binaries.
- **Completion Gate**: Phase `S8.P2` completion gate passed (`S8_P2_SP1`..`SP3` green). Proceed to Phase `S8.P3`.
