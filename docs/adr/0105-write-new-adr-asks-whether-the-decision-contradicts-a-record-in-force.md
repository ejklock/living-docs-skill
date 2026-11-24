---
type: ADR
title: Write new adr asks whether the decision contradicts a record in force
description: write new adr prints a fixed question with the count of records in force and how to list them, in text, JSON and MCP, without naming any record.
owner: Evaldo Klock
status: Proposed
timestamp: 2026-11-24T03:13:12Z
---

# 0105. Write new adr asks whether the decision contradicts a record in force

## Context

Issue 0153 measured how often a session catches an in-force decision that a new ADR contradicts or partly reverses. Every miss was a session that was not asked to look and whose conflicting record used other words. The transcripts show those sessions did search: `read records --topic` returned nothing on the paraphrase and the session read the empty answer as no conflict. Sessions asked to list conflicts caught every plant. The binary cannot judge which decision governs ([ADR 0001](0001-living-docs-cli.md)), so it cannot name the conflict itself; it can ask the question at the moment the decision is written.

## Decision

We will make `write new adr` ask, every time it scaffolds an ADR, whether the new decision contradicts or partly reverses a record in force, and say how many records are in force and how to list them.

- Text output adds one fixed line on stderr after the path, the same for every ADR: the count of records in force, the question, and the command `living-docs read records` that lists them. The JSON output adds `in_force` (a number) and `ask` (the same fixed text); `path`, `instruction` and `indexed` are unchanged. The MCP `write` tool returns the same fields ([ADR 0078](0078-an-mcp-front-serves-the-bundle-over-stdio-as-write-read-manage-and-check-tools-from-the-living-docs-mcp-subcommand.md)).
- The line never names a record, never says a conflict exists or not, and never changes the exit code. Only the ADR type asks; other types are unchanged.
- The text of `ask` is a contract an agent may read; changing its meaning needs a later ADR, its wording may be refined.

This is variant V2 of issue 0153. It ships only if the remeasurement under the rule fixed in that issue keeps it; otherwise a later ADR removes it.

## Consequences

- Every `write new adr` prints one more line; a script that reads the JSON keeps working, since the change is additive.
- The question costs no search and no noise: it is the same for every write, so it cannot be wrong about a specific record.
- An agent that ignores the line gains nothing; the remeasurement shows whether agents act on it.
- Candidate records (variant V3) stay unbuilt unless V1 and V2 fall short of the rule.
