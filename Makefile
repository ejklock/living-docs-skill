# Living Docs — build, lint and gate targets, plus a thin `cli-install`
# wrapper over install.sh (which bootstraps the living-docs binary only).
# Run `make help` for the list of targets.

SHELL := /bin/bash
INSTALL := ./install.sh

# Docker-always dev environment for cli/ (Rust). The host is not assumed to have a
# toolchain — Dockerfile.dev pins the exact version from cli/rust-toolchain.toml, plus
# rustfmt/clippy/build-essential. Mounts the repo + the host cargo registry (reused
# across runs) and runs as the host uid:gid so target stays host-owned.
DEV_IMAGE := living-docs-dev
DOCKER_CARGO = docker run --rm \
	-u "$$(id -u):$$(id -g)" \
	-e HOME=/tmp \
	-v "$(CURDIR):/work" \
	-v "$$HOME/.cargo/registry:/usr/local/cargo/registry" \
	-w /work \
	$(DEV_IMAGE)

# Native release binary built by `build`; reused by `check`/`test-fixtures` so the
# invariant checks and hostile fixtures don't each trigger their own compile.
LIVING_DOCS_BIN := target/release/living-docs

.DEFAULT_GOAL := help
.PHONY: help check lint test-fixtures \
        test-release-gate test-version-gate version \
        test-filesize-gate filesize \
        allow-inventory test-allow-inventory-gate test-install-gate test-npm-gate \
        cli-dev-image cli-build cli-test cli-fmt cli-clippy build cli-install

help: ## Show this help
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN{FS=":.*?## "}{printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

check: version filesize allow-inventory build test-fixtures test-release-gate test-version-gate test-filesize-gate test-allow-inventory-gate test-install-gate test-npm-gate ## Check version sync, file-size ratchet, allow-inventory gate, validate install.sh, run Rust tests, living-docs check + mermaid, hook fixtures, release-asset gate fixtures, version-gate fixtures, file-size gate fixtures, allow-inventory gate fixtures, install gate fixtures, dry-run install.sh
	bash -n install.sh
	bash -n scripts/check-version.sh
	bash -n scripts/verify-release-assets.sh
	bash -n scripts/check-file-size.sh
	bash -n scripts/check-allow-inventory.sh
	bash -n scripts/tests/install/run.sh
	bash -n scripts/tests/npm/run.sh
	cargo test --manifest-path cli/Cargo.toml
	$(LIVING_DOCS_BIN) check --require-owner --plain examples/linkly/docs
	$(LIVING_DOCS_BIN) check --mermaid-only
	$(INSTALL) --dry-run

test-fixtures: build ## Run the hostile/negative fixtures that guard the check parsers
	LIVING_DOCS_BIN=$(LIVING_DOCS_BIN) ./skills/living-docs/tests/run.sh

test-release-gate: ## Run the verify-release-assets.sh fixtures (ADR 0024), stubbed gh
	./scripts/tests/verify-release-assets/run.sh

test-version-gate: ## Run the check-version.sh fixtures, synthetic repos
	./scripts/tests/check-version/run.sh

test-filesize-gate: ## Run the check-file-size.sh fixtures, synthetic repos
	./scripts/tests/check-file-size/run.sh

test-allow-inventory-gate: ## Run the check-allow-inventory.sh fixtures, synthetic repos
	./scripts/tests/check-allow-inventory/run.sh

test-install-gate: ## Run the install.sh fixtures (ADR 0041), stubbed curl
	./scripts/tests/install/run.sh

test-npm-gate: ## Run the npm package fixtures, local HTTP stub, no registry
	./scripts/tests/npm/run.sh

version: ## Assert the release version is consistent across VERSION and every SKILL.md
	./scripts/check-version.sh

filesize: ## Assert every .rs file stays within the 300-line ratchet (issue 0028)
	./scripts/check-file-size.sh

allow-inventory: ## Assert the clippy::too_many_lines allow annotations only shrink (issue 0028)
	./scripts/check-allow-inventory.sh

lint: check ## Alias for check

# --- cli/ (Rust) — Docker-always dev targets ---
# cli-* targets run cargo inside the pinned Dockerfile.dev image. `build` uses host
# cargo to compile locally (see cli/rust-toolchain.toml for the pinned version) ->
# target/release/living-docs. `cli-install` fetches the published release binary via
# install.sh (ADR 0041); it never compiles.

cli-dev-image: ## Build the pinned Rust dev image (rustfmt + clippy + build-essential)
	docker build -f Dockerfile.dev -t $(DEV_IMAGE) .

cli-build: cli-dev-image ## Build the CLI inside the dev image (cargo build)
	@mkdir -p "$(HOME)/.cargo/registry"
	$(DOCKER_CARGO) cargo build --manifest-path cli/Cargo.toml

cli-test: cli-dev-image ## Run the CLI test suite inside the dev image
	@mkdir -p "$(HOME)/.cargo/registry"
	$(DOCKER_CARGO) cargo test --manifest-path cli/Cargo.toml

cli-fmt: cli-dev-image ## Check CLI formatting inside the dev image (cargo fmt --check)
	@mkdir -p "$(HOME)/.cargo/registry"
	$(DOCKER_CARGO) cargo fmt --manifest-path cli/Cargo.toml --check

cli-clippy: cli-dev-image ## Lint the CLI inside the dev image (clippy --all-targets -D warnings -W clippy::too_many_lines)
	@mkdir -p "$(HOME)/.cargo/registry"
	$(DOCKER_CARGO) cargo clippy --manifest-path cli/Cargo.toml --all-targets -- -D warnings -W clippy::too_many_lines

build: ## Build the release CLI binary natively (host cargo) -> target/release/living-docs
	cargo build --release --manifest-path cli/Cargo.toml

cli-install: ## Install the living-docs CLI from the latest GitHub release
	$(INSTALL) cli

