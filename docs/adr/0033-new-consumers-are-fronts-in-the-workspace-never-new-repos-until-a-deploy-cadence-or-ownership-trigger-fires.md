---
type: ADR
title: New consumers are fronts in the workspace, never new repos, until a deploy-cadence or ownership trigger fires
description: Every new consumer (API, MCP, frontend) is a front inside the Cargo workspace over living-docs-core; crates.io publication serves external consumers; a repo split waits for a deploy-cadence or ownership trigger.
owner: Evaldo Klock
status: Accepted
timestamp: 2026-08-05T20:33:40Z
---

# 0033. New consumers are fronts in the workspace, never new repos, until a deploy-cadence or ownership trigger fires

## Context

The knowledge-graph direction (ADRs 0031/0032) raised the question of splitting `cli`, `core`, `db-store`, and a future graph module into separate repositories, motivated by future consumers: an HTTP API, a frontend, MCP servers. The re-deliberation confirmed what ADR 0002 locked, with sharper reasons. New consumers need stable interfaces, not repo boundaries: an API server and an MCP server are fronts over `living-docs-core`, exactly the shape `web` already has; a frontend consumes the HTTP API, not the Rust code; external Rust consumers consume published crates. The direct precedent is Graphiti itself — core, server, and MCP server ship from one repository. The local scar argues the same way: taming nine version-declaration sites took three release rounds inside ONE repo (lessons 3987/3988/4091); multiple repos multiply that class of bug plus semver coordination and split CI. A separate "graph core" would also reopen what ADR 0031 settled: the graph is a projection of the same records, not a system.

## Decision

We will add every new consumer as a front inside the existing Cargo workspace (`mcp` is the first candidate, alongside `cli` and `web`). When external consumption demands stable versioned artifacts, we will publish `living-docs-core` (and adapters as needed) to crates.io from the monorepo rather than extract repositories. A repository split is reconsidered only when a named trigger fires: a front needs an independent deploy cadence, or a component gains separate ownership. A frontend in a non-Rust toolchain may live outside the workspace; its boundary is the HTTP API.

## Consequences

**Easier / gained:**
- Domain changes stay atomic: one PR crosses core, adapters, and every front; no cross-repo semver dance.
- New fronts inherit the release train, CI, and the version gate (`check-version.sh`) instead of re-growing them.
- The hexagonal ports remain the extraction seam, so a future split stays cheap precisely because it is not exercised early.

**Harder / accepted trade-offs:**
- One release train: a front cannot ship on its own cadence without firing the trigger and revisiting this ADR.
- The workspace grows; build times and the version-site count grow with each front (each new front's version site must enter `check-version.sh` on day one).

**Follow-ups:**
- Issue 0027 specifies the MCP front.
- Publishing `living-docs-core` to crates.io gets its own decision when the first external consumer is real, not before.

## Verification

**Implementation impact:** workspace `Cargo.toml` members; `scripts/check-version.sh` when a front is added.

**Verification criteria:**
- Every front in the workspace depends on `living-docs-core` and never on another front (dependency-direction check).
- A new front's version declaration site is covered by `check-version.sh` in the same PR that adds the front.

# References

[1] [ADR 0002 — hexagonal core, Cargo workspace](0002-hexagonal-core-workspace.md)
[2] [Research 0003 — provenance-aware knowledge graph for living-docs](../research/0003-provenance-aware-knowledge-graph-for-living-docs.md)
[3] [getzep/graphiti — core, server, and MCP server in one repository](https://github.com/getzep/graphiti)
