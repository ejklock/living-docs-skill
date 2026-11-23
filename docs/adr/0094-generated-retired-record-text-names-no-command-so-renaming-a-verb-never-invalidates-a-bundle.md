---
type: ADR
title: Generated retired-record text names no command, so renaming a verb never invalidates a bundle
description: The Superseded and Deprecated callouts and the index history note drop the command name; one canonical form, and a bundle from an earlier release migrates with manage fmt and manage index.
owner: Evaldo Klock
status: Accepted
timestamp: 2026-11-20T16:30:40Z
---

# 0094. Generated retired-record text names no command, so renaming a verb never invalidates a bundle

## Context

The CLI writes two pieces of text into a bundle that `check` then compares byte for byte with what the installed binary would generate: the callout a Superseded or Deprecated record opens with, and the history note above the retired rows of a type index. Both ended with an instruction naming a CLI command. That command was `living-docs effective` in v0.17.0, `living-docs read` from v0.20.0 to v0.27.0 and `living-docs read records` from v0.28.0, when [ADR 0076](0076-the-cli-groups-its-operations-under-write-read-and-manage-as-nested-subcommands-with-check-kept-top-level.md) regrouped the verbs.

Each rename made every retired record and every index written by an earlier release fail `check`. On 2026-11-20, 26 of 43 bundles on the owner's machine failed, and in five of them (relent, klckit-qa, se-guard, dev-fleet-dev-box-roles, klckit-field-service) regenerating the ADR index changed only that one line. A CI job pinned to v0.25.2 and a workstation on v0.30.0 judged the same bundle differently. The tool's output was a function of its inputs and of its own version, which is the opposite of the determinism [ADR 0001](0001-living-docs-cli.md) promises.

## Decision

We will write no CLI command name into any text the CLI generates inside a bundle file. The callouts become `> **SUPERSEDED — do not act on this record.** Replaced by [NNNN](<destination>).` and `> **DEPRECATED — do not act on this record.** It has no successor.`, and the index note becomes `_History only. Do not act on these records._`.

`check` keeps comparing against the one canonical form. A bundle written by an earlier release fails until `living-docs manage fmt` and `living-docs manage index` run once; both already rewrite the callout and the note, and the finding already names them.

Rejected alternatives:

- **Verb-free text and `check` accepting the three earlier wordings.** No bundle would need a migration, but the gate would carry a list of historical strings forever and two bundles with different bytes would both be canonical. The owner chose one canonical form on 2026-11-20.
- **Keep the current wording and accept the earlier ones.** The next rename repeats the failure unless someone remembers to extend the list.
- **A semantic comparison that ignores the trailing sentence.** It hides a hand-edited callout from the gate that exists to catch it.

## Consequences

**Easier / gained:**
- Renaming or regrouping a verb no longer changes what a correct bundle looks like.
- CI and a workstation on different releases agree on a callout and an index note from this release on.

**Harder / accepted trade-offs:**
- Every bundle with a retired record or a retired index section fails `check` once after the upgrade, until `manage fmt` and `manage index` run. The release notes must say so.
- The callout no longer tells a reader which command lists what is in force; the session line and the skill carry that.

**Fitness function:** a test migrates a fixture bundle holding each earlier wording with `manage fmt` and `manage index`, asserts `check` is green and a second run changes no byte; a second test asserts that no callout or index note the CLI generates contains `living-docs`.
