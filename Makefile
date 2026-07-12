# Living Docs — convenience wrapper around install.sh.
# Run `make help` for the list of targets.

SHELL := /bin/bash
INSTALL := ./install.sh

.DEFAULT_GOAL := help
.PHONY: help install install-claude install-cursor install-copilot \
        install-opencode install-codex install-pi install-all install-pocock \
        project-claude project-opencode project-codex project-pi \
        uninstall uninstall-all check lint lint-docs lint-mermaid test-fixtures version \
        lint-docker-build lint-docker

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

check: version lint-docs lint-mermaid test-fixtures ## Check version sync, validate install.sh, dry-run harnesses, lint docs, validate mermaid, run fixtures
	bash -n install.sh
	bash -n skills/living-docs/scripts/lint-docs.sh
	bash -n skills/living-docs/scripts/lint-mermaid.sh
	bash -n skills/living-docs/tests/run.sh
	bash -n scripts/check-version.sh
	$(INSTALL) all --dry-run

lint-docs: ## Validate the example docs bundle against the Living Docs invariants
	./skills/living-docs/scripts/lint-docs.sh examples/linkly/docs

lint-mermaid: ## Validate every fenced mermaid block via the real parser (requires Docker)
	./skills/living-docs/scripts/lint-mermaid.sh

test-fixtures: ## Run the hostile/negative fixtures that guard the lint-docs parsers
	./skills/living-docs/tests/run.sh

lint-docker-build: ## Build the self-contained linter image (bundles lychee + yq + jq)
	docker build -f Dockerfile.lint -t living-docs-lint .

lint-docker: lint-docker-build ## Lint the example corpus via Docker (no host tools needed)
	docker run --rm -v "$(CURDIR):/work" living-docs-lint examples/linkly/docs

version: ## Assert the release version is consistent across VERSION and every SKILL.md
	./scripts/check-version.sh

lint: check ## Alias for check
