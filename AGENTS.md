# AGENTS.md — Universal Mobile Compute Fabric (UMCF)

This file defines global instructions, workspace conventions, and customization discovery for Antigravity agents working in the **Universal Mobile Compute Fabric (UMCF)** monorepo.

---

## 1. Authoritative Governance & References

Before modifying code or executing any stage/phase/subphase, always consult:
- [`ENGINEERING_CONTRACT.md`](./ENGINEERING_CONTRACT.md) — Mandatory architectural invariants, quality gates (`>=95%` meaningful coverage, `100%` pass rate), security standards, and Definition of Done.
- [`docs/implementation/MASTER_ROADMAP.md`](./docs/implementation/MASTER_ROADMAP.md) — The 8-Stage, 24-Phase, 72-Subphase master implementation roadmap.
- [`docs/architecture/SYSTEM_OVERVIEW.md`](./docs/architecture/SYSTEM_OVERVIEW.md) & [`docs/architecture/COMPONENT_CONTRACTS.md`](./docs/architecture/COMPONENT_CONTRACTS.md) — System design and strict interface contracts.
- [`docs/requirements/TRACEABILITY_MATRIX.md`](./docs/requirements/TRACEABILITY_MATRIX.md) — End-to-end requirement traceability (`FR-001`..`FR-080`, `NFR-001`..`NFR-040`).
- [`TODO.md`](./TODO.md) & [`CHANGELOG.md`](./CHANGELOG.md) — Strictly **append-only** project tracking logs.

---

## 2. Repository Directory Map

```text
ucf/
├── ENGINEERING_CONTRACT.md       # Mandatory engineering governance contract
├── AGENTS.md                     # Global Antigravity instructions (this file)
├── TODO.md                       # Append-only task & milestone ledger
├── CHANGELOG.md                  # Append-only chronological change log
├── SECURITY.md                   # Threat model, cryptographic policy & vulnerability reporting
├── Cargo.toml                    # Root Rust workspace manifest
├── .agent/                       # Antigravity workspace customizations
│   ├── rules/                    # Mandatory global & component engineering rules
│   ├── skills/                   # Reusable specialized agent skills (SKILL.md)
│   └── workflows/                # Repeatable implementation, audit & release workflows
├── .github/                      # GitHub Actions CI/CD workflows & issue/PR templates
│   ├── workflows/                # ci-rust, ci-web, ci-flutter, ci-security-sbom, release
│   └── ISSUE_TEMPLATE/           # Structured YAML issue templates
├── crates/                       # Rust-first core crates & binaries
│   ├── umcf-proto/               # Versioned wire protocol & capability negotiation schemas
│   ├── umcf-core/                # Identity, roles, hardware descriptors, config & errors
│   ├── umcf-coordinator/         # Master/Coordinator cluster registry & lease manager
│   ├── umcf-worker/              # Worker runtime, resource guard & hardware probe traits
│   ├── umcf-scheduler/           # Heterogeneous capability-aware DAG scheduler
│   ├── umcf-cli/                 # `umcf` developer & operator CLI binary
│   └── umcf-daemon/              # `umcfd` node runtime daemon binary
├── apps/
│   ├── mobile-desktop/           # Flutter cross-platform app (Android, iOS, Windows, Linux, macOS)
│   └── web-dashboard/            # Svelte 5 + TypeScript web/admin dashboard
├── services/
│   ├── control-plane-cf/         # Cloudflare Workers + Durable Objects primary control plane
│   └── control-plane-cloudrun/   # Google Cloud Run stateless container fallback control plane
├── infra/
│   └── podman/                   # Reproducible Podman container definitions & 30 GB storage config
├── scripts/                      # Quota enforcement, verification, packaging & SBOM scripts
└── docs/
    ├── architecture/             # System diagrams, component contracts, data flows, ADRs
    ├── requirements/             # Functional, non-functional & traceability matrix
    ├── implementation/           # MASTER_ROADMAP.md & 104 executable stage/phase/subphase prompts
    └── certification/            # Verification evidence & bootstrap/stage certification reports
```

---

## 3. Antigravity Rules, Skills & Workflows Index

