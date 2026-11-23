#!/usr/bin/env bash
#
# runner.sh — the isolated runner for the scale scenarios, for LIVING_DOCS_EVAL_RUNNER. The model
# sees only the scratch repository's own settings: no user settings, plugins or MCP servers, no
# session kept, and Bash limited to the CLI under test and plain file tools.
#
# environment: LIVING_DOCS_EVAL_MODEL  the model to run (default: claude-sonnet-5-5)
#
# The session hook names the CLI by its absolute path, so the model often calls it that way; the
# absolute path of the living-docs first on PATH is allowed next to the bare name.

CLI_PATH="$(command -v living-docs || echo living-docs)"

exec claude -p \
	--model "${LIVING_DOCS_EVAL_MODEL:-claude-sonnet-5-5}" \
	--output-format stream-json --verbose \
	--setting-sources project \
	--strict-mcp-config \
	--no-session-persistence \
	--permission-mode acceptEdits \
	--allowedTools "Bash(living-docs:*)" "Bash($CLI_PATH:*)" "Bash(git status:*)" "Bash(git diff:*)" "Bash(git log:*)" "Bash(git mv:*)" "Bash(ls:*)" "Bash(cat:*)" "Bash(grep:*)" "Bash(rg:*)" "Bash(find:*)" "Bash(head:*)" "Bash(sed:*)" "Bash(mv:*)" "Bash(mkdir:*)" Read Edit Write Glob Grep \
	--disallowedTools Agent WebFetch WebSearch
