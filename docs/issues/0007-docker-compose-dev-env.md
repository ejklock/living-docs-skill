---
type: Issue
title: Dev environment — docker-compose (ParadeDB) + Makefile targets over the workspace
description: Extend the existing Docker-always dev setup with a docker-compose stack that brings up ParadeDB and the web, plus Makefile targets, so db-mode and multi-engine slices have a reproducible local/CI environment.
status: done
labels: [enabling, tooling, infra, docker]
blocked_by: [1]
timestamp: 2026-07-16T00:00:00Z
---

## Dev environment — docker-compose (ParadeDB) + Makefile targets

Enables [ADR 0004](/adr/0004-db-engine-and-data-layer.md) (ParadeDB) and
[ADR 0006](/adr/0006-web-read-only-axum.md) (web). **Enabling/tooling issue** — not a
user-facing vertical slice; its observable outcome is a healthy, reachable stack (smoke).

### Objective link

Constitution non-negotiable (Postgres is a *db-mode deployment* dependency; the binary stays
self-contained) → ADR 0004 needs a reproducible ParadeDB to run against → this dev env.

### Context manifest

- Read: `Dockerfile.dev` (pinned Rust toolchain), `Makefile` (existing `cli-*` Docker-always
  targets + `DOCKER_CARGO`), `cli/rust-toolchain.toml`, ADR 0004.
- Seams touched: new `docker-compose.yml` (workspace root); new `.env.example`
  (`DATABASE_URL` and Postgres creds); new Makefile targets that compose the stack.
- Pattern: extend the existing Docker-always approach — the host is not assumed to have a
  toolchain or Postgres. Compose owns the ParadeDB service; the app crates
  (`db-store`/`web`) run against it.

### Scope

Add a `docker-compose.yml` with:
- a **`paradedb`** service (ParadeDB image), `pg_isready` healthcheck, a named volume, and
  `DATABASE_URL` wired via `.env`;
- a **`web`** service (built from the workspace) reachable on a host port, depending on
  `paradedb` being healthy.

Add Makefile targets: `up` / `down` (compose lifecycle), `db-up` / `db-psql` / `db-logs`,
and `db-test` (run the dual-engine suite against the composed Postgres). Keep the existing
`cli-*` / `build` / `check` targets untouched. Add `.env.example`; keep secrets out of git.

### Vertical Demo

- **Given** the repo, **When** I run `make up`, **Then** `docker compose ps` shows the
  `paradedb` service **healthy** and `make db-psql` runs `select 1` successfully.
- **Given** the stack up, **When** I run `make down`, **Then** the services stop and the
  named volume persists (data survives a restart).

### Acceptance

- `docker compose config` validates the compose file (no schema errors). — `verify_by: command`
- **Smoke:** `make up` brings `paradedb` to a healthy healthcheck within a bounded timeout;
  a scripted `pg_isready` / `select 1` against `DATABASE_URL` returns success. —
  `verify_by: smoke`
- `.env` is git-ignored; only `.env.example` is committed (no secrets in the tree). —
  `verify_by: inspection`
- Existing `make check` / `cli-*` targets still pass unchanged. — `verify_by: command`

### Out of scope

No production/deploy manifests, no orchestration (k8s), no CI provider config beyond the
compose the CI can call. The web *content* is issue 0003; this issue only wires its service.

### Plan

`docker-compose.yml` (paradedb + web) → `.env.example` + gitignore → Makefile `up`/`down`/
`db-*` targets → healthcheck smoke. One pipeline slice.

### Delivery note

Delivered focused on the ParadeDB dev environment. Two scope deviations, decided with the user:

- **The compose `web` service was deferred to issues 0004/0006.** `web` cannot talk to ParadeDB
  until the `db-store` Postgres engine lands (0004), so wiring it now would only crash-loop or
  serve the SQLite read-model. The smoke here is `paradedb` healthy + `select 1`.
- **The template is committed as `env.example` (dotless), not `.env.example`.** The global
  `Write(**/.env.*)` permission rule denies writing any `.env.*` path (including the template);
  the dotless name sidesteps it while respecting the secrets-in-git guard.

Implementation notes: image pinned to `paradedb/paradedb:0.24.3`; the named volume mounts at
`/var/lib/postgresql` (the bundled Postgres 18 rejects the old `/data` subdirectory layout);
`make db-up` uses `up -d --wait` so the healthcheck gates the smoke. Verified live (paradedb
`Healthy`, `pg_isready` accepting, `select 1` → `1`).
