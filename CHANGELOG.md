# CHANGELOG — UNIVERSAL MOBILE COMPUTE FABRIC (UMCF)

> **GOVERNANCE NOTICE:** This file is an **append-only** chronological record (`ENGINEERING_CONTRACT.md` §2.7). Existing entries must never be overwritten or deleted. All new stage, phase, subphase, and release completions must be appended below.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0-bootstrap] — 2026-11-10

### Added
- **Monorepo & Git Initialization**:
  - Initialized repository with `main` and `develop` branches, `.gitignore`, `.gitattributes`, and `.editorconfig`.
  - Established Rust workspace (`Cargo.toml` + 7 crates: `umcf-proto`, `umcf-core`, `umcf-coordinator`, `umcf-worker`, `umcf-scheduler`, `umcf-cli`, `umcf-daemon`).
  - Established Svelte 5 + TypeScript Web Dashboard (`apps/web-dashboard`) and Flutter Cross-Platform Mobile/Desktop application (`apps/mobile-desktop`).
  - Established Cloudflare Workers + Durable Objects primary control plane (`services/control-plane-cf`) and Google Cloud Run stateless fallback control plane (`services/control-plane-cloudrun`).
- **Engineering Governance & Antigravity Customizations**:
  - Created `ENGINEERING_CONTRACT.md`, `AGENTS.md`, `SECURITY.md`, `CONTRIBUTING.md`, `TODO.md`, and `CHANGELOG.md`.
  - Created 7 workspace rules in `.agent/rules/`, 9 domain skills in `.agent/skills/`, and 6 repeatable workflows in `.agent/workflows/`.
- **Architecture, Requirements & 8-Stage Implementation Roadmap**:
  - Authored `docs/architecture/SYSTEM_OVERVIEW.md`, `COMPONENT_CONTRACTS.md`, `DATA_FLOWS.md`, `RISK_REGISTER_AND_PROTOTYPES.md`, and `ADR-0001` through `ADR-0006`.
  - Authored `docs/requirements/FUNCTIONAL_REQUIREMENTS.md` (`FR-001`..`FR-080`), `NON_FUNCTIONAL_REQUIREMENTS.md` (`NFR-001`..`NFR-040`), and `TRACEABILITY_MATRIX.md`.
  - Authored `docs/implementation/MASTER_ROADMAP.md` and 104 standalone executable Stage, Phase, and Subphase prompts across `docs/implementation/prompts/stage-01/` through `stage-08/`.
- **Reproducible Podman Environment (30 GB Cap) & CI/CD Workflows**:
  - Added `infra/podman/` multi-target container definitions and `scripts/enforce-podman-quota.ps1` / `scripts/enforce-podman-quota.sh` enforcing the strict 30 GB storage limit.
  - Added `.github/workflows/ci-rust.yml`, `ci-web.yml`, `ci-flutter.yml`, `ci-security-sbom.yml`, and `release.yml` with independent artifact uploads, SHA-256 checksums, build metadata, and SBOM generation.

