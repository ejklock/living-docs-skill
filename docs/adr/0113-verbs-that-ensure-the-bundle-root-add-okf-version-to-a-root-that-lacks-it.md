---
type: ADR
title: Verbs that ensure the bundle root add okf_version to a root that lacks it
description: write new and manage index add okf_version frontmatter to an existing bundle-root index.md that has none, so older and hand-written bundles carry the living-docs marker.
owner: Evaldo Klock
status: Accepted
timestamp: 2026-11-27T02:30:18Z
---

# 0113. Verbs that ensure the bundle root add okf_version to a root that lacks it

## Context

`living-docs write new` and `living-docs manage index` create a missing bundle-root `index.md` with the frontmatter `okf_version: "0.1"`, from v0.24.0 (commit `e6a8c19`). A root that already exists is only appended to: `ensure_root_index` in `living-docs-core/src/commands/index/root.rs` goes to `append_unreached`, which adds link lines and never adds frontmatter, and `check` accepts a root without frontmatter. Bundles started before v0.24.0, or whose root was written by hand (ADR 0020 records that the root used to be hand-maintained), therefore carry no `okf_version` line, and no verb ever adds one.

Issue 0156 needs that line as the marker of a living-docs project: the Claude Code plugin's session hook (ADR 0108) fires in every project the user opens and must stay silent in a project whose `docs/` is MkDocs, Docusaurus or plain notes. Without a migration, every older living-docs project would stay silent for good.

## Decision

We will make every verb that ensures the bundle root, `write new` and `manage index`, add `okf_version: "0.1"` to a root `index.md` that lacks it, keeping the rest of the file as written.

- A root with no frontmatter gains a frontmatter block holding only `okf_version: "0.1"` above its first line.
- A root with frontmatter but no `okf_version` key gains the key as the block's first line; every other key and line stays.
- A root that already holds `okf_version` is left byte-for-byte as it is, whatever its value.
- The change is reported like any other file the verb writes, so `manage index` lists the root among its written files.
- `manage fmt` stays out: reserved files (`index.md`, `log.md`) are outside its scope by design.

Rejected alternatives:

- **A second marker in the hook only (a numbered record with `type:` frontmatter).** Changes no verb and works at once on old bundles; the owner chose one marker that the CLI guarantees.
- **`check` reports a root without `okf_version`.** Tells the author but leaves the repair to a hand edit, which the authoring contract forbids for CLI-owned structure.
- **Accept the silence and document the edit.** Leaves every older living-docs project without the session line.
- **`manage fmt` performs the migration too.** Brings reserved files into a verb that excludes them on purpose.

## Consequences

**Easier / gained:**
- One run of `manage index`, or the next `write new`, marks an older bundle; the plugin's session hook then recognizes it.
- Every bundle root converges on the same frontmatter.

**Harder / accepted trade-offs:**
- The first `manage index` or `write new` after upgrading rewrites the root of an older bundle, a diff the author did not ask for.
- A root whose first lines look like frontmatter but are not closed is treated as no frontmatter; the verb inserts a new block above it.

**Follow-ups:**
- Implemented as a slice of issue 0156.

## Verification

**Implementation impact:** `living-docs-core/src/commands/index/root.rs`, its tests, the `manage index` and `write new` CLI tests.

**Verification criteria:**
- `manage index` on a bundle whose root starts with `# Index` leaves `---`, `okf_version: "0.1"`, `---` above the unchanged original lines, and lists the root as written; a second run writes nothing.
- A root with frontmatter holding other keys keeps them and gains `okf_version` as the first key; a root already holding `okf_version: "9"` is unchanged.
- `check` passes on the migrated fixtures.
