---
type: Issue
title: status with a bare record number resolves across type directories, ADRs first
description: Accept a type qualifier on status and hard-error on an ambiguous bare record number, so a number shared across type directories can no longer mutate the wrong record.
status: closed
timestamp: 2026-07-30T18:58:44Z
---

## Problem

`living-docs status <N> <Status>` resolves a bare record number against every type directory, ADRs first. When a number exists in more than one type (adr/0016 and issues/0016), the verb mutates the ADR even when the user means the issue — there is no type disambiguator in the invocation.

## Repro (v0.8.0)

In a bundle containing both `adr/0016-*.md` and `issues/0016-*.md`: `living-docs status 16 Deprecated` → rewrites the ADR's `status:` frontmatter. Observed live in a downstream repo (ai-configs, 2026-07-30); the accidental ADR status change had to be reverted from HEAD.

## Workaround

Scope the search with the honored flag: `living-docs --docs-dir docs/issues status 16 Deprecated`.

## Proposal

Accept a type qualifier — `living-docs status issue 16 Deprecated` or `--type issue` — and hard-error on an ambiguous bare number instead of picking the first directory scanned.
