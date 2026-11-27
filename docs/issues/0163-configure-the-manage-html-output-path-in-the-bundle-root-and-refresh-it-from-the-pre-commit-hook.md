---
type: Issue
title: Configure the manage html output path in the bundle root and refresh it from the pre-commit hook
description: manage html reads its output path from the bundle-root index frontmatter and the generated pre-commit hook regenerates the page, as requested by the se-hub session; awaiting the owner's confirmation.
status: open
timestamp: 2026-11-27T19:27:33Z
---

## 0163. Configure the manage html output path in the bundle root and refresh it from the pre-commit hook

## Context

`living-docs manage html` writes `.living-docs/html/index.html` unless `--out` names another path (ADR 0090, "Output"). The se-hub project wants the page inside its bundle (`docs/html/index.html`) and regenerated without passing `--out` every time; `living-docs check` already accepts that file inside the bundle (reported by the se-hub session, 25 docs green). The request reached this session on 2026-11-27 from the se-hub session, which reports these owner decisions; they are not yet confirmed in this repository:

1. The output path is per-bundle config in the bundle-root `index.md` frontmatter, beside `okf_version` (for example `html_out: html/index.html`, relative to the bundle root). No new config file, no environment variable.
2. `manage html` without `--out` uses that key when present; `--out` still overrides; without the key the default stays `.living-docs/html/index.html`.
3. The pre-commit hook written by `manage install hooks` regenerates the page when the key is present; output is deterministic (ADR 0090), so an unchanged bundle writes no byte.
4. No `--watch`: a watcher reopens the live-reload alternative ADR 0090 rejected (ADR 0059's long-running process).

## Decision

- Config lives in a `living-docs.json` file at the bundle root (`docs/living-docs.json`), with a published JSON Schema, not in the root `index.md` frontmatter (owner, 2026-11-27); the root index keeps only the `okf_version` marker, and `check` ignores the file because it is not a record. This replaces decision 1 relayed by the se-hub session. Not taken: a `living-docs.json` at the repository root (ambiguous with more than one bundle); the frontmatter key.
- The default output moves inside the bundle, to `docs/html/index.html` (owner, 2026-11-27), replacing ADR 0090's `.living-docs/html/index.html`; this needs a superseding or amending ADR. Not taken: keeping the default outside the bundle. Open with it: whether that file is ignored by the project's `.gitignore` or committed by the pre-commit hook.
- With issue 0164's one-file-per-type output, the configured and default output is a directory (`docs/html/`), not a file (owner, 2026-11-27).
- Open: whether the hook stages the regenerated file when its path is tracked (in se-hub it is gitignored); how the key is validated (contained relative path, no `..`, no absolute path); whether this amends or supersedes ADR 0090's "Output" bullet.

## Acceptance

- The owner confirms decisions 1-4 in this repository and settles the open points; an ADR records the config key and the hook behavior before code.
- `manage html` with no `--out` writes to the path the root `index.md` frontmatter names, refusing a path that escapes the bundle; `--out` overrides it; without the key the default is unchanged.
- The installed pre-commit hook regenerates the page when the key is present and leaves the tree byte-identical when the bundle did not change.

## Scope

In: the frontmatter key, `manage html` output resolution, the generated pre-commit hook, an ADR amending ADR 0090. Out: a watcher or server.
