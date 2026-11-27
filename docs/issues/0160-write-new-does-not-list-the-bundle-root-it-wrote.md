---
type: Issue
title: write new does not list the bundle root it wrote
description: write new creates or migrates the bundle-root index.md without listing it among the files it wrote.
status: open
timestamp: 2026-11-27T03:33:40Z
---

## 0160. write new does not list the bundle root it wrote

## Context

`living-docs write new` writes the bundle-root `index.md` when it creates or migrates it (`ensure_root_index`), but its output lists only the type index: `living-docs-core/src/commands/new.rs` near line 81 keeps the error of `ensure_root_index` and drops the written path. `manage index` lists the root. Found in review of issue 0156, slice S9; a caller of `write new` cannot tell that the root changed.

## Decision

- None yet.

## Acceptance

- `write new` lists the bundle-root `index.md` among the files it wrote whenever it creates or changes it, in text and JSON, and over MCP alike.

## Scope

In: `living-docs-core/src/commands/new.rs` and the output of `write new`. Out: other verbs.
