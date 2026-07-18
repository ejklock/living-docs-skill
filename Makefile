# Living Docs — convenience wrapper around install.sh.
# Run `make help` for the list of targets.

SHELL := /bin/bash
INSTALL := ./install.sh

# Dev-env docker-compose (issue 0007): pulls POSTGRES_USER/POSTGRES_DB/PG_PORT from .env
# into the targets below. The leading `-` makes a missing .env non-fatal (docker compose
# itself also reads .env for ${VAR} substitution in docker-compose.yml).
-include .env
export

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
.PHONY: help install install-claude install-cursor install-copilot \
        install-opencode install-codex install-pi install-all install-pocock \
        project-claude project-opencode project-codex project-pi \
        uninstall uninstall-all check lint test-fixtures version \
        cli-dev-image cli-build cli-test cli-fmt cli-clippy build cli-install \
        up down db-up db-psql db-logs db-test

help: ## Show this help
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN{FS=":.*?## "}{printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

install: install-claude ## Install for Claude Code (global) — the default

install-claude: ## Install the skills for Claude Code (~/.claude/skills)
	$(INSTALL) claude

install-cursor: ## Install the Living Docs rule for Cursor (.cursor/rules)
	$(INSTALL) cursor

install-copilot: ## Install the Living Docs instruction for GitHub Copilot (.github/instructions)
	$(INSTALL) copilot

install-opencode: ## Install the skills for OpenCode (~/.config/opencode/skills)
	$(INSTALL) opencode

install-codex: ## Install the skills for Codex (~/.codex/skills)
	$(INSTALL) codex

install-pi: ## Install the skills for Pi (~/.pi/agent/skills + AGENTS.md)
	$(INSTALL) pi

install-all: ## Install for every supported harness
	$(INSTALL) all

install-pocock: ## Clone Matt Pocock's companion skills (grill-me, to-prd, to-issues) — MIT
	$(INSTALL) pocock

project-claude: ## Install for Claude Code into the current project (.claude/skills)
	$(INSTALL) claude --project

project-opencode: ## Install for OpenCode into the current project (.opencode/skills)
	$(INSTALL) opencode --project

project-codex: ## Install for Codex into the current project (.codex/skills)
	$(INSTALL) codex --project

project-pi: ## Install for Pi into the current project (.pi/skills)
	$(INSTALL) pi --project

uninstall: ## Remove the global Claude Code install
	$(INSTALL) claude --uninstall

uninstall-all: ## Remove the install for every supported harness
	$(INSTALL) all --uninstall

check: version build test-fixtures ## Check version sync, validate install.sh, run Rust tests, living-docs check + mermaid, dry-run harnesses
	bash -n install.sh
	bash -n scripts/check-version.sh
	cargo test --manifest-path cli/Cargo.toml
	$(LIVING_DOCS_BIN) check examples/linkly/docs
	$(LIVING_DOCS_BIN) check --mermaid-only
	$(INSTALL) all --dry-run

test-fixtures: build ## Run the hostile/negative fixtures that guard the check parsers
	LIVING_DOCS_BIN=$(LIVING_DOCS_BIN) ./skills/living-docs/tests/run.sh

version: ## Assert the release version is consistent across VERSION and every SKILL.md
	./scripts/check-version.sh

lint: check ## Alias for check

# --- cli/ (Rust) — Docker-always dev targets ---
# cli-* targets run cargo inside the pinned Dockerfile.dev image. `build`/`cli-install`
# use the host cargo instead (the "native" path) — see cli/rust-toolchain.toml for the
# pinned version both paths agree on. NOTE: `install` is already taken by the skill
# installer above, so the native CLI install target is `cli-install`, not `install`.

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

cli-clippy: cli-dev-image ## Lint the CLI inside the dev image (clippy --all-targets -D warnings)
	@mkdir -p "$(HOME)/.cargo/registry"
	$(DOCKER_CARGO) cargo clippy --manifest-path cli/Cargo.toml --all-targets -- -D warnings

build: ## Build the release CLI binary natively (host cargo) -> target/release/living-docs
	cargo build --release --manifest-path cli/Cargo.toml

cli-install: build ## Install the CLI binary onto PATH natively (host cargo, idempotent)
	cargo install --path cli --force

# Provisions the ParadeDB (Postgres + BM25) service from ADR 0004 for local db-mode work.
# The compose `web` service is deferred to issues 0004/0006.

up: ## Start every compose service in the background
	docker compose up -d

down: ## Stop compose services (the named paradedb-data volume is kept)
	docker compose down

db-up: ## Start only the paradedb service and block until its healthcheck passes
	docker compose up -d --wait paradedb

db-psql: ## Open a psql shell against the composed paradedb service
	docker compose exec paradedb psql -U $(POSTGRES_USER) -d $(POSTGRES_DB)

db-logs: ## Follow the paradedb service logs
	docker compose logs -f paradedb

db-test: db-up ## Run the db-store dual-engine test suite against the composed DB
	LIVING_DOCS_TEST_PG_URL=$(DATABASE_URL) cargo test --manifest-path db-store/Cargo.toml
