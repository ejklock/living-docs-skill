---
type: ADR
title: CLI-produced records carry an ephemeral HMAC provenance seal that check verifies
description: "An fs-mode write decorator seals every CLI-written record's CLI-owned frontmatter into a ledger under .git/living-docs/ keyed by a per-clone secret; check verifies seals when the key exists (fail-open otherwise), so records created or owned-key-edited outside the CLI fail the gate with a re-issue instruction. Friction, not cryptography: with shell access no local secret is unforgeable, and the ADR says so."
status: Proposed
timestamp: 2026-08-14T08:21:07Z
---

# 0039. CLI-produced records carry an ephemeral HMAC provenance seal that check verifies

## Context

The docs-handwrite hook (ADR 0019/0021) intercepts the Write/Edit tools — but an agent
editing through the shell (`sed`, `cat >`, heredocs) bypasses it entirely, and agents are
observed doing exactly that. The gate that cannot be bypassed is `check`, because it
validates the *result*, not the write path. What `check` cannot see today is provenance:
a hand-fabricated record with plausible frontmatter is indistinguishable from a
CLI-produced one.

Honesty about the threat model: with shell access, no local secret is unforgeable — an
agent could read any key and recompute an HMAC. An LLM is never a security boundary; the
goal is **asymmetric friction** (the CLI path must be the cheapest path, and fraud must
require deliberate multi-step effort no agent drifts into), not cryptography.

Epistemic type: **Judgment**. External critic: **pending** (real sessions).

## Decision

We will seal provenance per clone, ephemerally: `living-docs seal init` generates a
random key and a ledger under `.git/living-docs/` (never committed, dies with the
clone) and seals the current bundle as the trusted baseline. Every fs-mode CLI write
then re-seals automatically — a store decorator computes HMAC-SHA256(key, record path +
CLI-owned frontmatter lines: `type,title,status,supersedes,superseded_by,timestamp`) and
updates the ledger, so `new`/`brief`/`status`/`supersede`/`fmt`/`describe` need no
per-verb wiring. `check` gains a seal rule: when key and ledger exist, every CLI-owned
record must have a matching seal — a missing or mismatched seal is a violation naming
the re-issue path; when the key is absent (fresh clone, feature unused) the rule is
silent (fail-open, like the hook). Body prose, `description`, `tags`, and `kind` stay
freely editable — they are not sealed.

## Consequences

**Easier / gained:**
- Shell-path hand-writes are finally caught: the pre-commit doc-gate fails with an actionable message, whatever tool made the edit.
- Zero per-verb wiring (single choke point: the fs store's `write`), zero committed artifacts, zero cross-machine key coordination.
- Legitimate body authoring is untouched — the seal covers exactly the keys the hook already declares CLI-owned.

**Harder / accepted trade-offs (the declared sacrifice):**
- Not tamper-*proof*, tamper-*evident with effort*: a determined agent can read the key
  and forge a seal via `openssl`. Steelman of the rejected alternative — a truly
  unforgeable signature needs a secret outside the agent's reach (OS keychain, remote
  signer); rejected as disproportionate machinery for an incentive problem, and
  restated: an LLM is never a security boundary.
- Git operations that legitimately rewrite records (merge, checkout, pull) invalidate
  seals; the remedy is re-baselining (`living-docs seal init`) and the violation message
  says so. A baseline seals whatever is present — it asserts "trusted from here on",
  not "CLI-born", which is also what makes adoption and fresh containers workable.
- A frontmatter-embedded signature was rejected: it would sign content that includes
  itself (circular), pollute the portable record, and travel to clones whose key cannot
  verify it.

**Follow-ups:**
- Session bootstrap (SessionStart hook / `hooks install` docs) may run `seal init` so
  fresh containers are sealed from the first turn.

## Verification

**Implementation impact:** `living-docs-core/src/seal.rs` (new),
`living-docs-core/src/check/seal.rs` (new), `cli/src/store.rs` decorator,
`cli/src/{args,main}.rs` (`seal init`), deps `hmac`/`sha2`.

**Verification criteria:**
- After `seal init`: a shell-created record or a hand-edited `status:` fails `check`
  naming the seal rule; a body-only edit passes; with no key present the rule is silent.
- A CLI `status`/`supersede`/`fmt` write re-seals so `check` stays green.
- Fitness function: `check::seal` unit tests pin all four outcomes; a parity test pins
  the sealed key list to the hook's `CLI_OWNED_KEYS`.

# References

[1] [ADR 0019](/adr/0019-hand-written-record-frontmatter-is-blocked-at-write-time-detected-by-check-and-taught-at-point-of-use.md)
[2] [ADR 0021](/adr/0021-enforcement-layers-ship-with-the-repo-write-gate-hook-session-teaching-and-pre-commit-doc-gate.md)
