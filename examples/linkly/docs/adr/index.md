# ADRs

Architecture decisions — how Linkly is structured, and why. The decision *log* is this
listing plus each record's `status` / `superseded_by` frontmatter.

> **Active view (the corpus-at-scale convention).** Split the listing by `status` so a
> reader sees what is *in force* without scrolling through history. Superseded records are
> kept — never deleted — but parked below. See `rules/adr-conventions.md`.

## Active

* [0002 — SQLite store](0002-sqlite-store.md) - durable storage for minted links

## Superseded

* [0001 — In-memory store](0001-in-memory-store.md) - superseded by 0002 (lost links on restart)
