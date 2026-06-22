# Examples

Worked Living Docs corpora — the discipline shown end-to-end, not just described.

## `linkly/` — a fictional URL shortener

A complete doc trail for a tiny product, so you can see every link of the chain in one
place and copy the shapes into your own project:

```
constitution → PRD 0001 → ADR 0002 (+ superseded ADR 0001) + BDR 0001 → issue 0001 → code
```

It also demonstrates three things the prose only asserts:

- **Supersede, never rewrite (invariant 4).** [ADR 0001](linkly/docs/adr/0001-in-memory-store.md)
  is superseded by [ADR 0002](linkly/docs/adr/0002-sqlite-store.md) — kept, not deleted —
  and the [ADR index](linkly/docs/adr/index.md) splits **Active** from **Superseded** (the
  corpus-at-scale "active view" convention).
- **Doc → implement → verify.** [ADR 0002](linkly/docs/adr/0002-sqlite-store.md) carries a
  *Verification* block, and [issue 0001](linkly/docs/issues/0001-implement-shorten-endpoint.md)
  binds each acceptance line to a [BDR 0001](linkly/docs/bdr/0001-shorten-and-redirect.md)
  scenario, so "done" is machine-checkable.
- **Indexed or it doesn't exist.** Every file is reachable from
  [`docs/index.md`](linkly/docs/index.md).

### Lint it

The corpus is also the fixture for the skill's checker. From the repo root:

```bash
./skills/living-docs/scripts/lint-docs.sh examples/linkly/docs
# or simply:
make lint-docs
```

It should report `OK` with no violations. CI runs exactly this on every push.
