# Core invariants (the spine)

These five invariants hold across **every** document type — ADR, BDR, PRD, constitution, issue,
research, glossary, architecture. Everything else in this skill is one of these five applied to a
specific document type. When a rule seems unclear, re-derive the right action from these.

1. **Docs-first.** Author the body in the repo (`docs/…`) *before* publishing anywhere external
   (tracker, wiki). The repo file is the source of truth; the external copy is a mirror.
2. **One home per fact.** Each concept, decision, or requirement lives in exactly one file. No
   duplication — cross-reference instead of copying. Duplicated prose is drift waiting to happen.
3. **Indexed or it doesn't exist.** Every doc is reachable from an index (an `index.md` listing in
   its directory, and the bundle-root `docs/index.md` that the project guide links). No orphan
   files.
4. **Supersede, never rewrite history.** Decisions and requirements are append-only records. When
   something changes, mark the old record superseded and write a new one — never silently edit the
   past.
5. **No structural change without its doc.** New module, moved files, schema change, new data flow
   → update the relevant doc *and its diagram* in the same change. No "I'll document it later."

When in doubt, re-derive the right action from these five. The other topics are just these
invariants applied to each document type.
