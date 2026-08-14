---
type: ADR
title: migrate --apply is a CLI-front transaction over the mechanical subset
description: "Adds --apply to migrate as an fs-only, all-or-nothing applier in the CLI front: snapshot bundle + seal ledger, run index and fmt, roll back byte-for-byte on any failure or check regression; the core advisor stays read-only, AUTHOR steps are never applied, ADOPT plans refuse apply."
status: Proposed
timestamp: 2026-08-14T08:40:33Z
---

# 0040. migrate --apply is a CLI-front transaction over the mechanical subset

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly Proposed | Accepted | Deprecated. When superseding a prior ADR, set
     `supersedes` here; `living-docs supersede` sets Superseded on the old record
     -- never set it by hand. -->

## Context

ADR 0037 shipped `migrate` as a read-only advisor and named its first sacrifice explicitly: no `--apply`, because an applying verb needs its own transactional story before it earns existence. That follow-up is now called: agents reliably execute the printed `RUN` steps, but each execution is an unguarded sequence — a crash or a regression between `index` and `fmt` leaves the bundle half-adapted with nothing to restore it.

Epistemic type: **Judgment**. External critic: **pending** (real migrations).

## Decision

We will add `living-docs migrate --apply` as a **CLI-front transaction** over the mechanical subset, keeping the core advisor read-only (ADR 0037's determinism split survives intact). fs-backend only. The applier: (1) counts `check` violations before; (2) snapshots every `.md` under the bundle plus the provenance ledger (`.git/living-docs/seals.json`, ADR 0039) byte-for-byte; (3) runs `index` (bare sweep) then `fmt`; (4) counts violations after. Any step failure, or an after-count above the before-count, restores the snapshot — including deleting files the apply created — and exits nonzero naming the regression. `AUTHOR` steps are never applied; an `ADOPT` plan (no bundle) refuses `--apply` outright: bootstrap is judgment plus user confirmation.

## Consequences

**Easier / gained:**
- The mechanical subset becomes one guarded command: all-or-nothing, check-gated, seal-consistent (the ledger is part of the snapshot, so a rollback never leaves stale seals).
- The core `migrate` module stays pure — the transaction lives in the front, mirroring how `fmt`/`index` are already fronted.

**Harder / accepted trade-offs (the declared sacrifice):**
- The regression guard is count-based: an apply that fixes one violation while introducing another passes. Steelman of the rejected alternative — set-difference over violation identities; rejected for this slice because message texts carry paths that legitimately change (regenerated indexes), making identity matching brittle; the count guard plus the pre-commit gate covers the realistic failure.
- `--apply` executes `index` and `fmt` wholesale rather than only the steps the plan named — idempotent verbs make the difference unobservable, but the plan text and the applied set can diverge in the log.
- db-mode is refused rather than supported: its `write_checked` path is already transactional per-record and has no bundle-wide snapshot story.

**Follow-ups:**
- ADR 0037's first sacrifice bullet is annotated with a pointer here (partial amendment, rule 3).

## Verification

**Implementation impact:** `cli/src/commands/migrate.rs` (applier + snapshot/restore), `cli/src/args.rs` (`--apply`), `skills/living-docs/rules/migration.md`.

**Verification criteria:**
- A legacy table index is regenerated in place by `--apply` and `check` stays green; the exit is 0 and the remaining `AUTHOR` steps are re-printed.
- A failing apply restores the bundle byte-for-byte, including deleting created files and restoring the seals ledger.
- `--apply` on a missing bundle or `--backend db` is refused with exit 2.
- Fitness function: snapshot/restore unit tests pin content restore + created-file deletion; an integration test pins the table-index apply.

# References

[1] [ADR 0037](/adr/0037-migration-is-a-deterministic-advisor-verb-plus-skill-guided-judgment.md)
[2] [ADR 0039](/adr/0039-cli-produced-records-carry-an-ephemeral-hmac-provenance-seal-that-check-verifies.md)

