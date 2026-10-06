# living-docs check & quality checks

## living-docs check — the deterministic instrument

`living-docs check [docs/]` mechanically validates the invariants a machine checks better than
prose: frontmatter/`type`, directory-index membership + root reachability, link resolution,
supersede integrity, Mermaid fences, **unfilled `{{PLACEHOLDER}}` slots** (a violation — a
scaffold left half-authored fails the gate), and **required sections** (ADR 0064): a live record
missing a section its type requires fails with `missing required section '<name>'`. Retired
records are exempt. *A constraint without an instrument is a vibe* — so
the checkable invariants get a checker. Wire it into the project's quality gate / CI; a docs PR
that fails it does not merge. It does **not** check docs-first mirroring or "one home per fact"
semantics — those have no sound oracle and stay with the reviewer.

It also emits **advisories** — findings that never move the exit code, surfaced as work to
schedule:

- `SIZE` — a decision/execution body past the ~120-line target (aim ~100; research exempt).
- `MOVED-SOURCE` — a record whose linked source path moved.
- `LIVENESS stale-proposed` (ADR 0049) — an ADR still at its seed status (`Proposed`) whose
  linked issue is already `closed`/`done`/`Superseded`: the work landed but the record never
  left its birth state. Currency has a sound oracle where materiality does not, so the checkable
  part lives in the tool; the exit code stays put.

```bash
living-docs check docs   # check the project's bundle; exit 1 on any violation
```

It is a native Rust binary (correct without shelling out to a hand-rolled markdown/YAML
parser): `serde_yaml` for frontmatter, `pulldown-cmark` for link extraction and resolution
(every link form — inline, titled, angle-bracket, reference-style, images), and a native
directory-index/reachability BFS plus supersede-chain walk for the OKF structural graph.
No host tools to install — install the binary itself via `./install.sh` or
`make cli-install`. `living-docs check --mermaid-only` validates Mermaid fences in-process
via the pure-Rust merman-core parser (ADR 0013) — no Docker, no host tools.

A worked, lint-clean corpus lives in [`examples/linkly/`](../../examples/linkly/) — copy its shapes.

## Quality checks

Before considering a docs change complete. The frontmatter, indexing, link-resolution,
supersede, and placeholder items are enforced by `living-docs check` — run it rather than
eyeballing them; the rest are judgement:

- [ ] Every concept doc opens with OKF frontmatter carrying a non-empty `type`; `status` is in frontmatter, not a body line.
- [ ] Directory listings are `index.md` with no frontmatter (except the bundle-root `docs/index.md` → `okf_version`); cross-links are bundle-relative (`/…`).
- [ ] Every new doc is linked from its directory `index.md` **and** the bundle-root `docs/index.md`.
- [ ] No concept appears in two files (cross-reference instead).
- [ ] No unfilled `{{PLACEHOLDER}}` slot remains in any record.
- [ ] Every live record carries its type's required sections as headings (issue: Scope, Acceptance; ADR: Context, Decision, Consequences; PRD: Problem / Motivation, Non-goals; constitution: Product, Scope Boundaries, Non-negotiables).
- [ ] Superseded ADRs/PRDs carry frontmatter `status: Superseded` + `superseded_by: NNNN`; the superseding record sets `supersedes` and links back.
- [ ] A Superseded or Deprecated record's body opens with its exact CLI-written callout above the heading, and no active record opens with one; `living-docs fmt` is the remediation.
- [ ] Any structural code change in the same task updated its doc, including its Mermaid diagram(s).
- [ ] Architecture diagrams use Mermaid (in-repo text) and match the code.
- [ ] The constitution is singular (`docs/constitution.md`) — no NNNN prefix, no index entry.
- [ ] Each index file's links all resolve (no dangling references).
- [ ] Docs-first respected: the repo body matches the published tracker/wiki copy.
