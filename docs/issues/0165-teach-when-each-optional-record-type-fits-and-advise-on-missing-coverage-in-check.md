---
type: Issue
title: Teach when each optional record type fits and advise on missing coverage in check
description: The session line and MCP instructions say when a view, domain record, research note or constitution fits, write new adr suggests the missing neighbour, and check gains non-blocking coverage advisories.
status: open
timestamp: 2026-11-27T19:32:31Z
---

## 0165. Teach when each optional record type fits and advise on missing coverage in check

## Context

The owner observed on 2026-11-27 that the authoring model uses the optional record types (architecture views, domain records, research notes, the constitution, a PRD) only when told to. The session line and the MCP instructions teach ADRs, issues and the Decision section, but not when the other types fit; this is an inference about the cause. Scaffolding optional records when a bundle is created was considered and rejected: an unfilled slot fails `check`, and the ADR conventions' materiality rule forbids manufacturing a record per layer.

## Decision

- Teach at the point of use and add coverage advisories (owner, 2026-11-27): the session line and the MCP discovery instructions name the trigger for each type (an architecture change: a view; a domain term or business rule: a domain record; external evidence behind a choice: a research note; scope and non-negotiables: the constitution); `write new adr` suggests a neighbour record that is missing; `check` gains non-blocking advisories for missing coverage. Not taken: teaching only, without advisories; scaffolding optional records when the bundle is created.
- The new advisory kinds change `check`'s output contract, so an ADR records them before code.

## Acceptance

- An ADR names each new coverage advisory kind, its trigger and why it never fails the gate.
- The session line and the MCP instructions carry the per-type triggers; their tests pin the wording.
- `check` reports each coverage advisory on a fixture that lacks the coverage and stays silent on one that has it; the exit code never changes.

## Scope

In: the session hook line, `mcp/src/instructions.rs`, `write new adr`'s ask, `check` advisories, an ADR. Out: creating records automatically.
