# living-docs check & quality checks

## living-docs check — the deterministic instrument

`living-docs check [docs/]` mechanically validates invariants 2, 3, and 4 (the ones a
machine checks better than prose): frontmatter/`type`, directory-index membership + root
reachability, link resolution, and supersede integrity. *A constraint without an instrument is a
vibe* — so the checkable invariants get a checker. Wire it into the project's quality gate / CI;
a docs PR that fails it does not merge. It does **not** check docs-first mirroring or "one home
per fact" semantics — those have no sound oracle and stay with the reviewer.

```bash
living-docs check docs          # check the project's bundle; exit 1 on any violation
```

It is a native Rust binary (correct without shelling out to a hand-rolled markdown/YAML
parser): `serde_yaml` for frontmatter, `pulldown-cmark` for link extraction and resolution
(every link form — inline, titled, angle-bracket, reference-style, images), and a native
directory-index/reachability BFS plus supersede-chain walk for the OKF structural graph.
No host tools to install — install the binary itself via `./install.sh cli` or
`make cli-install`. `living-docs check --mermaid-only` validates Mermaid fences in-process
via the pure-Rust merman-core parser (ADR 0013) — no Docker, no host tools.

A worked, lint-clean corpus lives in [`examples/linkly/`](../../examples/linkly/) — copy its shapes.

## Quality checks

Before considering a docs change complete. The frontmatter, indexing, link-resolution, and
supersede items are enforced by `living-docs check` — run it rather than eyeballing them; the
rest are judgement:

- [ ] Every concept doc opens with OKF frontmatter carrying a non-empty `type`; `status` is in frontmatter, not a body line.
- [ ] Directory listings are `index.md` with no frontmatter (except the bundle-root `docs/index.md` → `okf_version`); cross-links are bundle-relative (`/…`).
- [ ] Every new doc is linked from its directory `index.md` **and** the bundle-root `docs/index.md`.
- [ ] No concept appears in two files (cross-reference instead).
- [ ] Every acronym the docs use has a glossary entry with its expansion **and** a definition in the doc language; the headword is the acronym as-is. Term names, identifiers, and acronym headwords/expansions stay in their original form — only the explanation is in the doc language.
- [ ] Each term is defined once (in the glossary); other docs link to it rather than redefine.
- [ ] Superseded ADRs/PRDs/BDRs carry frontmatter `status: Superseded` + `superseded_by: NNNN`; the superseding record sets `supersedes` and links back.
- [ ] Any structural code change in the same task updated its doc **and its Mermaid diagram(s)**.
- [ ] Architecture diagrams use Mermaid (in-repo text), match the code, and use context-index vocabulary for node/participant names.
- [ ] Every BDR has a Mermaid diagram, a textual description, **a Contract section** (public signatures + agent tool schemas, observable-only), numbered Given/When/Then scenarios, **and a Test Design matrix** (each row names what it proves); an execution issue links the matrix rather than copying it.
- [ ] Every NFR is a quality-attribute scenario in the PRD bound to a verifying instrument (not a freeform "should be fast" line); a structural architecture view names whether it is checked or inspection-only.
- [ ] The constitution is singular (`docs/constitution.md`) — no NNNN prefix, no index entry.
- [ ] Each index file's links all resolve (no dangling references).
- [ ] Any doc declaring `visibility` uses `private | public | showcase` (check enforces the domain); absent ⇒ private, and a doc meant for a public bundle carries `visibility: public | showcase`.
- [ ] Docs-first respected: the repo body matches the published tracker/wiki copy.
