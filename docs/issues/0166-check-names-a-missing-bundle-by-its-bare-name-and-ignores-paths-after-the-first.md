---
type: Issue
title: check names a missing bundle by its bare name and ignores paths after the first
description: living-docs check prints the bundle root as given rather than resolved, and check_bundle silently drops every path after the first.
status: open
timestamp: 2026-11-27T20:46:35Z
---

## 0166. check names a missing bundle by its bare name and ignores paths after the first

## Context

Found in the review of issue 0158 (2026-11-27). `living-docs --docs-dir <absolute file> check` prints `bundle root not found: fileroot`, the bare name, while `manage fmt` prints the absolute path. `check_bundle` (`cli/src/commands/check.rs` near line 238) takes only the first of several path arguments and ignores the rest without saying so.

## Decision

- None yet.

## Acceptance

- `check` names a missing bundle root the same way `manage fmt` does.
- `check` with more than one path either checks each or refuses the extra ones with a message; it never drops them silently.

## Scope

In: `cli/src/commands/check.rs` path handling and its message. Out: other verbs.