### 3.1 Workspace Rules (`.agent/rules/`)
- [`.agent/rules/00-global-governance.md`](./.agent/rules/00-global-governance.md)
- [`.agent/rules/01-rust-standards.md`](./.agent/rules/01-rust-standards.md)
- [`.agent/rules/02-flutter-standards.md`](./.agent/rules/02-flutter-standards.md)
- [`.agent/rules/03-svelte-web-standards.md`](./.agent/rules/03-svelte-web-standards.md)
- [`.agent/rules/04-security-and-privacy.md`](./.agent/rules/04-security-and-privacy.md)
- [`.agent/rules/05-hardware-and-benchmarking.md`](./.agent/rules/05-hardware-and-benchmarking.md)
- [`.agent/rules/06-ci-cd-and-podman.md`](./.agent/rules/06-ci-cd-and-podman.md)

### 3.2 Workspace Skills (`.agent/skills/`)
- `umcf-architecture` — [`.agent/skills/umcf-architecture/SKILL.md`](./.agent/skills/umcf-architecture/SKILL.md)
- `umcf-rust-dev` — [`.agent/skills/umcf-rust-dev/SKILL.md`](./.agent/skills/umcf-rust-dev/SKILL.md)
- `umcf-flutter-dev` — [`.agent/skills/umcf-flutter-dev/SKILL.md`](./.agent/skills/umcf-flutter-dev/SKILL.md)
- `umcf-svelte-dev` — [`.agent/skills/umcf-svelte-dev/SKILL.md`](./.agent/skills/umcf-svelte-dev/SKILL.md)
- `umcf-distributed-systems` — [`.agent/skills/umcf-distributed-systems/SKILL.md`](./.agent/skills/umcf-distributed-systems/SKILL.md)
- `umcf-networking-p2p` — [`.agent/skills/umcf-networking-p2p/SKILL.md`](./.agent/skills/umcf-networking-p2p/SKILL.md)
- `umcf-security-audit` — [`.agent/skills/umcf-security-audit/SKILL.md`](./.agent/skills/umcf-security-audit/SKILL.md)
- `umcf-testing-verification` — [`.agent/skills/umcf-testing-verification/SKILL.md`](./.agent/skills/umcf-testing-verification/SKILL.md)
- `umcf-devops-podman` — [`.agent/skills/umcf-devops-podman/SKILL.md`](./.agent/skills/umcf-devops-podman/SKILL.md)

### 3.3 Repeatable Workflows (`.agent/workflows/`)
- [`.agent/workflows/stage-execution-workflow.md`](./.agent/workflows/stage-execution-workflow.md)
- [`.agent/workflows/audit-workflow.md`](./.agent/workflows/audit-workflow.md)
- [`.agent/workflows/subphase-implementation-workflow.md`](./.agent/workflows/subphase-implementation-workflow.md)
- [`.agent/workflows/code-review-workflow.md`](./.agent/workflows/code-review-workflow.md)
- [`.agent/workflows/testing-and-coverage-workflow.md`](./.agent/workflows/testing-and-coverage-workflow.md)
- [`.agent/workflows/release-and-packaging-workflow.md`](./.agent/workflows/release-and-packaging-workflow.md)
- [`.agent/workflows/hardware-certification-workflow.md`](./.agent/workflows/hardware-certification-workflow.md)

---

## 4. Operational Directives for Agents

1. **Podman 30 GB Quota Check**: Always run `pwsh scripts/enforce-podman-quota.ps1` (Windows) or `bash scripts/enforce-podman-quota.sh` (Linux/macOS) before building or pulling container images.
2. **Stage-by-Stage Execution**: Execute only the explicitly requested Stage, Phase, or Subphase prompt from `docs/implementation/prompts/`. Never jump ahead to future stages without user approval.
3. **Evidence-Driven Completion**: Never claim a subphase or stage is complete without running the test suite, capturing command outputs, classifying tests (`[UNIT]`, `[INTEGRATION]`, `[SIMULATION]`, `[HARDWARE-REQUIRED]`, `[MANUALLY-VERIFIED]`), and updating `TODO.md` and `CHANGELOG.md`.

