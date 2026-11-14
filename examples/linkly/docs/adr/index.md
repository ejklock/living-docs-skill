# ADRs

Architecture decisions — how Linkly is structured, and why. The decision *log* is this
listing plus each record's `status` / `superseded_by` frontmatter.

> **Active view (the corpus-at-scale convention).** Split the listing by `status` so a
> reader sees what is *in force* without scrolling through history. Superseded records are
> kept — never deleted — but parked below. See `rules/adr-conventions.md`.

## Active

* [0002 — SQLite store for minted links](0002-sqlite-store.md) - Accepted

## Superseded

_History only. Do not act on these records — run `living-docs read records` for what is in force._

* [0001 — In-memory store for minted links](0001-in-memory-store.md) - Superseded by [0002](0002-sqlite-store.md)
