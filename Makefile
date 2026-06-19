# Living Docs — convenience wrapper around install.sh.
# Run `make help` for the list of targets.

SHELL := /bin/bash
INSTALL := ./install.sh

.DEFAULT_GOAL := help
.PHONY: help install install-claude install-cursor install-copilot \
        install-opencode install-pi install-all install-pocock \
        project-claude project-opencode project-pi \
        uninstall uninstall-all check lint

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

install-pi: ## Install the skills for Pi (~/.pi/agent/skills)
	$(INSTALL) pi

install-all: ## Install for every supported harness
	$(INSTALL) all

install-pocock: ## Clone Matt Pocock's companion skills (grill-me, to-prd, to-issues) — MIT
	$(INSTALL) pocock

project-claude: ## Install for Claude Code into the current project (.claude/skills)
	$(INSTALL) claude --project

project-opencode: ## Install for OpenCode into the current project (.opencode/skills)
	$(INSTALL) opencode --project

project-pi: ## Install for Pi into the current project (.pi/skills)
	$(INSTALL) pi --project

uninstall: ## Remove the global Claude Code install
	$(INSTALL) claude --uninstall

uninstall-all: ## Remove the install for every supported harness
	$(INSTALL) all --uninstall

check: ## Validate install.sh syntax and run a dry-run install for all harnesses
	bash -n install.sh
	$(INSTALL) all --dry-run

lint: check ## Alias for check
