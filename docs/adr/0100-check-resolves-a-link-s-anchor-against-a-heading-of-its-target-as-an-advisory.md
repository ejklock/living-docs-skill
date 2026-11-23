---
type: ADR
title: check resolves a link's anchor against a heading of its target, as an advisory
description: A local markdown link whose fragment matches no heading slug of its target is reported as an anchor-missing advisory.
owner: Evaldo Klock
status: Proposed
timestamp: 2026-11-23T02:23:42Z
---

# 0100. check resolves a link's anchor against a heading of its target, as an advisory

## Context

A business rule in a domain record is a heading (ADR 0099), and an issue or another record links it as `context/<path>.md#<heading-slug>`. `check` resolves a local link's file but strips its anchor before testing existence (`living-docs-core/src/check/links.rs:7`), so a renamed or removed rule heading leaves every link to it pointing nowhere with `check` green. The same holds today for links into any record's section.

## Decision

We will have `check` resolve the fragment of a local markdown link whose target is a markdown file inside the bundle: the fragment must equal the slug of a heading in the target, computed as GitHub computes heading anchors (lowercase, punctuation dropped, spaces as hyphens, a `-1`, `-2` suffix for a repeated heading). A link whose fragment matches no heading is reported under a new advisory kind, `anchor-missing`, naming the link and the target. Following ADR 0095, it starts as an advisory and becomes a violation only by a later ADR that counts it over real bundles.

## Consequences

- Renaming or deleting a rule heading shows every link it breaks, in domain records and in any other record.
- External links, links to non-markdown files and links with no fragment are not judged; an inline HTML anchor (`<a id>`) is not read, so a link to one is reported until its author links a heading instead.
- The slug must match the renderer the owner reads on; a mismatch would report true links as missing, so the implementation is tested against GitHub's algorithm, including repeated headings and non-ASCII text.
- `manage html` emits ids per record only, so a rule anchor does not resolve inside the local HTML file; that is a separate change.
- No rule identifier syntax is added: the anchor is the heading, so the BR-ID grammar issue 0144 rejected stays rejected.
