---
type: ADR
title: check advises on an inline-code repository path that no longer exists
description: A path-missing advisory for a live record naming a repository-relative path that does not exist, by an existence-only probe rooted at the repository; amends the rejection in ADR 0075.
owner: Evaldo Klock
status: Accepted
timestamp: 2026-11-20T16:30:40Z
---

# 0096. check advises on an inline-code repository path that no longer exists

## Context

A record often names the code it governs by path, in inline code. When that file is renamed or deleted nothing tells the author, and the record keeps describing a place that no longer exists. No `check` rule reads anything outside the bundle today; the `moved-source` advisory compares a record with another record.

[ADR 0075](0075-doc-to-code-drift-is-an-advisory-skill-whose-llm-judge-runs-outside-the-binary-through-a-configurable-command.md) put doc-to-code drift in a skill whose judge is a language model outside the binary, and rejected "a deterministic anchor advisory in check now" because a probe found no true positive at the time. The skill has never run with a real judge on any bundle, and its deterministic half (extract the inline-code paths, test that each exists) needs no model. The owner asked on 2026-11-20 for the deterministic half inside the gate.

The filesystem store is rooted at the bundle ([ADR 0073](0073-the-filesystem-store-is-rooted-at-the-bundle-and-refuses-any-read-write-or-rename-that-resolves-outside-it.md)); a code path lives outside it.

## Decision

We will have `check` report a `path-missing` advisory for a live record that names, in inline code outside a fenced block, a repository-relative path that does not exist.

- A token is a path when it holds only `A-Z a-z 0-9 _ . / @ + ~ -`, contains a `/`, and its first segment exists as an entry of the repository root. A trailing `:<line>` is ignored. The first-segment rule drops record references such as `adr/0001` and word pairs such as `write/read`.
- The repository root is the nearest ancestor of the bundle that holds a `.git` entry. With none, the rule is skipped.
- The probe answers existence only. It never reads a file's contents, and it refuses an absolute path, a `..` segment and a path whose resolution leaves the repository root.
- Retired records are not examined.

This amends the rejection in ADR 0075 and nothing else in it: the judge, the skill and the rule that no model runs inside the binary stand.

Rejected alternatives:

- **Keep it in the skill only.** The skill is never run, so the drift is never reported.
- **A `covers:` frontmatter key with globs, as a violation.** It is the declared lineage of the Deprecated [ADR 0032](0032-doc-code-lineage-is-declared-not-inferred.md): a new key on every record of every bundle, and a gate that fails on a rename.
- **Symbol presence (`path › symbol`).** A word search in a file reads contents and cannot tell a renamed symbol from a removed one; it stays in the skill.

## Consequences

**Easier / gained:**
- A rename or a delete that orphans a record's reference shows up in the next `check`.
- The rule is reproducible from the working tree alone.

**Harder / accepted trade-offs:**
- A record that mentions a deleted file as history is advised until it is closed or reworded.
- `check` gains one read outside the bundle, limited to existence.
- A path written without inline code, or below a directory that was itself removed from the repository root, is not seen.

**Fitness function:** tests assert the advisory for a missing path under an existing top-level directory, and no advisory for an existing path, a record reference, a path in a fenced block, a retired record, an absolute path and a `..` path.
