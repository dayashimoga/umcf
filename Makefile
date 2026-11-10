.PHONY: default verify lint test build build-all quota-check check-placeholders verify-governance clean

default: verify

verify: lint test quota-check check-placeholders verify-governance

lint:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings
	bash scripts/check-no-placeholders.sh

test:
	cargo test --workspace

build:
	cargo build --workspace

build-all:
	cargo build --workspace --release
	node apps/web-dashboard/scripts/build.mjs
	node apps/web_console/scripts/build.mjs
	node services/control-plane-cf/scripts/build.mjs
	node services/cf_control_plane/scripts/build.mjs

quota-check:
	bash scripts/podman-quota-check.sh

check-placeholders:
	bash scripts/check-no-placeholders.sh

verify-governance:
	bash scripts/verify-governance.sh

clean:
	cargo clean
