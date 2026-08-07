---
type: Issue
title: db commands can create a literal 'sqlite:' directory by treating the connection string as a filesystem path
description: <One sentence — the change and its motivation.>
status: closed
timestamp: 2026-08-06T21:03:43Z
---

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly open | in-progress | closed. `living-docs supersede` sets Superseded on
     this issue -- never set it by hand -- when a later issue replaces it. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## db commands can create a literal 'sqlite:' directory by treating the connection string as a filesystem path

An empty directory literally named `sqlite:` appeared at the repository root. The name matches a SQLite connection-string prefix (`sqlite:...`), so some code path most likely passed the connection string to a filesystem call that creates parent directories. The suspects are the `db sync` / `search` engine wiring and the `--engine sqlite` fallback path (`.living-docs/index.db` when `$DATABASE_URL` is unset) from [ADR 0004](/adr/0004-db-engine-and-data-layer.md). The directory was empty, so no data was written through the wrong path; the defect is silent litter plus the risk that a future write lands in it.

### Scope

- Find the code path that turns the connection string into a directory-creation call and make it parse the string into a path first (or reject it loudly).
- KEPT: the engine selection contract of ADR 0004 (ParadeDB via `$DATABASE_URL` default, SQLite opt-in) and the `.living-docs/index.db` fallback location.

### Acceptance

- Running the `db`/`search` commands with each engine configuration (`--engine sqlite` with and without `$DATABASE_URL`, and the ParadeDB default) never creates a `sqlite:`-named filesystem entry.
- A regression test covers the connection-string-to-path translation for the sqlite engine.

### Plan

1. Reproduce: trace which command created the directory (audit the connection-string handling in the db-store engine setup).
2. Fix the translation (strip/parse the scheme prefix before any `create_dir_all`/open call) and add the regression test.
