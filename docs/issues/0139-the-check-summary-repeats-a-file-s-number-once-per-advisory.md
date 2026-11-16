---
type: Issue
title: The check summary repeats a file's number once per advisory
description: The plain advisory summary lists the same record number once per advisory instead of once per file
status: open
timestamp: 2026-11-16T04:07:59Z
---

## 0139. The check summary repeats a file's number once per advisory

### Scope

The plain `check` summary lists an advisory kind's files by number once per advisory instead of once per file: an issue with three non-numeric `blocked_by` items and a cycle gives "4 BLOCKED-BY advisories: 0005, 0005, 0005, 0001". The summary renderer already repeated numbers for several advisories on one file; the blocked-by advisory makes it frequent (agent-memory carries 143). Found by the review of issue 0138 slice S1; it predates it.

### Decision

- List each file once in the summary line, keeping the advisory count.

### Acceptance

- The same fixture prints "4 BLOCKED-BY advisories: 0001, 0005" (or the file order the renderer uses), and a test proves it; `--advisories` output is unchanged.
