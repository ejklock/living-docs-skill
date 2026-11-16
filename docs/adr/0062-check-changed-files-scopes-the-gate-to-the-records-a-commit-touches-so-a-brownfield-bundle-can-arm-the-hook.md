---
type: ADR
title: check --changed-files scopes the gate to the records a commit touches, so a brownfield bundle can arm the hook
description: check gains a --changed-files mode that reports only the findings anchored to the records a commit touches, so a brownfield bundle can arm the pre-commit channel while it pays legacy debt down
owner: Evaldo Klock
status: Accepted
timestamp: 2026-09-22T13:03:44Z
---

# 0062. check --changed-files scopes the gate to the records a commit touches, so a brownfield bundle can arm the hook

## Context

The pre-commit channel execs a full-bundle `check`. In a bundle born clean that is exactly right: every commit proves the whole corpus. In a bundle that adopts living-docs with history behind it, one legacy violation fails every future commit whatever it touches, so the gate cannot be armed at all until the entire corpus is clean. [Issue 0016](../issues/0016-check-needs-a-ratchet-or-changed-files-mode-so-brownfield-repos-can-arm-the-pre-commit-channel.md) records the adoption attempt that hit this: 180 violations across 177 docs, `fmt` unable to reduce the residue, and the channel shipped disarmed — artifacts committed, `core.hooksPath` left unset.

An all-or-nothing gate in that state teaches the wrong lesson. The author cannot pay the debt down incrementally, because no intermediate state is rewarded; the only two moves are a corpus-wide cleanup nobody has budget for, or leaving the gate off, which is what happened.

Two shapes answer it. A ratchet compares against a baseline — either a git ref or a stored manifest of accepted debt — and fails only on findings absent from it. A changed-files mode scopes the gate to what the commit touched. Both let a bundle arm the hook while dirty; they differ in what they must remember. A baseline is state: a ref to resolve or a manifest file to keep honest, shrinkable but never growable, one more artifact that can drift from the corpus it describes. Changed files are already in the author's hand — git names them at the moment of the commit — and need no storage at all.

## Decision

We will add `check --changed-files <path>...`, which runs every invariant over the whole bundle exactly as today and then reports only the findings anchored to a listed file. The gate's verdict follows the filtered violations, so a commit that touches a clean record passes in a bundle that is still dirty elsewhere. The installed pre-commit hook opts in through `LIVING_DOCS_CHECK_CHANGED_ONLY=1`, passing its staged records; unset, the hook keeps its full-bundle behavior, so a clean bundle loses nothing.

The mode's guarantee is stated in the flag's own help and is deliberately narrow: it proves the records this commit touched, not the bundle. A finding anchored in another record — the classic case is a rename that orphans someone else's link — is invisible to it. That is the honest cost of any ratchet, and it is why CI keeps running the full `check` on every push: the scoped mode is an adoption ramp for the local hook, never a replacement for the gate.

Filtering the report after the full run, rather than scoping the walk itself, is the deliberate choice: every invariant that reads sibling records — the supersede chain, reachability, the moved-source rule — keeps its whole-bundle view and cannot produce a different finding because of the flag. The mode changes what is reported, never what is computed.

Rejected: `--baseline <ref>`, the diff-aware ratchet. It is the more precise instrument — it catches a new finding anchored in an untouched file, which changed-files misses — but it must resolve and check out a second tree, which makes the local hook slow and ties the gate to git's object store, a dependency `check` does not otherwise have.

Rejected: an accepted-debt manifest at `.living-docs/check-baseline`. It answers the same need with a file that must be kept honest forever: a finding's identity has to survive line moves and rewordings, and a manifest that silently stops matching is a gate that silently stops gating. If a project later needs debt tracked rather than scoped, that is a record of its own.

## Consequences

**Easier / gained:**
- A brownfield bundle can arm the pre-commit channel on day one and pay the legacy debt down record by record.
- Nothing new is stored, so there is no baseline artifact to drift from the corpus.
- A clean bundle is unaffected: without the flag and without the env var, the gate is what it was.

**Harder / accepted trade-offs:**
- A finding anchored outside the commit's files is not reported locally; CI is what catches it.
- Every invariant still runs, so the scoped mode costs the same time as the full one. It buys adoption, not speed.
- The hook grows an env-var branch — one more path to keep tested.

**Follow-ups:**
- None. A debt manifest, if a project ever needs one, is a new decision, not a deferred part of this one.

## Verification

**Implementation impact:** `living-docs-core/src/check/mod.rs` (the scoped report), `cli/src/args.rs` and `cli/src/commands/check.rs` (the flag), and `.githooks/pre-commit` (the opt-in).

**Verification criteria:**
- `check --changed-files <clean record>` exits 0 in a bundle whose violations all sit in other records.
- `check --changed-files <dirty record>` exits 1 and reports that record's violations only.
- With no flag, the report is byte-identical to today's.
- The hook runs the scoped form only when `LIVING_DOCS_CHECK_CHANGED_ONLY=1`, proven by a test over the hook script.

# References

[1] [Issue 0016 — check needs a ratchet or changed-files mode](../issues/0016-check-needs-a-ratchet-or-changed-files-mode-so-brownfield-repos-can-arm-the-pre-commit-channel.md)
