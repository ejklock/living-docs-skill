# Issues

Vertical, demoable slices for the living-docs system (greenfield backlog). Each issue
delivers one user-observable behavior end-to-end and links up the trail
([Constitution](../constitution.md) → [ADRs](../adr/)). Consume them smallest/lowest first,
one slice per fresh context, starting from the skeleton.

## Done

* [0001 — Walking skeleton: Cargo workspace + living-docs-core + fs-store + thin cli](0001-workspace-core-skeleton.md) - done · delivered as S0a + S0b1 + S0b2
* [0002 — Findability: db sync builds a SQLite/FTS5 read-model and living-docs search queries it](0002-findability-search.md) - done · delivered as S2a + S2b + S2c
* [0003 — Read-only web view: axum search + record page over the read-model](0003-web-read-only.md) - done · delivered as S3a + S3b + S3c

## Open

* [0004 — ParadeDB (Postgres + BM25) as a selectable db engine alongside SQLite](0004-paradedb-engine.md) - open · blocked_by: 0002, 0007
* [0005 — projects root + multi-project ingestion and cross-project search](0005-projects-multi-project.md) - open · blocked_by: 0002
* [0006 — db-mode authoritative authoring + lossless .md export](0006-db-mode-authoring.md) - open · blocked_by: 0002, 0005
* [0007 — Dev environment: docker-compose (ParadeDB) + Makefile targets](0007-docker-compose-dev-env.md) - open · blocked_by: 0001
