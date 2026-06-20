# Living Docs — convenience wrapper around install.sh.
# Run `make help` for the list of targets.

SHELL := /bin/bash
INSTALL := ./install.sh

.DEFAULT_GOAL := help
.PHONY: help install install-claude install-cursor install-copilot \
        install-opencode install-codex install-pi install-all install-pocock \
        project-claude project-opencode project-codex project-pi \
        uninstall uninstall-all check lint lint-docs test-lint-docs test-ratchet version \
        sync-plugin-skill check-plugin-skill-sync

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

check: version test-lint-docs test-ratchet lint-docs check-plugin-skill-sync ## Check version sync, validate install.sh, dry-run harnesses, test+run the docs linter (+ ratchet), assert the plugin's vendored skill is in sync
	bash -n install.sh
	bash -n skills/living-docs/scripts/lint-docs.sh
	bash -n enforcement/pre-commit.sh
	bash -n plugins/living-docs-enforcer/hooks/block-docs-drift.sh
	bash -n scripts/check-version.sh
	$(INSTALL) all --dry-run

lint-docs: ## Validate the example docs bundle against the Living Docs invariants
	./skills/living-docs/scripts/lint-docs.sh examples/linkly/docs

test-lint-docs: ## Run the lint-docs.sh fixture/parity corpus (clean passes, each violation caught)
	./tests/lint-docs/run.sh

test-ratchet: ## Run the diff-aware ratchet corpus (new violation blocks, pre-existing debt grandfathered)
	./tests/lint-docs/run-ratchet.sh

version: ## Assert the release version is consistent across VERSION and every SKILL.md
	./scripts/check-version.sh

sync-plugin-skill: ## Regenerate the plugin's vendored living-docs skill from the canonical source
	rm -rf plugins/living-docs-enforcer/skills/living-docs
	mkdir -p plugins/living-docs-enforcer/skills
	cp -R skills/living-docs plugins/living-docs-enforcer/skills/living-docs
	@echo "synced: skills/living-docs -> plugins/living-docs-enforcer/skills/living-docs"

check-plugin-skill-sync: ## Fail if the plugin's vendored living-docs skill drifted from the source
	@if diff -r skills/living-docs plugins/living-docs-enforcer/skills/living-docs >/dev/null 2>&1; then \
		echo "plugin vendored skill is in sync with skills/living-docs"; \
	else \
		echo "ERROR: plugins/living-docs-enforcer/skills/living-docs has DRIFTED from skills/living-docs." >&2; \
		echo "       Edit the source (skills/living-docs/), then run: make sync-plugin-skill" >&2; \
		diff -r skills/living-docs plugins/living-docs-enforcer/skills/living-docs >&2 || true; \
		exit 1; \
	fi

lint: check ## Alias for check
