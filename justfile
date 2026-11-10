# UMCF Unified Task Runner & Build Orchestrator (justfile)
# Enforces zero-warning quality gates, 30 GB Podman quota ceiling, and multi-language builds.

default:
    @just --list

# Run all quality, safety, and governance verification gates
verify: lint test quota-check check-placeholders verify-governance

# Check formatting and static analysis across Rust, Web, and shell scripts
lint:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    bash scripts/check-no-placeholders.sh

# Run workspace unit, integration, and simulation tests
test:
    cargo test --workspace

# Build all workspace binaries and release targets
build:
    cargo build --workspace

# Build all workspace components across Rust, Web, and Cloudflare services
build-all:
    cargo build --workspace --release
    node apps/web-dashboard/scripts/build.mjs
    node apps/web_console/scripts/build.mjs
    node services/control-plane-cf/scripts/build.mjs
    node services/cf_control_plane/scripts/build.mjs

# Verify Podman 30 GB hard storage ceiling and auto-prune guards
quota-check:
    bash scripts/podman-quota-check.sh

# Scan production code paths for forbidden placeholders
check-placeholders:
    bash scripts/check-no-placeholders.sh

# Verify presence, non-emptiness, and schemas of all 55+ governance files and 72 subphases
verify-governance:
    bash scripts/verify-governance.sh

# Clean build artifacts while preserving Podman caches
clean:
    cargo clean
