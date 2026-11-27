---
type: Issue
title: manage fmt reports success on a bundle root that is a regular file
description: manage fmt on a docs path that is a regular file returns an empty success, while check refuses the same path as bundle root not found.
status: open
timestamp: 2026-11-27T02:53:42Z
---

## 0158. manage fmt reports success on a bundle root that is a regular file

## Context

`living-docs --docs-dir <file> --json manage fmt` prints `{"files":[]}` and exits 0 when the docs path is a regular file, and the MCP `manage` tool with operation `fmt` answers the same. `check` on the same path refuses with `bundle root not found`. The fmt path resolves the bundle in `living-docs-core/src/commands/fmt.rs` near line 48. Found in review of issue 0156, slice S4; harmless, but a caller learns nothing about the wrong path.

## Decision

- `manage fmt` refuses a docs path that is not a directory with the same `bundle root not found` text `check` gives; the MCP `manage` tool inherits it through core (session, 2026-11-27). Not taken: an empty success.

## Acceptance

- `manage fmt` on a docs path that is a regular file refuses with a message naming the path, in the CLI and over MCP alike.

## Scope

In: `living-docs-core/src/commands/fmt.rs` and its tests. Out: other verbs.
