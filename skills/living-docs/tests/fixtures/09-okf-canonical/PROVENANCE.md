# Positive parity fixture — the OKF format author's own canonical bundle

`docs/` is a **verbatim, vendored copy** of an example bundle published by the authors
of the Open Knowledge Format, used here as a positive parity test: our checker must
accept the bundle the format's own authors ship as canonical.

| Field | Value |
|---|---|
| Source repo | `GoogleCloudPlatform/knowledge-catalog` |
| Source path | `okf/bundles/crypto_bitcoin/` (the 8 `.md` files; `viz.html` is omitted — no doc links to it) |
| Ref (commit) | `d44368c15e38e7c92481c5992e4f9b5b421a801d` (branch `main`) |
| Retrieved | 2026-06-21 |
| Upstream license | Apache-2.0 (see `ATTRIBUTION.md`) |

This is the smallest of the three upstream bundles (crypto_bitcoin / ga4 / stackoverflow);
all three pass `living-docs check` cleanly. Only one is vendored to keep the repo small.

It also documents the relationship between OKF and Living Docs: `living-docs check` enforces
**OKF §9 conformance** (parseable frontmatter, non-empty `type`, reserved-file rules) **plus**
the stricter Living Docs governance invariants (links resolve, index membership +
reachability, supersede integrity). OKF's own §9 tells *consumers* to be permissive
(they MUST NOT reject for broken links or a missing `index.md`); Living Docs is an
*authoring* discipline that deliberately adds those stricter checks. This canonical
bundle happens to satisfy both — which is exactly what makes it a good parity probe.
