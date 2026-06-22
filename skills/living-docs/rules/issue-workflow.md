# Issue Workflow (docs-first)

Issues are authored **in the repo first**, then published to the tracker. The repo file is the source of truth; the tracker entry is a mirror that must stay byte-identical in body. Each issue is an **OKF concept** (`type: Issue`) — see the `okf-knowledge-format` skill.

## The workflow

1. **Create** — write `docs/issues/NNNN-slug.md`: OKF frontmatter (`type`, `title`, `status`, `labels`, `blocked_by`, `tracker`) followed by the issue **body**. Add a row to `docs/issues/index.md`. *Then* publish to the tracker, stripping the frontmatter so only the body is sent (e.g. pipe the content below the closing `---` to `gh issue create --body-file -`).
2. **Edit** — update the body file first, then push to the tracker (`gh issue edit <n> --body-file …`, frontmatter stripped).
3. **Identical bodies** — the repo file's body (everything below the closing `---`) and the tracker issue body must match exactly. If they diverge, the repo wins; re-sync. Frontmatter is repo-only.
4. **Metadata lives in frontmatter** — number, title, labels, blocked-by, status, and tracker number live in the issue file's **frontmatter** (and are mirrored into the `docs/issues/index.md` listing), *not* in the body. The body is portable prose; the frontmatter carries the tracker-specific fields.
5. **Prefer editing the body over adding comments** — so each issue keeps a single, coherent source of truth rather than a scattered comment thread.

See `templates/issue.md` for the body skeleton.

## Body structure

A good issue body states, in order:

- **What / Why** — the change and its motivation. If it implements a PRD or ADR, link it ("Implements ADR NNNN").
- **Scope** — what's included; for removals/refactors, what's explicitly kept.
- **Acceptance** — observable, testable conditions for "done".
- **Plan** — a short outline (and slicing, for large tasks) so a reader knows the approach.

## Rules

1. **Number consistently** with the project's scheme; index every issue.
2. **Link bidirectionally** — body links to its PRD/ADR; the PRD/ADR's decision log or the issue index links back.
3. **Closing keywords** belong in the PR that resolves the issue (`Closes #NN`), not scattered in the body.
4. **Historical issue bodies are not rewritten** when superseded — set the frontmatter `status` (e.g. `superseded`) and annotate the `index.md` row, leaving the body as closed history.

## Why docs-first

A tracker is an external system that can change auth, API, or vendor. The repo is durable, diffable, and reviewable in the same PR as the code. Authoring in the repo means the issue is versioned alongside the work it describes, and the body survives any tracker migration.
